#!/usr/bin/env python3
"""Finite contract and mutation tests for the advanced CodeQL workflow."""

from __future__ import annotations

from pathlib import Path
import re
import shutil
import tempfile
from typing import Callable

import workflow_quality_gate


ROOT = Path(__file__).resolve().parents[1]
CODEQL_WORKFLOW = ROOT / ".github" / "workflows" / "codeql.yml"
WINDOWS_WORKFLOW = ROOT / ".github" / "workflows" / "windows-client.yml"
RUST_WORKFLOW = ROOT / ".github" / "workflows" / "rust.yml"

BINARY_IMPACT_JOB_IF = workflow_quality_gate.BINARY_IMPACT_JOB_IF
EXPECTED_LANGUAGES = ("actions", "csharp", "python", "rust")


def _replace_exact(source: str, needle: str, replacement: str) -> str:
    count = source.count(needle)
    if count != 1:
        raise AssertionError(
            f"mutation target count={count}, expected 1: {needle!r}"
        )
    return source.replace(needle, replacement, 1)


def _nonblank(lines: tuple[str, ...] | list[str]) -> tuple[str, ...]:
    return tuple(line for line in lines if line.strip())


def validate(
    codeql_path: Path = CODEQL_WORKFLOW,
    windows_path: Path = WINDOWS_WORKFLOW,
) -> list[str]:
    errors: list[str] = []
    try:
        codeql = workflow_quality_gate.parse_workflow(codeql_path)
        sections = workflow_quality_gate._top_level_ranges(codeql.lines)
        triggers = workflow_quality_gate._section_children(codeql, "on")
        if set(triggers) != {"workflow_call", "schedule", "workflow_dispatch"}:
            errors.append(f"CodeQL triggers changed: {sorted(triggers)}")
        for empty_trigger in ("workflow_call", "workflow_dispatch"):
            child = triggers.get(empty_trigger)
            if child is None or child[2] or _nonblank(list(codeql.lines[child[0] + 1 : child[1]])):
                errors.append(f"CodeQL {empty_trigger} trigger is not an empty mapping")

        schedule_lines = workflow_quality_gate._event_child_lines(codeql, "schedule")
        if _nonblank(list(schedule_lines)) != ('    - cron: "23 4 * * 1"',):
            errors.append("CodeQL weekly schedule changed")

        permissions = sections.get("permissions")
        if permissions is None or permissions[2]:
            errors.append("CodeQL permissions mapping is missing")
        else:
            permission_lines = _nonblank(
                list(codeql.lines[permissions[0] + 1 : permissions[1]])
            )
            if permission_lines != (
                "  contents: read",
                "  security-events: write",
            ):
                errors.append("CodeQL permissions are not least-privilege exact")

        if set(codeql.jobs) != {"analyze"}:
            errors.append(f"CodeQL jobs changed: {sorted(codeql.jobs)}")
        else:
            analyze = codeql.jobs["analyze"]
            if analyze.properties.get("name") != "Analyze (${{ matrix.language }})":
                errors.append("CodeQL analyze job name changed")
            if analyze.properties.get("runs-on") != "ubuntu-latest":
                errors.append("CodeQL analyze runner changed")
            if analyze.needs or "uses" in analyze.properties:
                errors.append("CodeQL analyze job shape is not local and standalone")

        matrix = (
            "      matrix:\n"
            "        include:\n"
            + "".join(
                f"          - language: {language}\n"
                "            build-mode: none\n"
                for language in EXPECTED_LANGUAGES
            )
        )
        if codeql.text.count(matrix) != 1:
            errors.append("CodeQL language matrix or build-mode changed")
        if codeql.text.count("      fail-fast: false\n") != 1:
            errors.append("CodeQL matrix is not fail-fast false")

        steps = workflow_quality_gate._job_steps(codeql, "analyze")
        expected_steps = (
            "uses:actions/checkout@v4",
            "uses:github/codeql-action/init@v4",
            "uses:github/codeql-action/analyze@v4",
        )
        if tuple(step.label for step in steps) != expected_steps:
            errors.append("CodeQL step set or order changed")
        else:
            init_text = "\n".join(steps[1].lines)
            analyze_text = "\n".join(steps[2].lines)
            for marker in (
                "          languages: ${{ matrix.language }}",
                "          build-mode: ${{ matrix.build-mode }}",
            ):
                if init_text.count(marker) != 1:
                    errors.append(f"CodeQL init input changed: {marker.strip()}")
            if analyze_text.count(
                '          category: "/language:${{ matrix.language }}"'
            ) != 1:
                errors.append("CodeQL analysis category changed")

        if re.search(r"github/codeql-action/autobuild@", codeql.text, re.IGNORECASE):
            errors.append("CodeQL Autobuild step is present")
        if re.search(r"^\s+run:", codeql.text, re.MULTILINE):
            errors.append("CodeQL workflow contains a build or shell run step")

        workflow_directory = codeql_path.parent
        for candidate in sorted(
            (*workflow_directory.glob("*.yml"), *workflow_directory.glob("*.yaml"))
        ):
            if candidate.resolve() == codeql_path.resolve():
                continue
            try:
                candidate_text = candidate.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                errors.append(f"cannot audit CodeQL source duplication: {candidate.name}")
                continue
            if "github/codeql-action/" in candidate_text:
                errors.append(f"CodeQL action is duplicated in {candidate.name}")

        windows = workflow_quality_gate.parse_workflow(windows_path)
        codeql_call = windows.jobs.get("codeql-analysis")
        if codeql_call is None:
            errors.append("Windows workflow is missing codeql-analysis")
        else:
            if codeql_call.properties.get("if") != BINARY_IMPACT_JOB_IF:
                errors.append("Windows CodeQL call is not exact binary-impact-only")
            if codeql_call.needs != ("version-prepared",):
                errors.append("Windows CodeQL call does not depend only on trusted scope")
            if codeql_call.properties.get("uses") != "./.github/workflows/codeql.yml":
                errors.append("Windows CodeQL call does not use the CodeQL source")
        acceptance = windows.jobs.get("acceptance")
        if acceptance is None:
            errors.append("Windows workflow is missing acceptance")
        else:
            if "codeql-analysis" not in acceptance.needs:
                errors.append("acceptance does not need codeql-analysis")
            outcome = acceptance.run_blocks[0].body if acceptance.run_blocks else ""
            owners = (
                'for result in "$NATIVE_RESULT" "$CODEQL_RESULT" '
                '"$WINDOWS_RESULT" "$UI_RESULT"; do'
            )
            if outcome.count(owners) != 2:
                errors.append("acceptance does not require CodeQL success and skip outcomes")
            acceptance_text = "\n".join(
                line
                for step in workflow_quality_gate._job_steps(windows, "acceptance")
                for line in step.lines
            )
            if acceptance_text.count(
                "CODEQL_RESULT: ${{ needs.codeql-analysis.result }}"
            ) != 1:
                errors.append("acceptance CodeQL result binding changed")

        integration_errors = workflow_quality_gate.validate_workflows(
            windows_path, RUST_WORKFLOW
        )
        if integration_errors:
            errors.extend(f"Windows integration: {error}" for error in integration_errors)
    except (OSError, UnicodeDecodeError, workflow_quality_gate.WorkflowError) as exc:
        errors.append(str(exc))
    return errors


Mutation = Callable[[Path, Path], None]


def _mutate_file(
    target: str, needle: str, replacement: str
) -> Mutation:
    def mutate(codeql: Path, windows: Path) -> None:
        path = codeql if target == "codeql" else windows
        source = path.read_text(encoding="utf-8")
        path.write_text(
            _replace_exact(source, needle, replacement),
            encoding="utf-8",
        )

    return mutate


def _mutations() -> tuple[tuple[str, Mutation], ...]:
    owner_loop = (
        '              for result in "$NATIVE_RESULT" "$CODEQL_RESULT" '
        '"$WINDOWS_RESULT" "$UI_RESULT"; do\n'
    )
    return (
        (
            "workflow-call-missing",
            _mutate_file("codeql", "  workflow_call:\n", ""),
        ),
        (
            "direct-pull-request-trigger",
            _mutate_file("codeql", "on:\n", "on:\n  pull_request:\n"),
        ),
        (
            "language-missing",
            _mutate_file(
                "codeql",
                "          - language: rust\n            build-mode: none\n",
                "",
            ),
        ),
        (
            "build-mode-not-none",
            _mutate_file(
                "codeql",
                "          - language: csharp\n            build-mode: none\n",
                "          - language: csharp\n            build-mode: autobuild\n",
            ),
        ),
        (
            "autobuild-step",
            _mutate_file(
                "codeql",
                "      - uses: github/codeql-action/analyze@v4\n",
                "      - uses: github/codeql-action/autobuild@v4\n"
                "      - uses: github/codeql-action/analyze@v4\n",
            ),
        ),
        (
            "shell-build-step",
            _mutate_file(
                "codeql",
                "      - uses: github/codeql-action/analyze@v4\n",
                "      - name: Build\n"
                "        run: cargo build --locked\n"
                "      - uses: github/codeql-action/analyze@v4\n",
            ),
        ),
        (
            "security-events-permission-missing",
            _mutate_file("codeql", "  security-events: write\n", ""),
        ),
        (
            "post-merge-push-trigger",
            _mutate_file(
                "codeql",
                "on:\n",
                'on:\n  push:\n    branches: ["main"]\n',
            ),
        ),
        (
            "caller-source-wrong",
            _mutate_file(
                "windows",
                "    uses: ./.github/workflows/codeql.yml\n",
                "    uses: ./.github/workflows/rust.yml\n",
            ),
        ),
        (
            "acceptance-edge-missing",
            _mutate_file(
                "windows",
                "needs: [version-prepared, native-quality, codeql-analysis, windows-quality, ui-quality]",
                "needs: [version-prepared, native-quality, windows-quality, ui-quality]",
            ),
        ),
        (
            "acceptance-result-binding-missing",
            _mutate_file(
                "windows",
                "          CODEQL_RESULT: ${{ needs.codeql-analysis.result }}\n",
                "",
            ),
        ),
        (
            "binary-impact-outcome-missing",
            _mutate_file(
                "windows",
                owner_loop + '                [[ "$result" == success ]] || {\n',
                '              for result in "$NATIVE_RESULT" "$WINDOWS_RESULT" "$UI_RESULT"; do\n'
                '                [[ "$result" == success ]] || {\n',
            ),
        ),
        (
            "no-binary-impact-outcome-missing",
            _mutate_file(
                "windows",
                owner_loop + '                [[ "$result" == skipped ]] || {\n',
                '              for result in "$NATIVE_RESULT" "$WINDOWS_RESULT" "$UI_RESULT"; do\n'
                '                [[ "$result" == skipped ]] || {\n',
            ),
        ),
    )


def main() -> int:
    baseline_errors = validate()
    if baseline_errors:
        raise AssertionError(
            "production CodeQL contract failed: " + "; ".join(baseline_errors)
        )
    cases = 1
    for name, mutate in _mutations():
        with tempfile.TemporaryDirectory(prefix="codeql-workflow-") as temporary:
            root = Path(temporary)
            codeql = root / "codeql.yml"
            windows = root / "windows-client.yml"
            shutil.copyfile(CODEQL_WORKFLOW, codeql)
            shutil.copyfile(WINDOWS_WORKFLOW, windows)
            mutate(codeql, windows)
            cases += 1
            if not validate(codeql, windows):
                raise AssertionError(f"CodeQL workflow mutation was accepted: {name}")

    try:
        _replace_exact(CODEQL_WORKFLOW.read_text(encoding="utf-8"), "not present", "")
    except AssertionError:
        cases += 1
    else:
        raise AssertionError("CodeQL mutation target guard accepted a no-op")
    if cases <= 1:
        raise AssertionError("CodeQL workflow test count is not positive")
    print(f"codeql-workflow: PASS cases={cases}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

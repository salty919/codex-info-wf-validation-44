#!/usr/bin/env python3
"""Fail-closed quality oracle for the Windows client workflow.

This is intentionally a small, line-oriented parser for the two workflow
shapes used by this repository.  It is not a general YAML parser: unsupported
or ambiguous structure is rejected.  The merge table below is synthetic test
data only; it is not live GitHub evidence and must not be treated as a
substitute for an API audit snapshot.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass, replace
from pathlib import Path
import re
import shutil
import tempfile
from typing import Callable, Iterable, Mapping, Sequence


ROOT = Path(__file__).resolve().parents[1]
WINDOWS_WORKFLOW = ROOT / ".github" / "workflows" / "windows-client.yml"
RUST_WORKFLOW = ROOT / ".github" / "workflows" / "rust.yml"

WINDOWS_JOBS = (
    "version-prepared",
    "native-quality",
    "windows-quality",
    "ui-quality",
    "acceptance",
    "release",
)
EXPECTED_NEEDS = {
    "version-prepared": [],
    "native-quality": ["version-prepared"],
    "windows-quality": ["version-prepared"],
    "ui-quality": ["version-prepared"],
    "acceptance": [
        "version-prepared",
        "native-quality",
        "windows-quality",
        "ui-quality",
    ],
    "release": [],
}

OWNER_PATTERNS = {
    "regression_guard": re.compile(r"(?:^|[\s'\"./])scripts/regression_guard\.sh(?:\s|$)"),
    "record_daemon_e2e": re.compile(r"(?:^|[\s'\"./])scripts/record_daemon_e2e\.sh(?:\s|$)"),
    "data_protection_gate": re.compile(r"(?:^|[\s'\"./])scripts/data_protection_gate\.sh(?:\s|$)"),
    "windows_client_contract_gate": re.compile(
        r"(?:^|[\s'\"./])scripts/windows_client_contract_gate\.sh(?:\s|$)"
    ),
    "Build-WindowsInstaller": re.compile(r"(?:^|[\s'\"./])Build-WindowsInstaller\.ps1(?:\s|$)"),
    "Run-WindowsClientE2E": re.compile(r"(?:^|[\s'\"./])Run-WindowsClientE2E\.ps1(?:\s|$)"),
    "windows_window_move_smoke": re.compile(
        r"(?:^|[\s'\"./])scripts/windows_window_move_smoke\.ps1(?:\s|$)"
    ),
    "final_acceptance_gate": re.compile(r"(?:^|[\s'\"./])scripts/final_acceptance_gate\.sh(?:\s|$)"),
}
OWNER_JOBS = {
    "regression_guard": "native-quality",
    "record_daemon_e2e": "native-quality",
    "data_protection_gate": "native-quality",
    "windows_client_contract_gate": "windows-quality",
    "Build-WindowsInstaller": "ui-quality",
    "Run-WindowsClientE2E": "ui-quality",
    "windows_window_move_smoke": "ui-quality",
    "final_acceptance_gate": "acceptance",
}
PRODUCER_PATTERNS = tuple(
    pattern
    for name, pattern in OWNER_PATTERNS.items()
    if name != "final_acceptance_gate"
)
QUALITY_TOOL = re.compile(
    r"(?:^|[;&|]\s*)(?:sudo\s+)?(?:cargo|dotnet|npm|pnpm|yarn|go|pytest|ctest|"
    r"gradle|mvn|msbuild(?:\.exe)?)(?:\s|$)",
    re.IGNORECASE | re.MULTILINE,
)
QUALITY_WRAPPER = re.compile(
    r"(?<![A-Za-z0-9_.-])(?:\./|[^\s;&|]+/)?[A-Za-z0-9_.-]+"
    r"(?:_gate|_test|_e2e)(?:\.(?:sh|ps1|py|cmd|bat))?(?=\s|$)",
    re.IGNORECASE,
)
QUALITY_WRAPPER_ALLOWLIST = {
    "acceptance": frozenset({"scripts/final_acceptance_gate.sh"}),
    "release": frozenset(
        {
            "scripts/release_candidate_gate.sh",
            "scripts/release_state_gate.py",
        }
    ),
}


class WorkflowError(ValueError):
    """Raised when the deliberately limited workflow grammar is ambiguous."""


@dataclass(frozen=True)
class RunBlock:
    job: str
    body: str
    line: int


@dataclass(frozen=True)
class Job:
    name: str
    properties: Mapping[str, str]
    needs: tuple[str, ...]
    run_blocks: tuple[RunBlock, ...]


@dataclass(frozen=True)
class Workflow:
    path: Path
    text: str
    lines: tuple[str, ...]
    jobs: Mapping[str, Job]


def _indent(line: str) -> int:
    if "\t" in line[: len(line) - len(line.lstrip())]:
        raise WorkflowError("tabs are not supported")
    return len(line) - len(line.lstrip(" "))


def _key_value(line: str, expected_indent: int | None = None) -> tuple[int, str, str] | None:
    indentation = _indent(line)
    if expected_indent is not None and indentation != expected_indent:
        return None
    match = re.match(r"^( *)([A-Za-z0-9_.-]+):(?:[ \t]*(.*))?$", line)
    if not match:
        return None
    return indentation, match.group(2), (match.group(3) or "").strip()


def _top_level_ranges(lines: Sequence[str]) -> dict[str, tuple[int, int, str]]:
    starts: list[tuple[int, str, str]] = []
    for index, line in enumerate(lines):
        parsed = _key_value(line, 0)
        if parsed is not None:
            _, key, value = parsed
            starts.append((index, key, value))
    result: dict[str, tuple[int, int, str]] = {}
    for position, (start, key, value) in enumerate(starts):
        if key in result:
            raise WorkflowError(f"duplicate top-level key: {key}")
        end = starts[position + 1][0] if position + 1 < len(starts) else len(lines)
        result[key] = (start, end, value)
    return result


def _parse_list(value: str, lines: Sequence[str], start: int, end: int, indent: int) -> list[str]:
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return []
        result = [item.strip().strip("'\"") for item in inner.split(",")]
        if any(not item or re.search(r"[\[\]]", item) for item in result):
            raise WorkflowError("unsupported inline list")
        return result
    if value:
        raise WorkflowError("only inline lists are supported")
    result: list[str] = []
    for line in lines[start + 1 : end]:
        if not line.strip():
            continue
        current_indent = _indent(line)
        if current_indent <= indent:
            break
        match = re.match(r"^\s*-\s+([^#]+?)\s*$", line)
        if not match:
            raise WorkflowError("unsupported block list")
        result.append(match.group(1).strip().strip("'\""))
    if not result:
        raise WorkflowError("empty non-inline list")
    return result


def _run_blocks(job_name: str, lines: Sequence[str], start: int, end: int) -> tuple[RunBlock, ...]:
    """Extract only ``run`` keys in actual six-space step mappings.

    A generic ``run:`` search would mistake nested ``env.run`` or an
    arbitrary field embedded in a script literal for an executable step.
    """
    blocks: list[RunBlock] = []
    steps = [index for index in range(start, end) if re.match(r"^ {4}steps:\s*$", lines[index])]
    if len(steps) > 1:
        raise WorkflowError(f"duplicate steps mapping in job {job_name}")
    if not steps:
        return ()
    step_starts = [
        index
        for index in range(steps[0] + 1, end)
        if re.match(r"^ {6}-\s", lines[index])
    ]
    for position, step_start in enumerate(step_starts):
        step_end = step_starts[position + 1] if position + 1 < len(step_starts) else end
        run_lines = [
            index
            for index in range(step_start + 1, step_end)
            if _key_value(lines[index], 8) is not None and _key_value(lines[index], 8)[1] == "run"
        ]
        if len(run_lines) > 1:
            raise WorkflowError(f"duplicate run mapping in job {job_name}")
        if not run_lines:
            continue
        index = run_lines[0]
        _, _, value = _key_value(lines[index], 8)
        if value in ("|", ">", "|-", ">-"):
            body: list[str] = []
            cursor = index + 1
            while cursor < step_end:
                candidate = lines[cursor]
                if candidate.strip() and _indent(candidate) <= 8:
                    break
                if candidate.strip():
                    body.append(candidate[10:] if len(candidate) > 10 else "")
                else:
                    body.append("")
                cursor += 1
            if not body:
                raise WorkflowError(f"run block without body at line {index + 1}")
            blocks.append(RunBlock(job_name, "\n".join(body), index + 1))
        elif value:
            blocks.append(RunBlock(job_name, value, index + 1))
        else:
            raise WorkflowError(f"unsupported run scalar at line {index + 1}")
    return tuple(blocks)


def parse_workflow(path: Path) -> Workflow:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise WorkflowError(f"cannot read workflow {path}: {exc}") from exc
    lines = tuple(text.splitlines())
    sections = _top_level_ranges(lines)
    jobs_section = sections.get("jobs")
    if jobs_section is None or jobs_section[2]:
        raise WorkflowError("jobs must be a block mapping")
    jobs_start, jobs_end, _ = jobs_section
    starts: list[tuple[int, str]] = []
    for index in range(jobs_start + 1, jobs_end):
        parsed = _key_value(lines[index], 2)
        if parsed is not None:
            starts.append((index, parsed[1]))
    jobs: dict[str, Job] = {}
    for position, (start, name) in enumerate(starts):
        if name in jobs:
            raise WorkflowError(f"duplicate job: {name}")
        end = starts[position + 1][0] if position + 1 < len(starts) else jobs_end
        properties: dict[str, str] = {}
        property_locations: dict[str, int] = {}
        for index in range(start + 1, end):
            parsed = _key_value(lines[index], 4)
            if parsed is None:
                continue
            _, key, value = parsed
            if key in properties:
                raise WorkflowError(f"duplicate property {key} in job {name}")
            properties[key] = value
            property_locations[key] = index
        needs = tuple(
            _parse_list(
                properties["needs"],
                lines,
                property_locations["needs"],
                end,
                4,
            )
            if "needs" in properties
            else []
        )
        jobs[name] = Job(name, properties, needs, _run_blocks(name, lines, start + 1, end))
    if not jobs:
        raise WorkflowError("jobs mapping is empty")
    return Workflow(path, text, lines, jobs)


def _section_children(workflow: Workflow, key: str) -> dict[str, tuple[int, int, str]]:
    sections = _top_level_ranges(workflow.lines)
    section = sections.get(key)
    if section is None or section[2]:
        raise WorkflowError(f"{key} must be a block mapping")
    start, end, _ = section
    starts: list[tuple[int, str, str]] = []
    for index in range(start + 1, end):
        parsed = _key_value(workflow.lines[index], 2)
        if parsed is not None:
            starts.append((index, parsed[1], parsed[2]))
    result: dict[str, tuple[int, int, str]] = {}
    for position, (child_start, child_key, child_value) in enumerate(starts):
        if child_key in result:
            raise WorkflowError(f"duplicate {key} child: {child_key}")
        child_end = starts[position + 1][0] if position + 1 < len(starts) else end
        result[child_key] = (child_start, child_end, child_value)
    return result


def _event_child_lines(workflow: Workflow, event: str) -> tuple[str, ...]:
    children = _section_children(workflow, "on")
    child = children.get(event)
    if child is None:
        raise WorkflowError(f"missing trigger: {event}")
    start, end, value = child
    if value:
        raise WorkflowError(f"trigger {event} must be a block")
    return workflow.lines[start + 1 : end]


def _mask_quoted(line: str) -> str:
    """Blank quoted text while retaining command punctuation and positions."""
    output: list[str] = []
    quote: str | None = None
    escaped = False
    for character in line:
        if escaped:
            output.append(" ")
            escaped = False
            continue
        if quote == '"' and character == "\\":
            output.append(" ")
            escaped = True
            continue
        if quote is None and character in ("'", '"'):
            quote = character
            output.append(" ")
        elif quote is not None and character == quote:
            quote = None
            output.append(" ")
        elif quote is None:
            output.append(character)
        else:
            output.append(" ")
    return "".join(output)


def _active_run_text(block: RunBlock) -> str:
    return "\n".join(
        _mask_quoted(line)
        for line in block.body.splitlines()
        if not line.lstrip().startswith("#")
    )


def _owner_occurrences(owner: str, text: str) -> int:
    """Count marker tokens only where the surrounding syntax invokes them."""
    pattern = OWNER_PATTERNS[owner]
    count = 0
    suffix = ".ps1" if owner in {"Build-WindowsInstaller", "Run-WindowsClientE2E", "windows_window_move_smoke"} else ".sh"
    for line in text.splitlines():
        for match in pattern.finditer(line):
            before = line[: match.start()]
            segment = before.rsplit(";", 1)[-1].rsplit("&&", 1)[-1].rsplit("||", 1)[-1].strip()
            if suffix == ".sh":
                shell_tokens = segment.split()
                if (
                    shell_tokens
                    and shell_tokens[-1] in {"bash", "sh", "source"}
                    and not any(token.rstrip(":") in {"echo", "printf", "require_text", "description"} for token in shell_tokens)
                ):
                    count += 1
                elif not segment and line[match.start() :].startswith(("./", "scripts/")):
                    count += 1
            elif (
                re.match(r"^(?:&\s*)?(?:\./|[A-Za-z0-9_.-]+/)+[A-Za-z0-9_.-]*$", segment)
                or re.search(r"(?:^|[({])\s*&\s*\.$", segment)
                or not segment
            ):
                count += 1
    return count


def _owner_counts(workflows: Iterable[Workflow]) -> dict[str, list[RunBlock]]:
    found: dict[str, list[RunBlock]] = {name: [] for name in OWNER_PATTERNS}
    for workflow in workflows:
        for job in workflow.jobs.values():
            for block in job.run_blocks:
                active = _active_run_text(block)
                for owner, pattern in OWNER_PATTERNS.items():
                    found[owner].extend([block] * _owner_occurrences(owner, active))
    return found


def _quality_commands(workflow: Workflow, job_names: Iterable[str]) -> list[str]:
    violations: list[str] = []
    for job_name in job_names:
        job = workflow.jobs[job_name]
        allowed_wrappers = QUALITY_WRAPPER_ALLOWLIST.get(job_name, frozenset())
        for block in job.run_blocks:
            active = _active_run_text(block)
            if QUALITY_TOOL.search(active):
                violations.append(f"{job_name}:{block.line}:quality tool")
            for line in active.splitlines():
                for wrapper in QUALITY_WRAPPER.finditer(line):
                    wrapper_path = wrapper.group(0)
                    if wrapper_path.startswith("./"):
                        wrapper_path = wrapper_path[2:]
                    if wrapper_path not in allowed_wrappers:
                        violations.append(f"{job_name}:{block.line}:quality wrapper")
            for owner in OWNER_PATTERNS:
                if owner != "final_acceptance_gate" and _owner_occurrences(owner, active):
                    violations.append(f"{job_name}:{block.line}:{owner}")
    return violations


def validate_workflows(windows_path: Path = WINDOWS_WORKFLOW, rust_path: Path = RUST_WORKFLOW) -> list[str]:
    errors: list[str] = []
    try:
        windows = parse_workflow(windows_path)
        rust = parse_workflow(rust_path)
        if set(windows.jobs) != set(WINDOWS_JOBS):
            errors.append(f"windows jobs mismatch: {sorted(windows.jobs)}")
        if set(rust.jobs) != {"native-quality"}:
            errors.append(f"rust reusable jobs mismatch: {sorted(rust.jobs)}")
        for name, expected in EXPECTED_NEEDS.items():
            if name in windows.jobs and list(windows.jobs[name].needs) != expected:
                errors.append(f"needs mismatch for {name}")
        for job_name, job in windows.jobs.items():
            unknown = [need for need in job.needs if need not in windows.jobs]
            if unknown:
                errors.append(f"unknown needs for {job_name}: {unknown}")
        if "native-quality" in windows.jobs:
            if windows.jobs["native-quality"].properties.get("uses") != "./.github/workflows/rust.yml":
                errors.append("native-quality is not the rust reusable job")
        for name in WINDOWS_JOBS:
            if name != "native-quality" and name in windows.jobs and "uses" in windows.jobs[name].properties:
                errors.append(f"unexpected reusable uses in {name}")

        # The top-level trigger shape is checked independently of job strings.
        pull_request_lines = _event_child_lines(windows, "pull_request")
        if any(re.match(r"^\s{4}paths(?:-ignore)?:", line) for line in pull_request_lines):
            errors.append("pull_request path filter is present")
        target_lines = _event_child_lines(windows, "pull_request_target")
        type_lines = [line for line in target_lines if re.match(r"^\s{4}types:", line)]
        if len(type_lines) != 1 or _key_value(type_lines[0], 4)[2] != "[closed]":
            errors.append("pull_request_target is not closed-only")
        concurrency = _top_level_ranges(windows.lines).get("concurrency")
        if concurrency is None:
            errors.append("missing concurrency")
        else:
            start, end, value = concurrency
            if value or sum(1 for line in windows.lines[start + 1 : end] if re.match(r"^\s{2}cancel-in-progress:", line)) != 1:
                errors.append("concurrency shape is ambiguous")
            elif not any(re.match(r"^\s{2}cancel-in-progress:\s*false\s*$", line) for line in windows.lines[start + 1 : end]):
                errors.append("cancel-in-progress is not false")

        acceptance_if = windows.jobs.get("acceptance", Job("", {}, (), ())).properties.get("if", "")
        if not (
            "always()" in acceptance_if
            and "github.event_name == 'pull_request'" in acceptance_if
            and "needs.version-prepared.outputs.ready == 'true'" in acceptance_if
        ):
            errors.append("acceptance does not guard always/PR/version-ready")
        release_if = windows.jobs.get("release", Job("", {}, (), ())).properties.get("if", "")
        if not (
            "github.event_name == 'pull_request_target'" in release_if
            and "github.event.pull_request.merged == true" in release_if
        ):
            errors.append("release is not closed-and-merged-only")

        counts = _owner_counts((windows, rust))
        for owner, blocks in counts.items():
            if len(blocks) != 1:
                errors.append(f"owner {owner} cardinality is {len(blocks)}")
            elif blocks[0].job != OWNER_JOBS[owner]:
                errors.append(f"owner {owner} is in job {blocks[0].job}")
        if "acceptance" in windows.jobs and "release" in windows.jobs:
            errors.extend(_quality_commands(windows, ("acceptance", "release")))

        # A reusable workflow must advertise workflow_call; this avoids
        # accepting a source copy whose job text merely happens to match.
        rust_sections = _top_level_ranges(rust.lines)
        on_section = rust_sections.get("on")
        if on_section is None or not any(re.match(r"^\s{2}workflow_call:", line) for line in rust.lines[on_section[0] + 1 : on_section[1]]):
            errors.append("rust workflow is not reusable")
    except WorkflowError as exc:
        errors.append(str(exc))
    return errors


@dataclass(frozen=True)
class MergeJob:
    context: str
    job_id: str = ""
    status: str = "completed"
    conclusion: str = "success"


@dataclass(frozen=True)
class MergeRun:
    run_id: str
    head_sha: str
    base_sha: str
    tree_sha: str
    jobs: tuple[MergeJob, ...]
    artifact_ids: tuple[str, ...] = ()
    provenance_markers: tuple[str, ...] = ()


@dataclass(frozen=True)
class MergeState:
    required_contexts: frozenset[str]
    strict: bool
    current_base: str
    current_head: str
    current_tree: str
    provenance_source: str
    provenance_tree: str
    runs: tuple[MergeRun, ...]
    provenance_id: str = ""


_SHA40 = re.compile(r"^[0-9a-fA-F]{40}$")


def _nonempty_identifier(value: object) -> bool:
    return value is not None and str(value).strip() not in {"", "0"}


def _full_sha(value: object) -> bool:
    return isinstance(value, str) and _SHA40.fullmatch(value) is not None


def evaluate_merge_state(state: MergeState) -> bool:
    """Return ALLOW only for one exact, current-head synthetic run set."""
    required = {"acceptance", "version-prepared", "CodeQL"}
    if state.required_contexts != required or state.strict is not True:
        return False
    if not all(
        _full_sha(value)
        for value in (
            state.current_base,
            state.current_head,
            state.current_tree,
            state.provenance_source,
            state.provenance_tree,
        )
    ):
        return False
    if not _nonempty_identifier(state.provenance_id):
        return False
    if state.provenance_source != state.current_head or state.provenance_tree != state.current_tree:
        return False
    # The supplied collection is the current PR mapping, not an unbounded
    # history query. Stale rows must be rejected rather than silently filtered.
    if len(state.runs) != 1:
        return False
    run = state.runs[0]
    if (
        not _nonempty_identifier(run.run_id)
        or not _full_sha(run.head_sha)
        or not _full_sha(run.base_sha)
        or not _full_sha(run.tree_sha)
        or run.head_sha != state.current_head
        or run.base_sha != state.current_base
        or run.tree_sha != state.current_tree
        or not run.artifact_ids
        or not all(_nonempty_identifier(identifier) for identifier in run.artifact_ids)
        or not run.provenance_markers
        or not all(_nonempty_identifier(marker) for marker in run.provenance_markers)
        or f"source-sha: {state.current_head}" not in run.provenance_markers
        or f"tree-sha: {state.current_tree}" not in run.provenance_markers
    ):
        return False
    contexts = [job.context for job in run.jobs]
    if (
        set(contexts) != required
        or len(contexts) != len(set(contexts))
        or not all(_nonempty_identifier(job.job_id) for job in run.jobs)
    ):
        return False
    return all(
        job.status == "completed" and job.conclusion == "success"
        for job in run.jobs
    )


def _valid_merge_state() -> MergeState:
    base = "1111111111111111111111111111111111111111"
    head = "2222222222222222222222222222222222222222"
    tree = "3333333333333333333333333333333333333333"
    return MergeState(
        required_contexts=frozenset({"acceptance", "version-prepared", "CodeQL"}),
        strict=True,
        current_base=base,
        current_head=head,
        current_tree=tree,
        provenance_source=head,
        provenance_tree=tree,
        runs=(
            MergeRun(
                run_id="run-101",
                head_sha=head,
                base_sha=base,
                tree_sha=tree,
                jobs=(
                    MergeJob("acceptance", job_id="job-acceptance"),
                    MergeJob("version-prepared", job_id="job-version"),
                    MergeJob("CodeQL", job_id="job-codeql"),
                ),
                artifact_ids=("artifact-acceptance", "artifact-version", "artifact-codeql"),
                provenance_markers=(f"source-sha: {head}", f"tree-sha: {tree}"),
            ),
        ),
        provenance_id="provenance-101",
    )


def _copy_workflows(directory: Path) -> tuple[Path, Path]:
    windows = directory / "windows-client.yml"
    rust = directory / "rust.yml"
    shutil.copyfile(WINDOWS_WORKFLOW, windows)
    shutil.copyfile(RUST_WORKFLOW, rust)
    return windows, rust


def _replace_exact(text: str, needle: str, replacement: str) -> str:
    count = text.count(needle)
    if count != 1:
        raise AssertionError(f"mutation target count={count}, expected 1: {needle!r}")
    return text.replace(needle, replacement, 1)


def _static_mutations() -> tuple[tuple[str, Callable[[Path, Path], None]], ...]:
    def missing_dag(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(
            _replace_exact(
                text,
                "needs: [version-prepared, native-quality, windows-quality, ui-quality]",
                "needs: [version-prepared, native-quality, windows-quality]",
            ),
            encoding="utf-8",
        )

    def duplicate_owner(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "run: bash scripts/windows_client_contract_gate.sh"
        replacement = "run: |\n          bash scripts/windows_client_contract_gate.sh\n          bash scripts/regression_guard.sh"
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def no_acceptance_always(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "if: always() && github.event_name == 'pull_request'", "if: github.event_name == 'pull_request'"), encoding="utf-8")

    def add_path_filter(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "  pull_request:\n    branches:", "  pull_request:\n    paths: ['**']\n    branches:"), encoding="utf-8")

    def forbidden_quality_command(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "run: bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e artifacts/native-quality artifacts/windows-quality"
        replacement = "run: |\n          bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e artifacts/native-quality artifacts/windows-quality\n          cargo test"
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def nested_env_run(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "run: bash scripts/windows_client_contract_gate.sh"
        replacement = "env:\n          run: bash scripts/windows_client_contract_gate.sh"
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def require_text_false_positive(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "run: bash scripts/windows_client_contract_gate.sh"
        replacement = "run: |\n          require_text: bash scripts/windows_client_contract_gate.sh"
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def quoted_false_positive(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "run: bash scripts/windows_client_contract_gate.sh"
        replacement = "run: |\n          echo \"bash scripts/windows_client_contract_gate.sh\""
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def forbidden_quality_tool(command: str) -> Callable[[Path, Path], None]:
        def mutate(windows: Path, _rust: Path) -> None:
            text = windows.read_text(encoding="utf-8")
            needle = "run: bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e artifacts/native-quality artifacts/windows-quality"
            replacement = (
                "run: |\n"
                "          bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e artifacts/native-quality artifacts/windows-quality\n"
                f"          {command}"
            )
            windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

        return mutate

    def forbidden_quality_wrapper(suffix: str) -> Callable[[Path, Path], None]:
        return forbidden_quality_tool(f"bash scripts/custom_quality_{suffix}.sh")

    def unknown_job(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "jobs:\n  version-prepared:", "jobs:\n  unexpected-job:\n    runs-on: ubuntu-latest\n  version-prepared:"), encoding="utf-8")

    def unknown_need(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "needs: [version-prepared]\n    runs-on: ubuntu-latest", "needs: [unknown-job]\n    runs-on: ubuntu-latest"), encoding="utf-8")

    return (
        ("dag-edge-missing", missing_dag),
        ("owner-duplicate", duplicate_owner),
        ("acceptance-always-missing", no_acceptance_always),
        ("pull-request-path-filter", add_path_filter),
        ("quality-command-in-acceptance", forbidden_quality_command),
        ("nested-env-run-not-command", nested_env_run),
        ("require-text-not-command", require_text_false_positive),
        ("quoted-not-command", quoted_false_positive),
        ("unknown-job", unknown_job),
        ("unknown-needs", unknown_need),
        ("quality-cargo", forbidden_quality_tool("cargo test")),
        ("quality-dotnet", forbidden_quality_tool("dotnet test")),
        ("quality-npm", forbidden_quality_tool("npm test")),
        ("quality-pnpm", forbidden_quality_tool("pnpm test")),
        ("quality-yarn", forbidden_quality_tool("yarn test")),
        ("quality-go", forbidden_quality_tool("go test ./...")),
        ("quality-pytest", forbidden_quality_tool("pytest")),
        ("quality-ctest", forbidden_quality_tool("ctest")),
        ("quality-gradle", forbidden_quality_tool("gradle test")),
        ("quality-mvn", forbidden_quality_tool("mvn test")),
        ("quality-wrapper-gate", forbidden_quality_wrapper("gate")),
        ("quality-wrapper-test", forbidden_quality_wrapper("test")),
        ("quality-wrapper-e2e", forbidden_quality_wrapper("e2e")),
    )


def _run_quality_wrapper_cases() -> int:
    def fixture(body: str) -> Workflow:
        return Workflow(
            Path("<quality-wrapper-fixture>"),
            "",
            (),
            {
                "release": Job(
                    "release",
                    {},
                    (),
                    (RunBlock("release", body, 1),),
                )
            },
        )

    allowed = fixture(
        "\n".join(
            (
                "bash scripts/release_candidate_gate.sh",
                "python3 ./scripts/release_state_gate.py",
            )
        )
    )
    if _quality_commands(allowed, ("release",)):
        raise AssertionError("release verifier fixture was rejected")

    rejected = fixture("bash scripts/arbitrary_release_gate.sh")
    if _quality_commands(rejected, ("release",)) != ["release:1:quality wrapper"]:
        raise AssertionError("arbitrary release _gate fixture was accepted")
    return 2


def _run_static_cases() -> int:
    cases = 1
    baseline_errors = validate_workflows()
    if baseline_errors:
        raise AssertionError("production workflow baseline failed: " + "; ".join(baseline_errors))
    cases += _run_quality_wrapper_cases()
    for name, mutate in _static_mutations():
        with tempfile.TemporaryDirectory(prefix="workflow-quality-") as temporary:
            windows, rust = _copy_workflows(Path(temporary))
            mutate(windows, rust)
            cases += 1
            if not validate_workflows(windows, rust):
                raise AssertionError(f"mutation was accepted: {name}")
    # The fixture harness itself must fail closed for an absent or ambiguous
    # mutation target; otherwise a no-op fixture could report false evidence.
    source = WINDOWS_WORKFLOW.read_text(encoding="utf-8")
    for needle in ("not present in production", "name:"):
        try:
            _replace_exact(source, needle, "")
        except AssertionError:
            cases += 1
        else:
            raise AssertionError(f"mutation target guard did not reject {needle!r}")
    return cases


def _run_merge_cases() -> int:
    valid = _valid_merge_state()
    old_base = "4444444444444444444444444444444444444444"
    old_head = "5555555555555555555555555555555555555555"
    old_tree = "6666666666666666666666666666666666666666"

    def run_update(state: MergeState, **changes: object) -> MergeState:
        return replace(state, runs=(replace(state.runs[0], **changes),))

    def jobs_update(state: MergeState, jobs: tuple[MergeJob, ...]) -> MergeState:
        return run_update(state, jobs=jobs)

    mutations: tuple[tuple[str, Callable[[MergeState], MergeState], bool], ...] = (
        ("valid", lambda state: state, True),
        ("job-failure", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], conclusion="failure"), *state.runs[0].jobs[1:])), False),
        ("job-cancelled", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], conclusion="cancelled"), *state.runs[0].jobs[1:])), False),
        ("job-in-progress", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], status="in_progress"), *state.runs[0].jobs[1:])), False),
        ("context-missing", lambda state: jobs_update(state, state.runs[0].jobs[:2]), False),
        ("context-extra", lambda state: jobs_update(state, (*state.runs[0].jobs, MergeJob("extra", job_id="job-extra"))), False),
        ("context-duplicate", lambda state: jobs_update(state, (state.runs[0].jobs[0], replace(state.runs[0].jobs[1], context="acceptance"), state.runs[0].jobs[2])), False),
        ("base-behind", lambda state: run_update(state, base_sha=old_base), False),
        ("base-stale", lambda state: replace(state, current_base=old_base), False),
        ("head-mismatch", lambda state: run_update(state, head_sha=old_head), False),
        ("tree-mismatch", lambda state: run_update(state, tree_sha=old_tree), False),
        ("provenance-source-mismatch", lambda state: replace(state, provenance_source=old_head), False),
        ("provenance-tree-mismatch", lambda state: replace(state, provenance_tree=old_tree), False),
        ("strict-false", lambda state: replace(state, strict=False), False),
        ("context-set-mismatch", lambda state: replace(state, required_contexts=frozenset({"acceptance", "version-prepared"})), False),
        ("run-zero", lambda state: replace(state, runs=()), False),
        ("run-two", lambda state: replace(state, runs=state.runs + (replace(state.runs[0], run_id="run-102"),)), False),
        ("stale-mixed-run", lambda state: replace(state, runs=(state.runs[0], replace(state.runs[0], run_id="run-old", head_sha=old_head, base_sha=old_base, tree_sha=old_tree))), False),
        ("empty-run-id", lambda state: run_update(state, run_id=""), False),
        ("empty-job-id", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], job_id=""), *state.runs[0].jobs[1:])), False),
        ("empty-artifact-id", lambda state: run_update(state, artifact_ids=("",)), False),
        ("empty-provenance-marker", lambda state: run_update(state, provenance_markers=("",)), False),
        ("empty-provenance-id", lambda state: replace(state, provenance_id=""), False),
        ("short-current-head", lambda state: replace(state, current_head="head-1"), False),
        ("empty-current-tree", lambda state: replace(state, current_tree=""), False),
    )
    for name, mutate, expected in mutations:
        result = evaluate_merge_state(mutate(valid))
        if result is not expected:
            raise AssertionError(f"merge fixture {name} expected {expected}, got {result}")
    return len(mutations)


def self_test() -> tuple[int, int]:
    return _run_static_cases(), _run_merge_cases()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check the Windows workflow quality oracle")
    parser.add_argument("--self-test", action="store_true", help="run baseline, mutation, and merge fixtures")
    args = parser.parse_args(argv)
    if not args.self_test:
        parser.error("--self-test is required")
    static_cases, merge_cases = self_test()
    if static_cases <= 0 or merge_cases <= 0:
        raise AssertionError("fixture case counts must be positive")
    print(f"workflow-quality-gate: PASS static_cases={static_cases} merge_cases={merge_cases}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Finite fail-closed contracts for the final-head Windows quality graph."""

from __future__ import annotations

import argparse
import copy
import json
import sys
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION_WORKFLOW = ROOT / ".github" / "workflows" / "version-prepare.yml"
WINDOWS_WORKFLOW = ROOT / ".github" / "workflows" / "windows-client.yml"
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"
RUST_WORKFLOW = ROOT / ".github" / "workflows" / "rust.yml"
CODEQL_WORKFLOW = ROOT / ".github" / "workflows" / "codeql.yml"

EXPECTED_RULESET_ID = 21746295
EXPECTED_RULE_SOURCE = "salty919/codex_info_v2"
EXPECTED_CONTEXTS = frozenset({"acceptance", "version-prepared"})
BINARY_IMPACT_JOB_IF = (
    "inputs.premerge == true && "
    "needs.version-prepared.outputs.binary_impact == 'true' && "
    "needs.version-prepared.outputs.ready == 'true'"
)


class WorkflowError(ValueError):
    """Raised when a workflow contract is absent or ambiguous."""


def _count(source: str, needle: str, expected: int, errors: list[str]) -> None:
    actual = source.count(needle)
    if actual != expected:
        errors.append(f"count {needle!r}: expected {expected}, found {actual}")


def _between(source: str, start: str, end: str) -> str:
    try:
        start_index = source.index(start)
        end_index = source.index(end, start_index + len(start))
    except ValueError as error:
        raise WorkflowError(f"missing boundary: {start!r} .. {end!r}") from error
    return source[start_index:end_index]


def validate_sources(
    version: str, windows: str, release: str, rust: str, codeql: str
) -> list[str]:
    errors: list[str] = []

    version_trigger = _between(version, "on:\n", "permissions: {}\n")
    for marker in (
        "  pull_request_target:\n",
        '    branches: ["main"]\n',
        "    types: [opened, synchronize, reopened, ready_for_review]\n",
    ):
        _count(version_trigger, marker, 1, errors)
    for forbidden in ("pull_request:\n", "workflow_dispatch:", "/dispatches"):
        _count(version, forbidden, 0, errors)
    for job in (
        "  prepare:\n",
        "  register-final-head-checks:\n",
        "  quality:\n",
        "  finalize-final-head-checks:\n",
    ):
        _count(version, job, 1, errors)
    _count(version, "  dispatch-quality:\n", 0, errors)
    _count(version, "uses: ./.github/workflows/windows-client.yml", 1, errors)
    _count(version, "scripts/final_head_check_reporter.py register", 1, errors)
    _count(version, "scripts/final_head_check_reporter.py finalize", 1, errors)
    _count(version, "      checks: write\n", 2, errors)
    _count(version, "      actions: write\n", 0, errors)
    _count(version, "      contents: write\n", 1, errors)
    _count(version, "      premerge: true\n", 1, errors)
    _count(
        version,
        "      head_sha: ${{ needs.prepare.outputs.final_head_sha }}\n",
        1,
        errors,
    )
    _count(
        version,
        "    if: always() && needs.register-final-head-checks.outputs.quality_required == 'true'\n",
        1,
        errors,
    )
    for artifact in ("acceptance-verdict", "release-candidate"):
        _count(version, f"          name: {artifact}\n", 1, errors)

    windows_trigger = _between(windows, "on:\n", "permissions:\n")
    _count(windows_trigger, "  workflow_call:\n", 1, errors)
    for forbidden in (
        "  pull_request_target:\n",
        "  workflow_dispatch:\n",
        "  pull_request:\n",
    ):
        _count(windows_trigger, forbidden, 0, errors)
    for input_name in (
        "premerge",
        "pr_number",
        "base_repository",
        "head_repository",
        "base_sha",
        "head_sha",
        "head_ref",
    ):
        _count(windows_trigger, f"      {input_name}:\n", 1, errors)
    _count(windows, f"    if: {BINARY_IMPACT_JOB_IF}\n", 4, errors)
    _count(windows, "    if: inputs.premerge == true\n", 1, errors)
    _count(windows, "    if: always() && inputs.premerge == true\n", 1, errors)
    _count(windows, "          ref: ${{ inputs.head_sha }}\n", 4, errors)
    _count(windows, "      source_sha: ${{ inputs.head_sha }}\n", 2, errors)
    _count(windows, "      head_ref: ${{ inputs.head_ref }}\n", 1, errors)
    _count(
        windows,
        "needs: [version-prepared, native-quality, codeql-analysis, windows-quality, ui-quality]",
        1,
        errors,
    )
    _count(windows, "          name: acceptance-verdict\n", 1, errors)
    _count(windows, "          name: release-candidate\n", 1, errors)
    _count(windows, "codex-info-final-head-v1", 1, errors)
    _count(windows, "event=workflow_dispatch", 0, errors)
    _count(windows, "  release:\n", 0, errors)
    _count(windows, "contents: write\n", 0, errors)

    release_trigger = _between(release, "on:\n", "permissions:\n")
    for marker in (
        "  pull_request_target:\n",
        '    branches: ["main"]\n',
        "    types: [closed]\n",
    ):
        _count(release_trigger, marker, 1, errors)
    for forbidden in (
        "  workflow_call:\n",
        "  workflow_dispatch:\n",
        "  pull_request:\n",
    ):
        _count(release_trigger, forbidden, 0, errors)
    _count(release, "  release:\n", 1, errors)
    _count(release, "    if: github.event.pull_request.merged == true\n", 1, errors)
    _count(release, "      contents: write\n", 1, errors)
    _count(release, "          ref: refs/heads/main\n", 1, errors)
    _count(release, "          name: release-candidate\n", 1, errors)
    _count(release, "commits/$PR_HEAD_SHA/check-runs?check_name=acceptance", 1, errors)
    _count(release, '--workflow-run "$workflow_run_json"', 1, errors)
    release_job = release[release.index("  release:\n") :]
    for forbidden in (
        "cargo build",
        "cargo test",
        "dotnet test",
        "github/codeql-action/",
        "Run-WindowsClientE2E",
        "final_acceptance_gate.sh",
    ):
        _count(release_job, forbidden, 0, errors)

    for source, label in ((rust, "rust"), (codeql, "codeql")):
        _count(source, "      source_sha:\n", 1, errors)
        _count(source, "          persist-credentials: false\n", 1, errors)
        if "  pull_request:\n" in source or "  push:\n" in source:
            errors.append(f"{label} reusable workflow has a direct PR/push trigger")
    _count(codeql, "  workflow_dispatch:\n", 0, errors)
    _count(codeql, "  schedule:\n", 1, errors)
    _count(rust, "          ref: ${{ inputs.source_sha }}\n", 1, errors)
    _count(codeql, "          ref: ${{ inputs.source_sha || github.sha }}\n", 1, errors)
    _count(codeql, "      head_ref:\n", 1, errors)
    _count(codeql, "          sha: ${{ inputs.source_sha || github.sha }}\n", 1, errors)
    _count(codeql, "github/codeql-action/analyze@v4", 1, errors)
    _count(codeql, "github/codeql-action/autobuild@", 0, errors)

    owner_counts = {
        "cargo build --release --locked": 1,
        "scripts/cli_contract_e2e.sh": 1,
        "scripts/record_daemon_e2e.sh": 1,
        "Build-WindowsInstaller.ps1": 1,
        "Run-WindowsClientE2E.ps1": 1,
        "scripts/windows_window_move_smoke.ps1": 1,
        "scripts/final_acceptance_gate.sh": 1,
    }
    quality_sources = windows + rust
    for owner, expected in owner_counts.items():
        _count(quality_sources, owner, expected, errors)
    for outcome in ("success", "skipped"):
        _count(
            windows,
            f'                [[ "$result" == {outcome} ]] || {{\n',
            1,
            errors,
        )
    if release.index("Resolve accepted PR quality run") > release.index(
        "Download accepted PR release candidate"
    ):
        errors.append("release artifact download precedes accepted-run resolution")
    return errors


def validate_workflows() -> list[str]:
    try:
        return validate_sources(
            VERSION_WORKFLOW.read_text(encoding="utf-8"),
            WINDOWS_WORKFLOW.read_text(encoding="utf-8"),
            RELEASE_WORKFLOW.read_text(encoding="utf-8"),
            RUST_WORKFLOW.read_text(encoding="utf-8"),
            CODEQL_WORKFLOW.read_text(encoding="utf-8"),
        )
    except (OSError, UnicodeError, WorkflowError, ValueError) as error:
        return [str(error)]


def _mapping(value: object) -> Mapping[str, object] | None:
    return value if isinstance(value, Mapping) else None


def _valid_rule_metadata(rule: Mapping[str, object]) -> bool:
    return (
        set(rule)
        == {"type", "parameters", "ruleset_source_type", "ruleset_source", "ruleset_id"}
        and type(rule.get("ruleset_id")) is int
        and rule.get("ruleset_id") == EXPECTED_RULESET_ID
        and rule.get("ruleset_source_type") == "Repository"
        and rule.get("ruleset_source") == EXPECTED_RULE_SOURCE
    )


def _valid_applied_rules(payload: object) -> bool:
    if type(payload) is not list or len(payload) != 2:
        return False
    rules = [_mapping(item) for item in payload]
    if any(rule is None for rule in rules):
        return False
    typed_rules = {rule.get("type"): rule for rule in rules if rule is not None}
    if set(typed_rules) != {"required_status_checks", "code_scanning"}:
        return False
    status = typed_rules["required_status_checks"]
    code_scanning = typed_rules["code_scanning"]
    if not _valid_rule_metadata(status) or not _valid_rule_metadata(code_scanning):
        return False
    parameters = _mapping(status.get("parameters"))
    if parameters is None or set(parameters) != {
        "required_status_checks",
        "strict_required_status_checks_policy",
        "do_not_enforce_on_create",
    }:
        return False
    checks = parameters.get("required_status_checks")
    if (
        parameters.get("strict_required_status_checks_policy") is not True
        or parameters.get("do_not_enforce_on_create") is not False
        or type(checks) is not list
        or len(checks) != 2
    ):
        return False
    normalized = []
    for check in checks:
        entry = _mapping(check)
        if entry is None or set(entry) != {"context", "integration_id"}:
            return False
        if (
            type(entry.get("context")) is not str
            or entry.get("integration_id") != 15368
        ):
            return False
        normalized.append(entry["context"])
    if len(set(normalized)) != 2 or set(normalized) != EXPECTED_CONTEXTS:
        return False
    code_parameters = _mapping(code_scanning.get("parameters"))
    if code_parameters is None or set(code_parameters) != {"code_scanning_tools"}:
        return False
    tools = code_parameters.get("code_scanning_tools")
    if type(tools) is not list or len(tools) != 1:
        return False
    codeql = _mapping(tools[0])
    return (
        codeql is not None
        and set(codeql) == {"tool", "alerts_threshold", "security_alerts_threshold"}
        and codeql.get("tool") == "CodeQL"
        and codeql.get("alerts_threshold") == "errors"
        and codeql.get("security_alerts_threshold") == "high_or_higher"
    )


def validate_live_applied_rules_json(raw: str) -> bool:
    def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON key")
            result[key] = value
        return result

    try:
        payload = json.loads(raw, object_pairs_hook=unique_object)
    except (json.JSONDecodeError, TypeError, UnicodeError, ValueError):
        return False
    return _valid_applied_rules(payload)


def _valid_rules_fixture() -> list[dict[str, object]]:
    metadata = {
        "ruleset_source_type": "Repository",
        "ruleset_source": EXPECTED_RULE_SOURCE,
        "ruleset_id": EXPECTED_RULESET_ID,
    }
    return [
        {
            "type": "required_status_checks",
            "parameters": {
                "required_status_checks": [
                    {"context": "acceptance", "integration_id": 15368},
                    {"context": "version-prepared", "integration_id": 15368},
                ],
                "strict_required_status_checks_policy": True,
                "do_not_enforce_on_create": False,
            },
            **metadata,
        },
        {
            "type": "code_scanning",
            "parameters": {
                "code_scanning_tools": [
                    {
                        "tool": "CodeQL",
                        "alerts_threshold": "errors",
                        "security_alerts_threshold": "high_or_higher",
                    }
                ]
            },
            **metadata,
        },
    ]


Mutation = Callable[[list[str]], None]


def _static_self_test() -> int:
    sources = [
        VERSION_WORKFLOW.read_text(encoding="utf-8"),
        WINDOWS_WORKFLOW.read_text(encoding="utf-8"),
        RELEASE_WORKFLOW.read_text(encoding="utf-8"),
        RUST_WORKFLOW.read_text(encoding="utf-8"),
        CODEQL_WORKFLOW.read_text(encoding="utf-8"),
    ]
    baseline = validate_sources(*sources)
    if baseline:
        raise AssertionError(
            "production workflow contract failed: " + "; ".join(baseline)
        )

    mutations: tuple[tuple[int, str, str], ...] = (
        (0, "  pull_request_target:\n", "  pull_request:\n"),
        (0, "scripts/final_head_check_reporter.py register", "true"),
        (0, "      checks: write\n", "      checks: read\n"),
        (0, "uses: ./.github/workflows/windows-client.yml", "uses: ./other.yml"),
        (1, "  workflow_call:\n", "  workflow_dispatch:\n"),
        (1, "permissions:\n  contents: read\n", "permissions:\n  contents: write\n"),
        (1, "          ref: ${{ inputs.head_sha }}\n", "          ref: feat/next\n"),
        (
            1,
            '                [[ "$result" == success ]] || {\n',
            '                [[ "$result" == skipped ]] || {\n',
        ),
        (
            1,
            "scripts/final_acceptance_gate.sh",
            "scripts/cli_contract_e2e.sh",
        ),
        (1, "Run-WindowsClientE2E.ps1", "Build-WindowsInstaller.ps1"),
        (2, "    types: [closed]\n", "    types: [opened]\n"),
        (2, "      contents: write\n", "      contents: read\n"),
        (2, "commits/$PR_HEAD_SHA/check-runs?check_name=acceptance", "actions/runs"),
        (3, "          ref: ${{ inputs.source_sha }}\n", "          ref: feat/next\n"),
        (4, "          sha: ${{ inputs.source_sha || github.sha }}\n", ""),
        (4, "  schedule:\n", "  workflow_dispatch:\n"),
        (4, "github/codeql-action/analyze@v4", "github/codeql-action/autobuild@v4"),
    )
    cases = 1
    for index, needle, replacement in mutations:
        candidate = sources.copy()
        if candidate[index].count(needle) < 1:
            raise AssertionError(f"missing mutation target: {needle!r}")
        candidate[index] = candidate[index].replace(needle, replacement, 1)
        if not validate_sources(*candidate):
            raise AssertionError(f"workflow mutation was accepted: {needle!r}")
        cases += 1
    return cases


def _rules_self_test() -> int:
    valid = _valid_rules_fixture()
    cases: list[object] = [valid]
    for mutation in (
        lambda value: value.pop(),
        lambda value: value[0].update(ruleset_id=1),
        lambda value: value[0]["parameters"].update(
            strict_required_status_checks_policy=False
        ),
        lambda value: value[0]["parameters"]["required_status_checks"][0].update(
            integration_id=1
        ),
        lambda value: value[1]["parameters"]["code_scanning_tools"][0].update(
            alerts_threshold="none"
        ),
    ):
        candidate = copy.deepcopy(valid)
        mutation(candidate)
        cases.append(candidate)
    if not validate_live_applied_rules_json(json.dumps(cases[0])):
        raise AssertionError("valid rules fixture was rejected")
    for candidate in cases[1:]:
        if validate_live_applied_rules_json(json.dumps(candidate)):
            raise AssertionError("invalid rules fixture was accepted")
    for malformed in ("", "{}", "null", '{"a":1,"a":2}'):
        if validate_live_applied_rules_json(malformed):
            raise AssertionError("malformed rules JSON was accepted")
    return len(cases) + 4


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--self-test", action="store_true")
    modes.add_argument("--validate-live-applied-rules", action="store_true")
    args = parser.parse_args(argv)
    if args.validate_live_applied_rules:
        if not validate_live_applied_rules_json(sys.stdin.read()):
            print("workflow-quality-gate: FAIL live-applied-rules", file=sys.stderr)
            return 1
        print("workflow-quality-gate: PASS live-applied-rules")
        return 0
    static_cases = _static_self_test()
    rules_cases = _rules_self_test()
    print(
        f"workflow-quality-gate: PASS static_cases={static_cases} "
        f"rules_cases={rules_cases}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

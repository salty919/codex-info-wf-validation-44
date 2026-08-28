#!/usr/bin/env python3
"""Fail-closed quality oracle for the Windows client workflow.

This is intentionally a small, line-oriented parser for the two workflow
shapes used by this repository.  It is not a general YAML parser: unsupported
or ambiguous structure is rejected.  The merge table below is synthetic test
data only; live merge-policy evidence is validated separately from GitHub's
applied-rules endpoint.
"""

from __future__ import annotations

import argparse
from copy import deepcopy
from dataclasses import dataclass, replace
import json
from pathlib import Path
import re
import shutil
import sys
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
    "release_build": re.compile(r"(?:^|[\s;&|])cargo build --release --locked(?:\s|$)"),
    "cli_contract_e2e": re.compile(r"(?:^|[\s'\"./])scripts/cli_contract_e2e\.sh(?:\s|$)"),
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
    "release_build": "native-quality",
    "cli_contract_e2e": "native-quality",
    "record_daemon_e2e": "native-quality",
    "Build-WindowsInstaller": "ui-quality",
    "Run-WindowsClientE2E": "ui-quality",
    "windows_window_move_smoke": "ui-quality",
    "final_acceptance_gate": "acceptance",
}
PR_OWNERS = frozenset(OWNER_JOBS)
LOCAL_OWNERS = frozenset(
    {"regression_guard", "data_protection_gate", "windows_client_contract_gate"}
)
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
LIVE_AUDIT_ENDPOINT = 'gh api --method GET -H "X-GitHub-Api-Version: 2026-03-10" "repos/$REPOSITORY/rules/branches/main"'
LIVE_AUDIT_VALIDATOR = "env -u GH_TOKEN python3 scripts/workflow_quality_gate.py --validate-live-applied-rules"
EXPECTED_LIVE_RULESET_ID = 21746295
EXPECTED_LIVE_RULE_SOURCE_TYPE = "Repository"
EXPECTED_LIVE_RULE_SOURCE = "salty919/codex_info_v2"
EXPECTED_LIVE_RULE_KEYS = frozenset(
    {"type", "parameters", "ruleset_source_type", "ruleset_source", "ruleset_id"}
)
EXPECTED_LIVE_STATUS_CONTEXTS = frozenset({"acceptance", "version-prepared"})


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


_POWERSHELL_PATH_CHARS = frozenset("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.-")


def _powershell_relative_prefix_is_command(prefix: str) -> bool:
    """Accept only the finite relative-path prefix used by current PS steps.

    ``prefix`` ends immediately before the owner filename.  A call operator
    may precede the path (the smoke step uses ``@(& ./scripts``); anything
    that looks like a command word, an empty path component, or ``..`` is
    rejected.  The explicit character scan avoids an ambiguous repeated
    regex over attacker-controlled workflow text.
    """
    left = 0
    right = len(prefix)
    while left < right and prefix[left].isspace():
        left += 1
    while right > left and prefix[right - 1].isspace():
        right -= 1
    if left == right:
        return True

    # Keep only the path after the final PowerShell call operator.  The
    # caller has already removed quoted text, and a command word without '&'
    # remains part of the candidate and therefore fails path validation.
    call = -1
    index = left
    while index < right:
        if prefix[index] == "&":
            call = index
        index += 1
    if call >= 0:
        # The current array-expression form has only a variable assignment
        # and ``@(`` before ``&``.  Reject ordinary command words such as
        # ``Write-Output & .`` instead of treating every ampersand as a
        # trusted command boundary.
        index = left
        while index < call:
            character = prefix[index]
            if character.isspace() or character in "@()=":
                index += 1
                continue
            if character == "$":
                index += 1
                if index >= call or prefix[index] not in _POWERSHELL_PATH_CHARS:
                    return False
                while index < call and prefix[index] in _POWERSHELL_PATH_CHARS:
                    index += 1
                continue
            return False
        left = call + 1
        while left < right and prefix[left].isspace():
            left += 1
    if left == right:
        return False

    # A bare '.' is the prefix left by the marker scanner for './owner.ps1'.
    if right - left == 1 and prefix[left] == ".":
        return True
    if right - left < 3 or prefix[left : left + 2] != "./":
        return False

    component_start = left + 2
    if component_start == right:
        return False
    index = component_start
    while index <= right:
        if index == right or prefix[index] == "/":
            if index == component_start:
                return False
            component = prefix[component_start:index]
            if component == "..":
                return False
            component_start = index + 1
        elif prefix[index] not in _POWERSHELL_PATH_CHARS:
            return False
        index += 1
    return component_start == right + 1


def _owner_occurrences(owner: str, text: str) -> int:
    """Count marker tokens only where the surrounding syntax invokes them."""
    pattern = OWNER_PATTERNS[owner]
    if owner == "release_build":
        return sum(len(pattern.findall(line)) for line in text.splitlines())
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
            elif _powershell_relative_prefix_is_command(segment):
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


def _live_audit_contract(workflow: Workflow) -> list[str]:
    """Require one fail-closed applied-rules audit before provenance output."""
    errors: list[str] = []
    job = workflow.jobs.get("windows-quality")
    if job is None:
        return ["missing windows-quality job for live policy audit"]
    bodies = [
        "\n".join(
            line for line in block.body.splitlines() if not line.lstrip().startswith("#")
        )
        for block in job.run_blocks
    ]
    endpoint_hits = sum(body.count(LIVE_AUDIT_ENDPOINT) for body in bodies)
    validator_hits = sum(body.count(LIVE_AUDIT_VALIDATOR) for body in bodies)
    if endpoint_hits != 1:
        errors.append(f"live policy endpoint cardinality is {endpoint_hits}")
    if validator_hits != 1:
        errors.append(f"live policy validator cardinality is {validator_hits}")
    audit_bodies = [
        body
        for body in bodies
        if LIVE_AUDIT_ENDPOINT in body or LIVE_AUDIT_VALIDATOR in body
    ]
    if len(audit_bodies) != 1:
        return errors + ["live policy audit command scope is ambiguous"]
    body = audit_bodies[0]
    if "set -euo pipefail" not in body:
        errors.append("live policy audit must enable pipefail")
    endpoint_position = body.find(LIVE_AUDIT_ENDPOINT)
    validator_position = body.find(LIVE_AUDIT_VALIDATOR)
    if endpoint_position < 0 or validator_position < 0 or endpoint_position >= validator_position:
        errors.append("live policy endpoint must pipe into its validator")
    if re.search(r"\|\||\b(?:fallback|cache)\b", body, re.IGNORECASE):
        errors.append("live policy audit must not use fallback or cache")

    markers = [
        index
        for index, line in enumerate(workflow.lines)
        if line == "      - name: Audit live applied merge rules"
    ]
    if len(markers) != 1:
        errors.append("live policy audit step name cardinality is not one")
    else:
        start = markers[0]
        end = next(
            (
                index
                for index in range(start + 1, len(workflow.lines))
                if workflow.lines[index].startswith("      - name:")
            ),
            len(workflow.lines),
        )
        step_text = "\n".join(workflow.lines[start:end])
        if (
            step_text.count("GH_TOKEN:") != 1
            or step_text.count("GH_TOKEN: ${{ github.token }}") != 1
        ):
            errors.append("live policy audit token must be step-local")
        if (
            step_text.count("REPOSITORY:") != 1
            or step_text.count("REPOSITORY: ${{ github.repository }}") != 1
        ):
            errors.append("live policy audit repository must come from repository context")
        job_start = next(
            (index for index, line in enumerate(workflow.lines) if line == "  windows-quality:"),
            -1,
        )
        job_end = next(
            (
                index
                for index in range(job_start + 1, len(workflow.lines))
                if re.match(r"^  [A-Za-z0-9_.-]+:", workflow.lines[index])
            ),
            len(workflow.lines),
        )
        checkout_indices = [
            index
            for index in range(job_start + 1, job_end)
            if workflow.lines[index] == "      - uses: actions/checkout@v4"
        ]
        provenance_indices = [
            index
            for index in range(job_start + 1, job_end)
            if workflow.lines[index] == "      - name: Write Windows quality evidence"
        ]
        if (
            len(checkout_indices) != 1
            or len(provenance_indices) != 1
            or markers[0] <= checkout_indices[0]
            or markers[0] >= provenance_indices[0]
        ):
            errors.append("live policy audit must run after checkout and before provenance output")
        step_markers = [
            index
            for index in range(job_start + 1, job_end)
            if workflow.lines[index].startswith("      - ")
        ]
        if len(step_markers) != 4:
            errors.append("windows-quality must contain checkout, live audit, provenance, and upload steps only")
        else:
            try:
                checkout_step = step_markers.index(checkout_indices[0])
                audit_step = step_markers.index(markers[0])
                provenance_step = step_markers.index(provenance_indices[0])
            except ValueError:
                errors.append("windows-quality step boundaries are ambiguous")
            else:
                if (checkout_step, audit_step, provenance_step) != (0, 1, 2):
                    errors.append("windows-quality must audit immediately after checkout and write provenance next")
        job_text = "\n".join(workflow.lines[job_start:job_end])
        for forbidden in (
            "apt-get",
            "actions/setup-dotnet@",
            "scripts/windows_client_contract_gate.sh",
            "WINDOWS_CONTRACT_EVIDENCE_DIR",
        ):
            if forbidden in job_text:
                errors.append(f"windows-quality must not contain local gate setup or execution: {forbidden}")
        provenance_text = "\n".join(workflow.lines[provenance_indices[0] : job_end]) if provenance_indices else ""
        for marker in (
            "quality: merge-policy",
            "source-sha:",
            "tree-sha:",
            "live-applied-rules: PASS",
            "merge-policy: PASS",
        ):
            if marker not in provenance_text:
                errors.append(f"Windows merge-policy evidence is missing {marker}")
        for marker in ("windows-contract: PASS", "windows-tests: PASS", "windows-quality: PASS"):
            if marker in provenance_text:
                errors.append(f"Windows merge-policy evidence retains obsolete marker {marker}")
        for env_start in (
            index
            for index in range(job_start + 1, job_end)
            if workflow.lines[index] == "    env:"
        ):
            env_end = next(
                (
                    index
                    for index in range(env_start + 1, job_end)
                    if workflow.lines[index].strip()
                    and _indent(workflow.lines[index]) <= 4
                ),
                job_end,
            )
            if any(
                workflow.lines[index].lstrip().startswith("GH_TOKEN:")
                for index in range(env_start + 1, env_end)
            ):
                errors.append("live policy audit token must not be job-wide")
    return errors


def _native_artifact_contract(workflow: Workflow) -> list[str]:
    """Require the native job's three execution and evidence owners."""
    job = workflow.jobs.get("native-quality")
    if job is None:
        return ["missing native-quality job for native artifact contract"]
    text = "\n".join(block.body for block in job.run_blocks)
    errors: list[str] = []
    for marker in (
        "release-build: PASS",
        "cli-contract-e2e: PASS",
        "recorder-daemon: PASS",
    ):
        if text.count(marker) != 1:
            errors.append(f"native evidence marker cardinality is not one: {marker}")
    for marker in ("data-protection: PASS", "regression-guard: PASS"):
        if marker in text:
            errors.append(f"native evidence retains local-only marker: {marker}")
    return errors


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
        if acceptance_if != "always() && github.event_name == 'pull_request'":
            errors.append("acceptance must always instantiate on pull_request")
        acceptance = windows.jobs.get("acceptance")
        if acceptance is None or not acceptance.run_blocks:
            errors.append("acceptance outcome check is missing")
        else:
            first_acceptance_run = acceptance.run_blocks[0].body
            for required in (
                '[[ "$VERSION_RESULT" == success ]]',
                '[[ "$VERSION_READY" == true ]]',
                'for result in "$NATIVE_RESULT" "$WINDOWS_RESULT" "$UI_RESULT"',
                '[[ "$result" == success ]]',
            ):
                if required not in first_acceptance_run:
                    errors.append(f"acceptance first step is missing outcome guard: {required}")
            acceptance_start = next(
                (index for index, line in enumerate(windows.lines) if line == "  acceptance:"),
                -1,
            )
            acceptance_end = next(
                (
                    index
                    for index in range(acceptance_start + 1, len(windows.lines))
                    if re.match(r"^  [A-Za-z0-9_.-]+:", windows.lines[index])
                ),
                len(windows.lines),
            )
            acceptance_text = "\n".join(windows.lines[acceptance_start:acceptance_end])
            for required in (
                "VERSION_RESULT:",
                "VERSION_READY:",
                "NATIVE_RESULT:",
                "WINDOWS_RESULT:",
                "UI_RESULT:",
            ):
                if required not in acceptance_text:
                    errors.append(f"acceptance first step environment is missing {required}")
        release_if = windows.jobs.get("release", Job("", {}, (), ())).properties.get("if", "")
        if not (
            "github.event_name == 'pull_request_target'" in release_if
            and "github.event.pull_request.merged == true" in release_if
        ):
            errors.append("release is not closed-and-merged-only")

        counts = _owner_counts((windows, rust))
        for owner in PR_OWNERS:
            blocks = counts[owner]
            if len(blocks) != 1:
                errors.append(f"PR owner {owner} cardinality is {len(blocks)}")
            elif blocks[0].job != OWNER_JOBS[owner]:
                errors.append(f"PR owner {owner} is in job {blocks[0].job}")
        for owner in LOCAL_OWNERS:
            blocks = counts[owner]
            if blocks:
                errors.append(f"local owner {owner} appears in PR workflows: {len(blocks)}")
        if "acceptance" in windows.jobs and "release" in windows.jobs:
            errors.extend(_quality_commands(windows, ("acceptance", "release")))
        errors.extend(_native_artifact_contract(rust))
        errors.extend(_live_audit_contract(windows))

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
    current_base: str
    current_head: str
    current_tree: str
    provenance_source: str
    provenance_tree: str
    runs: tuple[MergeRun, ...]
    provenance_id: str = ""
    live_applied_rules: object | None = None


_SHA40 = re.compile(r"^[0-9a-fA-F]{40}$")


def _nonempty_identifier(value: object) -> bool:
    return value is not None and str(value).strip() not in {"", "0"}


def _full_sha(value: object) -> bool:
    return isinstance(value, str) and _SHA40.fullmatch(value) is not None


def _mapping(value: object) -> Mapping[str, object] | None:
    return value if isinstance(value, Mapping) else None


def _valid_live_rule_metadata(rule: Mapping[str, object]) -> bool:
    return (
        set(rule) == EXPECTED_LIVE_RULE_KEYS
        and type(rule.get("ruleset_id")) is int
        and rule["ruleset_id"] == EXPECTED_LIVE_RULESET_ID
        and rule.get("ruleset_source_type") == EXPECTED_LIVE_RULE_SOURCE_TYPE
        and rule.get("ruleset_source") == EXPECTED_LIVE_RULE_SOURCE
    )


def _valid_live_required_status_rule(rule: Mapping[str, object]) -> bool:
    if not _valid_live_rule_metadata(rule) or rule.get("type") != "required_status_checks":
        return False
    parameters = _mapping(rule.get("parameters"))
    if parameters is None or set(parameters) != {
        "required_status_checks",
        "strict_required_status_checks_policy",
        "do_not_enforce_on_create",
    }:
        return False
    if parameters.get("strict_required_status_checks_policy") is not True:
        return False
    if parameters.get("do_not_enforce_on_create") is not False:
        return False
    checks = parameters.get("required_status_checks")
    if type(checks) is not list or len(checks) != len(EXPECTED_LIVE_STATUS_CONTEXTS):
        return False
    contexts: list[str] = []
    for check in checks:
        entry = _mapping(check)
        if entry is None or set(entry) != {"context", "integration_id"}:
            return False
        if type(entry.get("context")) is not str:
            return False
        if type(entry.get("integration_id")) is not int or entry["integration_id"] != 15368:
            return False
        contexts.append(entry["context"])
    return len(contexts) == len(set(contexts)) and set(contexts) == EXPECTED_LIVE_STATUS_CONTEXTS


def _valid_live_codeql_rule(rule: Mapping[str, object]) -> bool:
    if not _valid_live_rule_metadata(rule) or rule.get("type") != "code_scanning":
        return False
    parameters = _mapping(rule.get("parameters"))
    if parameters is None or set(parameters) != {"code_scanning_tools"}:
        return False
    tools = parameters.get("code_scanning_tools")
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


def _valid_live_applied_rules(payload: object) -> bool:
    """Validate GitHub's raw GET /rules/branches/main response exactly."""
    if type(payload) is not list or len(payload) != 2:
        return False
    rules: list[Mapping[str, object]] = []
    for item in payload:
        rule = _mapping(item)
        if rule is None:
            return False
        rules.append(rule)
    types = [rule.get("type") for rule in rules]
    if any(type(rule_type) is not str for rule_type in types):
        return False
    if len(set(types)) != len(types) or set(types) != {"required_status_checks", "code_scanning"}:
        return False
    return all(
        _valid_live_required_status_rule(rule)
        if rule.get("type") == "required_status_checks"
        else _valid_live_codeql_rule(rule)
        for rule in rules
    )


def validate_live_applied_rules_json(raw: str) -> bool:
    """Parse and validate one raw applied-rules JSON document."""

    def reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON object key")
            result[key] = value
        return result

    try:
        payload = json.loads(raw, object_pairs_hook=reject_duplicate_keys)
    except (json.JSONDecodeError, TypeError, UnicodeDecodeError, ValueError):
        return False
    return _valid_live_applied_rules(payload)


def evaluate_merge_state(state: MergeState) -> bool:
    """Return ALLOW only for one exact, current-head synthetic run set.

    Live applied-rules evidence is checked here so callers cannot bypass the
    authority schema with a hand-built normalized dataclass.  Pending CodeQL
    analysis and current-head enforcement remain GitHub's official ruleset
    responsibilities; this oracle intentionally verifies policy declaration
    without duplicating that runtime behavior.
    """
    required = set(EXPECTED_LIVE_STATUS_CONTEXTS)
    if not _valid_live_applied_rules(state.live_applied_rules):
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
                ),
                artifact_ids=("artifact-acceptance", "artifact-version"),
                provenance_markers=(f"source-sha: {head}", f"tree-sha: {tree}"),
            ),
        ),
        provenance_id="provenance-101",
        live_applied_rules=_valid_live_applied_rules_fixture(),
    )


def _valid_live_applied_rules_fixture() -> list[dict[str, object]]:
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
            "ruleset_source_type": EXPECTED_LIVE_RULE_SOURCE_TYPE,
            "ruleset_source": EXPECTED_LIVE_RULE_SOURCE,
            "ruleset_id": EXPECTED_LIVE_RULESET_ID,
        },
        {
            "type": "code_scanning",
            "parameters": {
                "code_scanning_tools": [
                    {
                        "tool": "CodeQL",
                        "alerts_threshold": "errors",
                        "security_alerts_threshold": "high_or_higher",
                    },
                ],
            },
            "ruleset_source_type": EXPECTED_LIVE_RULE_SOURCE_TYPE,
            "ruleset_source": EXPECTED_LIVE_RULE_SOURCE,
            "ruleset_id": EXPECTED_LIVE_RULESET_ID,
        },
    ]


def _copy_workflows(directory: Path) -> tuple[Path, Path]:
    windows = directory / "windows-client.yml"
    rust = directory / "rust.yml"
    shutil.copyfile(WINDOWS_WORKFLOW, windows)
    shutil.copyfile(RUST_WORKFLOW, rust)
    return windows, rust


def _run_live_applied_rules_cases() -> int:
    valid = _valid_live_applied_rules_fixture()
    cases: list[tuple[str, object, bool]] = [("valid", valid, True)]

    def mutated(name: str, mutate: Callable[[list[dict[str, object]]], None]) -> None:
        candidate = deepcopy(valid)
        mutate(candidate)
        cases.append((name, candidate, False))

    mutated("extra-rule", lambda candidate: candidate.append(deepcopy(candidate[0])))
    cases.append(("missing-rule", deepcopy(valid[:1]), False))
    malformed_parameters = deepcopy(valid)
    malformed_parameters[0]["parameters"] = None
    cases.append(("malformed-parameters", malformed_parameters, False))
    mutated(
        "duplicate-status-context",
        lambda candidate: candidate[0]["parameters"]["required_status_checks"][1].update(  # type: ignore[index]
            {"context": "acceptance"}
        ),
    )
    mutated(
        "wrong-source-type",
        lambda candidate: candidate[0].update({"ruleset_source_type": "Organization"}),
    )
    mutated(
        "wrong-source",
        lambda candidate: candidate[0].update({"ruleset_source": "other/repository"}),
    )
    mutated("wrong-ruleset-id", lambda candidate: candidate[0].update({"ruleset_id": 1}))
    mutated(
        "missing-ruleset-id",
        lambda candidate: candidate[0].pop("ruleset_id"),
    )
    mutated(
        "required-status-strict-false",
        lambda candidate: candidate[0]["parameters"].update(  # type: ignore[index]
            {"strict_required_status_checks_policy": False}
        ),
    )
    mutated(
        "required-status-create-enforced",
        lambda candidate: candidate[0]["parameters"].update(  # type: ignore[index]
            {"do_not_enforce_on_create": True}
        ),
    )
    mutated(
        "wrong-integration-id",
        lambda candidate: candidate[0]["parameters"]["required_status_checks"][0].update(  # type: ignore[index]
            {"integration_id": 1}
        ),
    )
    mutated(
        "codeql-alert-threshold",
        lambda candidate: candidate[1]["parameters"]["code_scanning_tools"][0].update(  # type: ignore[index]
            {"alerts_threshold": "none"}
        ),
    )
    mutated(
        "codeql-security-threshold",
        lambda candidate: candidate[1]["parameters"]["code_scanning_tools"][0].update(  # type: ignore[index]
            {"security_alerts_threshold": "medium_or_higher"}
        ),
    )
    mutated(
        "codeql-tool",
        lambda candidate: candidate[1]["parameters"]["code_scanning_tools"][0].update(  # type: ignore[index]
            {"tool": "Semgrep"}
        ),
    )
    with_extra_metadata = deepcopy(valid)
    with_extra_metadata[0]["bypass_actors"] = []
    cases.append(("extra-top-level-metadata", with_extra_metadata, False))

    for name, payload, expected in cases:
        result = validate_live_applied_rules_json(json.dumps(payload))
        if result is not expected:
            raise AssertionError(f"live applied-rules fixture {name} expected {expected}, got {result}")
    parse_failures = ("", "{", "null", "{}", "1", '"rules"')
    for raw in parse_failures:
        if validate_live_applied_rules_json(raw):
            raise AssertionError(f"live applied-rules malformed JSON/type was accepted: {raw!r}")
    valid_raw = json.dumps(valid)
    duplicate_key_raw = valid_raw.replace(
        '"type": "required_status_checks",',
        '"type": "required_status_checks", "type": "required_status_checks",',
        1,
    )
    if duplicate_key_raw == valid_raw or validate_live_applied_rules_json(duplicate_key_raw):
        raise AssertionError("duplicate JSON object key was accepted")
    return len(cases) + len(parse_failures) + 1


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
        needle = "          mkdir -p artifacts/windows-quality\n"
        replacement = needle + "          bash scripts/windows_client_contract_gate.sh\n"
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def duplicate_data_owner(_windows: Path, rust: Path) -> None:
        text = rust.read_text(encoding="utf-8")
        needle = "      - name: Write native quality evidence\n"
        replacement = "      - name: Run data protection quality gate\n        run: bash scripts/data_protection_gate.sh\n" + needle
        rust.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def missing_release_marker(_windows: Path, rust: Path) -> None:
        text = rust.read_text(encoding="utf-8")
        rust.write_text(
            _replace_exact(text, "          release-build: PASS\n", ""),
            encoding="utf-8",
        )

    def missing_cli_marker(_windows: Path, rust: Path) -> None:
        text = rust.read_text(encoding="utf-8")
        rust.write_text(
            _replace_exact(text, "          cli-contract-e2e: PASS\n", ""),
            encoding="utf-8",
        )

    def missing_merge_policy_marker(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(
            _replace_exact(text, "          merge-policy: PASS\n", ""),
            encoding="utf-8",
        )

    def local_apt_setup(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "          mkdir -p artifacts/windows-quality\n"
        replacement = needle + "          sudo apt-get update\n"
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def local_dotnet_setup(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "          mkdir -p artifacts/windows-quality\n"
        replacement = needle + "          actions/setup-dotnet@v4\n"
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def no_acceptance_always(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "if: always() && github.event_name == 'pull_request'", "if: github.event_name == 'pull_request'"), encoding="utf-8")

    def missing_version_outcome_guard(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = '          [[ "$VERSION_RESULT" == success ]] || { echo "version-prepared job did not succeed: $VERSION_RESULT" >&2; exit 1; }\n'
        windows.write_text(_replace_exact(text, needle, ""), encoding="utf-8")

    def missing_owner_outcome_guard(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = '          for result in "$NATIVE_RESULT" "$WINDOWS_RESULT" "$UI_RESULT"; do\n'
        replacement = '          for result in "$NATIVE_RESULT" "$WINDOWS_RESULT"; do\n'
        windows.write_text(_replace_exact(text, needle, replacement), encoding="utf-8")

    def add_path_filter(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "  pull_request:\n    branches:", "  pull_request:\n    paths: ['**']\n    branches:"), encoding="utf-8")

    def forbidden_quality_command(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        needle = "run: bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e artifacts/native-quality artifacts/windows-quality"
        replacement = "run: |\n          bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e artifacts/native-quality artifacts/windows-quality\n          cargo test"
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

    def live_audit_scope(text: str) -> tuple[int, int]:
        marker = "      - name: Audit live applied merge rules"
        if text.count(marker) != 1:
            raise AssertionError("live audit step marker is not unique")
        start = text.index(marker)
        end = text.find("\n      - name:", start + len(marker))
        if end < 0:
            raise AssertionError("live audit step has no following step")
        return start, end + 1

    def replace_live_audit(text: str, needle: str, replacement: str) -> str:
        start, end = live_audit_scope(text)
        scope = text[start:end]
        return text[:start] + _replace_exact(scope, needle, replacement) + text[end:]

    def missing_live_audit(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        start, end = live_audit_scope(text)
        windows.write_text(text[:start] + text[end:], encoding="utf-8")

    def wrong_live_endpoint(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(
            replace_live_audit(
                text,
                LIVE_AUDIT_ENDPOINT,
                'gh api --method GET -H "X-GitHub-Api-Version: 2025-01-01" "repos/$REPOSITORY/rules/branches/main"',
            ),
            encoding="utf-8",
        )

    def wrong_live_validator(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(
            replace_live_audit(
                text,
                LIVE_AUDIT_VALIDATOR,
                "python3 scripts/workflow_quality_gate.py --validate-live-applied-rules",
            ),
            encoding="utf-8",
        )

    def missing_live_pipefail(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(
            replace_live_audit(text, "set -euo pipefail", "set -eu"),
            encoding="utf-8",
        )

    def live_audit_after_setup(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        start, end = live_audit_scope(text)
        audit = text[start:end]
        without_audit = text[:start] + text[end:]
        insertion = without_audit.index("      - name: Upload Windows quality evidence")
        windows.write_text(
            without_audit[:insertion] + audit + without_audit[insertion:],
            encoding="utf-8",
        )

    def live_audit_job_token(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        marker = "  windows-quality:\n"
        insertion = text.index(marker) + len(marker)
        windows.write_text(
            text[:insertion] + "    env:\n      GH_TOKEN: ${{ github.token }}\n" + text[insertion:],
            encoding="utf-8",
        )

    def unknown_job(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "jobs:\n  version-prepared:", "jobs:\n  unexpected-job:\n    runs-on: ubuntu-latest\n  version-prepared:"), encoding="utf-8")

    def unknown_need(windows: Path, _rust: Path) -> None:
        text = windows.read_text(encoding="utf-8")
        windows.write_text(_replace_exact(text, "needs: [version-prepared]\n    runs-on: ubuntu-latest", "needs: [unknown-job]\n    runs-on: ubuntu-latest"), encoding="utf-8")

    return (
        ("dag-edge-missing", missing_dag),
        ("owner-duplicate", duplicate_owner),
        ("data-owner-pr-workflow", duplicate_data_owner),
        ("native-release-marker-missing", missing_release_marker),
        ("native-cli-marker-missing", missing_cli_marker),
        ("merge-policy-marker-missing", missing_merge_policy_marker),
        ("windows-local-apt-setup", local_apt_setup),
        ("windows-local-dotnet-setup", local_dotnet_setup),
        ("acceptance-always-missing", no_acceptance_always),
        ("acceptance-version-outcome-missing", missing_version_outcome_guard),
        ("acceptance-owner-outcome-missing", missing_owner_outcome_guard),
        ("pull-request-path-filter", add_path_filter),
        ("quality-command-in-acceptance", forbidden_quality_command),
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
        ("live-audit-missing", missing_live_audit),
        ("live-audit-endpoint", wrong_live_endpoint),
        ("live-audit-validator", wrong_live_validator),
        ("live-audit-pipefail", missing_live_pipefail),
        ("live-audit-order", live_audit_after_setup),
        ("live-audit-job-token", live_audit_job_token),
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


def _run_powershell_position_cases() -> int:
    prefix_cases = (
        ("empty-prefix", "", True),
        ("bare-relative", ".", True),
        ("relative-path", "./windows-client/tools", True),
        ("call-relative-path", "& ./windows-client/tools", True),
        ("array-call-relative-path", "$moveSmokeOutput = @(& .", True),
        ("quoted-prefix", 'echo "./windows-client/tools', False),
        ("command-word", "Write-Output ./windows-client/tools", False),
        ("command-before-call", "Write-Output & .", False),
        ("empty-path", "./", False),
        ("empty-component", "./windows-client//tools", False),
        ("parent-traversal", "./windows-client/../tools", False),
        ("unsafe-character", "./windows-client/tools\\nested", False),
        ("long-repetition", "./" * 10000, False),
    )
    for name, prefix, expected in prefix_cases:
        actual = _powershell_relative_prefix_is_command(prefix)
        if actual is not expected:
            raise AssertionError(f"PowerShell prefix fixture {name} expected {expected}, got {actual}")

    owner_cases = (
        ("Build-WindowsInstaller", "Build-WindowsInstaller.ps1"),
        ("Build-WindowsInstaller", "./windows-client/tools/Build-WindowsInstaller.ps1"),
        ("Run-WindowsClientE2E", "& ./windows-client/tools/Run-WindowsClientE2E.ps1"),
        (
            "windows_window_move_smoke",
            "$moveSmokeOutput = @(& ./scripts/windows_window_move_smoke.ps1",
        ),
    )
    for owner, command in owner_cases:
        if _owner_occurrences(owner, command) != 1:
            raise AssertionError(f"actual PowerShell owner command was not counted: {owner}: {command}")

    quoted = _active_run_text(RunBlock("ui-quality", 'echo "./windows-client/tools/Build-WindowsInstaller.ps1"', 1))
    if _owner_occurrences("Build-WindowsInstaller", quoted) != 0:
        raise AssertionError("quoted PowerShell owner fixture was counted")
    require_text = "require_text: ./windows-client/tools/Build-WindowsInstaller.ps1"
    if _owner_occurrences("Build-WindowsInstaller", require_text) != 0:
        raise AssertionError("require_text PowerShell owner fixture was counted")
    return len(prefix_cases) + len(owner_cases) + 2


def _run_static_cases() -> int:
    cases = 1
    baseline_errors = validate_workflows()
    if baseline_errors:
        raise AssertionError("production workflow baseline failed: " + "; ".join(baseline_errors))
    cases += _run_quality_wrapper_cases()
    cases += _run_powershell_position_cases()
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
        ("live-policy-missing", lambda state: replace(state, live_applied_rules=None), False),
        ("live-policy-wrong-type", lambda state: replace(state, live_applied_rules={}), False),
        ("job-failure", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], conclusion="failure"), *state.runs[0].jobs[1:])), False),
        ("job-cancelled", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], conclusion="cancelled"), *state.runs[0].jobs[1:])), False),
        ("job-in-progress", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], status="in_progress"), *state.runs[0].jobs[1:])), False),
        ("job-status-missing", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], status=""), *state.runs[0].jobs[1:])), False),
        ("job-status-pending", lambda state: jobs_update(state, (replace(state.runs[0].jobs[0], status="pending"), *state.runs[0].jobs[1:])), False),
        ("context-missing", lambda state: jobs_update(state, state.runs[0].jobs[:1]), False),
        ("context-extra", lambda state: jobs_update(state, (*state.runs[0].jobs, MergeJob("extra", job_id="job-extra"))), False),
        ("context-duplicate", lambda state: jobs_update(state, (state.runs[0].jobs[0], replace(state.runs[0].jobs[1], context="acceptance"))), False),
        ("base-behind", lambda state: run_update(state, base_sha=old_base), False),
        ("base-stale", lambda state: replace(state, current_base=old_base), False),
        ("head-mismatch", lambda state: run_update(state, head_sha=old_head), False),
        ("tree-mismatch", lambda state: run_update(state, tree_sha=old_tree), False),
        ("provenance-source-mismatch", lambda state: replace(state, provenance_source=old_head), False),
        ("provenance-tree-mismatch", lambda state: replace(state, provenance_tree=old_tree), False),
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
    return len(mutations) + _run_live_applied_rules_cases()


def self_test() -> tuple[int, int]:
    return _run_static_cases(), _run_merge_cases()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check the Windows workflow quality oracle")
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--self-test", action="store_true", help="run baseline, mutation, and merge fixtures")
    modes.add_argument(
        "--validate-live-applied-rules",
        action="store_true",
        help="validate applied-rules JSON from stdin",
    )
    args = parser.parse_args(argv)
    if args.validate_live_applied_rules:
        try:
            raw = sys.stdin.read()
        except (OSError, UnicodeDecodeError):
            print("workflow-quality-gate: FAIL live-applied-rules input", file=sys.stderr)
            return 1
        if not validate_live_applied_rules_json(raw):
            print("workflow-quality-gate: FAIL live-applied-rules", file=sys.stderr)
            return 1
        print("workflow-quality-gate: PASS live-applied-rules")
        return 0
    if not args.self_test:
        parser.error("one validation mode is required")
    static_cases, merge_cases = self_test()
    if static_cases <= 0 or merge_cases <= 0:
        raise AssertionError("fixture case counts must be positive")
    print(f"workflow-quality-gate: PASS static_cases={static_cases} merge_cases={merge_cases}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

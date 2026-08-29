#!/usr/bin/env python3
"""Finite oracle for the trusted version-preparation workflow.

The production shell is the subject under test.  This fixture extracts the
named workflow step and executes that exact block with a deliberately small
fake GitHub CLI.  It also checks the source's trust and mutation boundaries,
including a finite set of one-edit mutations.

The external-mutation oracle is intentionally bounded: each run snapshots
the repository's status, diffs, and tracked-file content hashes before and
after execution, while temporary files and ``GITHUB_OUTPUT`` live outside the
repository.  It is not a whole-filesystem watcher.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
import textwrap
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "version-prepare.yml"
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "windows-client.yml"
EXPECTED_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "windows-client/Directory.Build.props",
)
_ALLOWED_RUNNER_TEMP_FILES = frozenset(
    f"version-prepare/{directory}/{path}"
    for directory in ("base", "head", "next")
    for path in EXPECTED_PATHS
)
REPOSITORY = "owner/repository"
BASE_SHA = "a" * 40
HEAD_SHA = "b" * 40
HEAD_TREE_SHA = "c" * 40
BLOB_SHAS = ("1" * 40, "2" * 40, "3" * 40)
TREE_SHA = "4" * 40
COMMIT_SHA = "5" * 40
SECRET = "fixture-secret-must-not-be-logged"


class FixtureError(RuntimeError):
    """A fixture assertion or source-contract failure."""


def fail(message: str) -> None:
    raise FixtureError(message)


def _extract_named_run_block(source: str, marker: str) -> str:
    """Extract and dedent one named production step's literal shell block."""

    lines = source.splitlines()
    marker_indexes = [index for index, line in enumerate(lines) if line == marker]
    if len(marker_indexes) != 1:
        fail(f"production step marker must occur exactly once: {marker}")
    marker_index = marker_indexes[0]
    run_indexes = [
        index
        for index in range(marker_index + 1, min(len(lines), marker_index + 20))
        if lines[index] == "        run: |"
    ]
    if len(run_indexes) != 1:
        fail(f"production step must have one literal run block: {marker}")

    body: list[str] = []
    for line in lines[run_indexes[0] + 1 :]:
        if line.strip() == "":
            body.append("")
            continue
        if not line.startswith("          "):
            break
        body.append(line[10:])
    while body and body[-1] == "":
        body.pop()
    if not body or not body[0].startswith("set -euo pipefail"):
        fail(f"production run block is empty or not shell code: {marker}")
    return "\n".join(body) + "\n"


def extract_run_block(source: str) -> str:
    """Extract the version-mutation shell that the runtime fixture executes."""

    return _extract_named_run_block(
        source, "      - name: Validate and prepare version data"
    )


def _between(source: str, start: str, end: str) -> str:
    start_index = source.find(start)
    if start_index < 0:
        fail(f"missing source section: {start}")
    end_index = source.find(end, start_index + len(start))
    if end_index < 0:
        fail(f"missing source section terminator: {end}")
    return source[start_index:end_index]


def _require(source: str, needle: str, label: str) -> None:
    if needle not in source:
        fail(f"static requirement failed: {label}")


def _replace_once(source: str, old: str, new: str, label: str) -> str:
    count = source.count(old)
    if count != 1:
        fail(f"mutation target {label!r} occurred {count} times, expected 1")
    return source.replace(old, new, 1)


def _release_job_permissions(source: str) -> tuple[str, ...]:
    """Extract the release job's small, explicit permission mapping."""

    marker = "  release:\n"
    marker_indexes = [index for index, line in enumerate(source.splitlines()) if line == marker.rstrip("\n")]
    if len(marker_indexes) != 1:
        fail("windows release job marker must occur exactly once")
    lines = source.splitlines()
    start = marker_indexes[0]
    end = next(
        (index for index in range(start + 1, len(lines)) if re.match(r"^  [A-Za-z0-9_.-]+:", lines[index])),
        len(lines),
    )
    job = lines[start:end]
    permission_indexes = [index for index, line in enumerate(job) if line == "    permissions:"]
    if len(permission_indexes) != 1:
        fail("windows release job must have one permissions mapping")
    permission_start = permission_indexes[0]
    permissions: list[str] = []
    for line in job[permission_start + 1 :]:
        if line and not line.startswith("      "):
            break
        if line.startswith("      "):
            permissions.append(line.strip())
    return tuple(permissions)


def validate_release_permissions(source: str) -> None:
    """Keep the release PR lookup at the least privilege required by GitHub."""

    permissions = _release_job_permissions(source)
    expected = ("contents: write", "actions: read", "pull-requests: read")
    if permissions != expected:
        fail(f"release job permissions must be exactly {expected}, found {permissions}")
    if "pull-requests: write" in permissions or "issues: write" in permissions:
        fail("release job must not grant PR or issue write access")


_ALLOWED_SHELL_COMMANDS = frozenset(
    {
        "base64",
        "cp",
        "echo",
        "exit",
        "fetch_version_file",
        "gh",
        "grep",
        "jq",
        "local",
        "mkdir",
        "printf",
        "python3",
        "return",
        "sed",
        "set",
        "tr",
        "create_blob",
    }
)
_SHELL_KEYWORDS = frozenset(
    {
        "case",
        "do",
        "done",
        "elif",
        "else",
        "esac",
        "fi",
        "if",
        "in",
        "then",
        "until",
        "while",
    }
)
_INTERPRETER_NAMES = frozenset(
    {
        "bash",
        "bun",
        "dash",
        "deno",
        "lua",
        "node",
        "nodejs",
        "perl",
        "php",
        "pypy",
        "python",
        "python3",
        "ruby",
        "sh",
        "zsh",
    }
)
_EXPECTED_FETCH_CALLS = (
    'fetch_version_file "$REPOSITORY" "$BASE_SHA" Cargo.toml "$base/Cargo.toml"',
    'fetch_version_file "$REPOSITORY" "$BASE_SHA" Cargo.lock "$base/Cargo.lock"',
    'fetch_version_file "$REPOSITORY" "$BASE_SHA" windows-client/Directory.Build.props "$base/windows-client/Directory.Build.props"',
    'fetch_version_file "$HEAD_REPOSITORY" "$HEAD_SHA" Cargo.toml "$head/Cargo.toml"',
    'fetch_version_file "$HEAD_REPOSITORY" "$HEAD_SHA" Cargo.lock "$head/Cargo.lock"',
    'fetch_version_file "$HEAD_REPOSITORY" "$HEAD_SHA" windows-client/Directory.Build.props "$head/windows-client/Directory.Build.props"',
)
_EXPECTED_VERSION_TOOL_PATHS = (
    "$base/Cargo.toml",
    "$base/Cargo.lock",
    "$base/windows-client/Directory.Build.props",
    "$head/Cargo.toml",
    "$head/Cargo.lock",
    "$head/windows-client/Directory.Build.props",
    "$next/Cargo.toml",
    "$next/Cargo.lock",
    "$next/windows-client/Directory.Build.props",
    "$next/Cargo.toml",
    "$next/Cargo.lock",
    "$next/windows-client/Directory.Build.props",
)


def _shell_structure(block: str) -> str:
    """Mask quoted separators/comments while retaining shell command structure."""

    normalized = re.sub(r"\\\r?\n[ \t]*", " ", block)
    characters: list[str] = []
    quote: str | None = None
    comment = False
    escaped = False
    for character in normalized:
        if comment:
            if character == "\n":
                comment = False
                characters.append(character)
            else:
                characters.append(" ")
            continue
        if quote is not None:
            if escaped:
                escaped = False
                characters.append(" ") if character in ";|&" else characters.append(character)
            elif character == "\\":
                escaped = True
                characters.append(character)
            elif character == quote:
                quote = None
                characters.append(character)
            elif character in ";|&":
                characters.append(" ")
            else:
                characters.append(character)
            continue
        if character == "#":
            comment = True
            characters.append(" ")
        elif character in "'\"":
            quote = character
            characters.append(character)
        else:
            characters.append(character)
    return "".join(characters)


def _shell_command_tokens(block: str) -> list[str]:
    """Return command-position tokens from the small production shell subset."""

    normalized = _shell_structure(block)
    command_pattern = re.compile(
        r"(?:^|[;&|]\s*|\$\(\s*)"
        r"(?P<command>(?:[A-Za-z_][A-Za-z0-9_.-]*|"
        r"(?:[A-Za-z_][A-Za-z0-9_.-]*/)+[^\s;&|()]+|"
        r"(?:\./|\.\./|/|~/)[^\s;&|()]+))"
    )
    return [match.group("command") for match in command_pattern.finditer(normalized)]


def _validate_command_allowlist(block: str) -> None:
    """Keep the extracted workflow to the commands its contract actually needs."""

    for command in _shell_command_tokens(block):
        if command in _SHELL_KEYWORDS:
            continue
        if command in _INTERPRETER_NAMES and command != "python3":
            fail(f"production run block invokes an unapproved interpreter: {command}")
        if command not in _ALLOWED_SHELL_COMMANDS:
            fail(f"production run block invokes an unapproved command: {command}")

    normalized = _shell_structure(block)
    for match in re.finditer(r"\bpython3\b", normalized):
        suffix = normalized[match.end() :]
        if not re.match(
            r"[ \t]+scripts/product_version\.py[ \t]+(?:check|next|bump)(?=[ \t|;&]|$)",
            suffix,
        ):
            fail("python3 is restricted to the trusted product_version.py commands")

    # A command-position path is an executable/script invocation, not a data path.
    if re.search(
        r"(?:^|[;&|]\s*|\$\(\s*)(?:\./|\.\./|/|~/|[A-Za-z_][A-Za-z0-9_.-]*/)",
        normalized,
    ):
        fail("production run block invokes an executable by path")


def _validate_external_effect_boundary(block: str) -> None:
    """Deny unbounded commands and writes in the production run block."""

    _validate_command_allowlist(block)

    denied_patterns = (
        r"\b(?:curl|wget|nc|ncat|socat|ssh|scp|sftp|ftp|telnet)\b",
        r"\bgit\s+(?:push|commit|reset|clean|checkout|write-tree|update-ref)\b",
        r"\b(?:touch|rm|mv|install|chmod|chown|ln|tee|dd)\b",
    )
    for pattern in denied_patterns:
        if re.search(pattern, block):
            fail(f"production run block contains denied external command: {pattern}")

    normalized = re.sub(r"\\\r?\n[ \t]*", " ", block)
    if re.search(r"\bsed\b[^|;&\n]*\s(?:-i|--in-place)(?:\s|$)", normalized):
        fail("sed in the production run block must not edit files in place")
    if re.search(r"\b(?:base64|jq)\b[^|;&\n]*\s(?:-o|--output|--outfile)(?:\s|=)", normalized):
        fail("file-producing base64/jq output options are not allowed")

    expected_assignments = (
        'work="$RUNNER_TEMP/version-prepare"',
        'base="$work/base"',
        'head="$work/head"',
        'next="$work/next"',
    )
    for assignment in expected_assignments:
        if block.count(assignment) != 1:
            fail(f"temporary path assignment is not unique and fixed: {assignment}")
    if block.count('local output="$4"') != 1:
        fail("Contents API output must remain the fourth bounded function argument")
    if block.count('> "$output"') != 1 or block.count('>> "$GITHUB_OUTPUT"') != 1:
        fail("file writes must use the single decoded output and GitHub output paths")

    expected_copy_commands = (
        'cp "$head/Cargo.toml" "$next/Cargo.toml"',
        'cp "$head/Cargo.lock" "$next/Cargo.lock"',
        'cp "$head/windows-client/Directory.Build.props" "$next/windows-client/Directory.Build.props"',
    )
    for copy_command in expected_copy_commands:
        if block.count(copy_command) != 1:
            fail(f"copy destination escaped the bounded temporary trees: {copy_command}")
    if len(re.findall(r"(?m)^\s*cp\s+", block)) != len(expected_copy_commands):
        fail("copy count or destination changed")
    expected_mkdir = 'mkdir -p "$base/windows-client" "$head/windows-client" "$next/windows-client"'
    if block.count(expected_mkdir) != 1 or len(re.findall(r"(?m)^\s*mkdir\s+", block)) != 1:
        fail("mkdir destination escaped the bounded temporary trees")

    for call in _EXPECTED_FETCH_CALLS:
        if block.count(call) != 1:
            fail(f"Contents API fetch destination escaped its bounded tree: {call}")
    if len(re.findall(r"(?m)^fetch_version_file\s+", block)) != len(_EXPECTED_FETCH_CALLS):
        fail("Contents API fetch count or call shape changed")

    path_arguments = re.findall(
        r"--(?:cargo-toml|cargo-lock|windows-props)\s+(?:\"([^\"]+)\"|([^\s|;&]+))",
        normalized,
    )
    actual_paths = [quoted or bare for quoted, bare in path_arguments]
    if sorted(actual_paths) != sorted(_EXPECTED_VERSION_TOOL_PATHS):
        fail("version authority file paths escaped the bounded temporary trees")

    for line in block.splitlines():
        stripped = line.strip()
        if re.match(r"cp\s", stripped) and not re.fullmatch(
            r'cp "\$head/[^\"]+" "\$next/[^\"]+"', stripped
        ):
            fail("copy destination escaped the temporary head/next trees")
        if re.match(r"mkdir\s", stripped) and not re.fullmatch(
            r'mkdir -p "\$(?:base|head|next)/[^\"]+"(?: "\$(?:base|head|next)/[^\"]+")*',
            stripped,
        ):
            fail("mkdir destination escaped the temporary version trees")

        # Here-strings (<<<) feed jq data and are not file writes.
        for redirection in re.finditer(
            r'(?<!<)(?:>>?|&>)(?:\s*)("[^"]*"|\'[^\']*\'|[^\s;&|]+)', line
        ):
            target = redirection.group(1)
            if target.startswith("&"):
                continue
            if target not in {'"$output"', '"$GITHUB_OUTPUT"'}:
                fail(f"run-block redirection escaped bounded output paths: {target}")

    _require(block, 'work="$RUNNER_TEMP/version-prepare"', "temporary work root")


def _validate_read_only_scope_block(block: str) -> None:
    """Keep a write-token scope classifier finite and mutation-free."""

    expected_markers = (
        '[[ "$REPOSITORY" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ &&',
        '"$BASE_REPOSITORY" == "$REPOSITORY" &&',
        '"$HEAD_REPOSITORY" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ &&',
        '"$PR_NUMBER" =~ ^[1-9][0-9]*$ &&',
        '"$BASE_SHA" =~ ^[0-9a-fA-F]{40}$ &&',
        '"$HEAD_SHA" =~ ^[0-9a-fA-F]{40}$ ]] || {',
        'scope_root="$RUNNER_TEMP/ci-change-scope"',
        'mkdir -p "$scope_root"',
        'gh api --method GET "repos/$REPOSITORY/pulls/$PR_NUMBER" > "$scope_root/pull-request.json"',
        'gh api --method GET --paginate --slurp \\\n  "repos/$REPOSITORY/pulls/$PR_NUMBER/files?per_page=100" > "$scope_root/files.json"',
        'scope="$(python3 scripts/ci_change_scope.py \\\n',
        '--pull-request "$scope_root/pull-request.json"',
        '--files "$scope_root/files.json"',
        '--expected-repository "$REPOSITORY"',
        '--expected-head-repository "$HEAD_REPOSITORY"',
        '--expected-number "$PR_NUMBER"',
        '--expected-base-sha "$BASE_SHA"',
        '--expected-head-sha "$HEAD_SHA"',
        'case "$scope" in',
        'binary-impact) binary_impact=true ;;',
        'no-binary-impact) binary_impact=false ;;',
        "*) echo 'change-scope classifier returned an invalid result' >&2; exit 1 ;;",
        'esac',
        "printf 'binary_impact=%s\\n' \"$binary_impact\" >> \"$GITHUB_OUTPUT\"",
    )
    for marker in expected_markers:
        if block.count(marker) != 1:
            fail(f"scope classifier marker must occur exactly once: {marker}")
    if len(re.findall(r"(?m)^gh api\s", block)) != 2:
        fail("scope classification must make exactly two GitHub API reads")
    if block.count("python3") != 1 or block.count("$(") != 1 or "`" in block:
        fail("scope classification gained an additional Python invocation")
    if len(re.findall(r"(?m)^mkdir\s", block)) != 1:
        fail("scope classification mkdir boundary changed")
    if re.search(r"--method\s+(?!GET(?:\s|$))", block):
        fail("scope classification GitHub API method is not read-only")
    if re.search(
        r"\b(?:POST|PATCH|PUT|DELETE|git|curl|wget|ssh|scp|bash|sh|node|ruby|perl|"
        r"rm|mv|cp|tee|dd|chmod|chown|ln|eval|source)\b",
        block,
    ):
        fail("scope classification contains an unapproved command or mutation")
    expected_command_counts = {
        "set ": 1,
        "echo ": 2,
        "exit ": 2,
        "mkdir ": 1,
        "gh api ": 2,
        "python3 ": 1,
        "printf ": 1,
    }
    for command, expected in expected_command_counts.items():
        if block.count(command) != expected:
            fail(f"scope classification command count changed: {command}")
    identity_failure_lines = (
        "echo 'pull request scope identity is malformed' >&2",
        "echo 'merged pull request scope identity is malformed' >&2",
    )
    if sum(block.count(line) for line in identity_failure_lines) != 1:
        fail("scope identity failure marker changed")
    allowed_starts = (
        "set -euo pipefail",
        "[[ ",
        '"$BASE_REPOSITORY" ',
        '"$HEAD_REPOSITORY" ',
        '"$PR_NUMBER" ',
        '"$BASE_SHA" ',
        '"$HEAD_SHA" ',
        "echo '",
        "exit 1",
        "}",
        "scope_root=",
        "mkdir -p ",
        "gh api --method GET ",
        '"repos/$REPOSITORY/pulls/',
        'scope="$(python3 scripts/ci_change_scope.py ',
        "--pull-request ",
        "--files ",
        "--expected-repository ",
        "--expected-head-repository ",
        "--expected-number ",
        "--expected-base-sha ",
        "--expected-head-sha ",
        'case "$scope" in',
        "binary-impact) binary_impact=true ;;",
        "no-binary-impact) binary_impact=false ;;",
        "*) echo '",
        "esac",
        "printf 'binary_impact=%s\\n' ",
    )
    for line in block.splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith(allowed_starts):
            fail(f"scope classification line is outside the finite grammar: {stripped}")
    for redirection in re.finditer(
        r'(?<!<)(?:>>?|&>)(?:\s*)("[^"]*"|\'[^\']*\'|[^\s;&|]+)', block
    ):
        target = redirection.group(1)
        if target.startswith("&"):
            continue
        if target not in {
            '"$scope_root/pull-request.json"',
            '"$scope_root/files.json"',
            '"$GITHUB_OUTPUT"',
        }:
            fail(f"scope classification write escaped bounded paths: {target}")


def _validate_scope_contract(source: str) -> None:
    """Require a read-only trusted classifier before the write-capable step."""

    checkout_marker = "      - name: Checkout trusted default branch"
    scope_marker = "      - name: Classify pull request scope"
    validation_marker = "      - name: Validate and prepare version data"
    for marker in (checkout_marker, scope_marker, validation_marker):
        if source.count(marker) != 1:
            fail(f"trusted step marker must occur exactly once: {marker}")
    if not source.index(checkout_marker) < source.index(scope_marker) < source.index(validation_marker):
        fail("scope classification must run after trusted checkout and before mutation")

    scope = _between(source, scope_marker, validation_marker)
    block = _extract_named_run_block(source, scope_marker)
    if re.search(r"(?m)^        (?:if|continue-on-error):", scope):
        fail("version scope classifier must run and propagate every failure")
    for needle, label in (
        ("        id: scope\n", "scope output id"),
        ("        shell: bash\n", "scope shell"),
        ("          GH_TOKEN: ${{ github.token }}\n", "step-local GitHub token"),
        ("          REPOSITORY: ${{ github.repository }}\n", "repository identity"),
        ("          PR_NUMBER: ${{ github.event.pull_request.number }}\n", "PR identity"),
        ("          BASE_REPOSITORY: ${{ github.event.pull_request.base.repo.full_name }}\n", "base repository identity"),
        ("          HEAD_REPOSITORY: ${{ github.event.pull_request.head.repo.full_name }}\n", "head repository identity"),
        ("          BASE_SHA: ${{ github.event.pull_request.base.sha }}\n", "base SHA identity"),
        ("          HEAD_SHA: ${{ github.event.pull_request.head.sha }}\n", "head SHA identity"),
    ):
        if scope.count(needle) != 1:
            fail(f"scope contract changed: {label}")
    _validate_read_only_scope_block(block)
    if source.count("        if: steps.scope.outputs.binary_impact == 'true'\n") != 1:
        fail("version mutation step must be guarded by binary impact")


def validate_release_scope(source: str) -> None:
    """Require release classification to finish read-only on trusted main."""

    job_marker = "  release:\n"
    scope_marker = "      - name: Classify merged pull request scope"
    mutation_marker = "      - name: Resolve accepted PR quality run"
    if source.count(job_marker) != 1 or source.count(scope_marker) != 1:
        fail("release scope job or step marker is not unique")
    if source.count(mutation_marker) != 1:
        fail("first release mutation step marker is not unique")
    release = source[source.index(job_marker) :]
    checkout_marker = "      - uses: actions/checkout@v4"
    if release.count(checkout_marker) != 1:
        fail("release must have one trusted checkout")
    if not release.index(checkout_marker) < release.index(scope_marker) < release.index(mutation_marker):
        fail("release classification must precede every artifact or release operation")
    checkout = release[release.index(checkout_marker) : release.index(scope_marker)]
    if checkout.count("          ref: refs/heads/main\n") != 1:
        fail("release classifier checkout must resolve trusted main")

    scope = _between(source, scope_marker, mutation_marker)
    if re.search(r"(?m)^        (?:if|continue-on-error):", scope):
        fail("release scope classifier must run and propagate every failure")
    for needle, label in (
        ("        id: scope\n", "scope output id"),
        ("        shell: bash\n", "scope shell"),
        ("          GH_TOKEN: ${{ github.token }}\n", "step-local GitHub token"),
        ("          REPOSITORY: ${{ github.repository }}\n", "repository identity"),
        ("          PR_NUMBER: ${{ github.event.pull_request.number }}\n", "PR identity"),
        ("          BASE_REPOSITORY: ${{ github.event.pull_request.base.repo.full_name }}\n", "base repository identity"),
        ("          HEAD_REPOSITORY: ${{ github.event.pull_request.head.repo.full_name }}\n", "head repository identity"),
        ("          BASE_SHA: ${{ github.event.pull_request.base.sha }}\n", "base SHA identity"),
        ("          HEAD_SHA: ${{ github.event.pull_request.head.sha }}\n", "head SHA identity"),
    ):
        if scope.count(needle) != 1:
            fail(f"release scope contract changed: {label}")
    _validate_read_only_scope_block(_extract_named_run_block(source, scope_marker))


def validate_static(source: str) -> None:
    """Validate the finite trust boundary without parsing untrusted YAML."""

    block = extract_run_block(source)
    _validate_external_effect_boundary(block)
    _validate_scope_contract(source)
    trigger = _between(source, "on:\n", "permissions:\n")
    _require(trigger, "  pull_request_target:\n", "pull_request_target trigger")
    _require(trigger, '    branches: ["main"]\n', "main branch trigger")
    _require(
        trigger,
        "    types: [opened, synchronize, reopened, ready_for_review]\n",
        "pull request event types",
    )

    if len(re.findall(r"(?m)^permissions:\s*\{\}\s*$", source)) != 1:
        fail("top-level permissions must be exactly {}")
    permissions = re.search(
        r"(?ms)^    permissions:\n(?P<body>(?:^      [^\n]*\n?)+)", source
    )
    expected_permissions = "      contents: write\n      pull-requests: read\n"
    if permissions is None or permissions.group("body") != expected_permissions:
        fail("prepare job permissions must be contents write and pull requests read only")
    if re.search(r"(?m)^\s+contents:\s+write\s*$", source) is None:
        fail("prepare job contents write permission is missing")
    if re.search(r"(?m)^\s+(?:checks|pull-requests|issues):\s+write\s*$", source):
        fail("PR/checks write permissions are not allowed")

    checkout = _between(
        source,
        "      - name: Checkout trusted default branch",
        "      - name: Classify pull request scope",
    )
    for needle, label in (
        ("        uses: actions/checkout@v4\n", "trusted checkout action"),
        ("          repository: ${{ github.repository }}\n", "trusted repository"),
        ("          ref: refs/heads/main\n", "trusted main ref"),
        ("          persist-credentials: false\n", "checkout credential persistence"),
    ):
        _require(checkout, needle, label)
    if source.count("actions/checkout@") != 1:
        fail("workflow must not add a PR-head checkout")
    if re.search(r"(?m)^\s*ref:\s*\$\{\{\s*github\.event\.pull_request\.head", source):
        fail("trusted checkout must not resolve the PR head")
    if re.search(r"(?m)^\s*checks:\s*write\s*$", source) or re.search(
        r"\bgit\s+push\b", source
    ):
        fail("PR checkout/checks write or git push is not allowed")

    _require(
        source,
        "  group: version-prepare-${{ github.event.pull_request.number }}\n",
        "version prepare concurrency group",
    )
    _require(source, "  cancel-in-progress: false\n", "non-cancelling concurrency")

    # The shell must not make actor, title, body, or message an authority.
    if re.search(r"(?:github\.actor|GITHUB_ACTOR|pull_request\.(?:title|body|message))", source):
        fail("actor or PR message is used as version authority")

    # Fail closed before any Git Data API mutation.
    early = block.find('if [[ "$head_version" == "$next_version" ]]; then')
    first_post = block.find("--method POST")
    if early < 0 or first_post < 0 or early > first_post:
        fail("already-next early exit must precede all writes")
    observed = block.find("observed_head=\"$(gh api")
    observed_check = block.find('[[ "$observed_head" == "$HEAD_SHA" ]]', observed)
    blob = block.find('"repos/$REPOSITORY/git/blobs"')
    if observed < 0 or observed_check < 0 or blob < 0 or observed_check > blob:
        fail("head race check must precede blob creation")
    if block.count('[[ "$observed_head" == "$HEAD_SHA" ]]') != 1:
        fail("head race check must occur exactly once")

    commit_payload = block.find("commit_payload=")
    commit_proof = block.find("commit_info=", commit_payload + 1)
    final_patch = block.find("--method PATCH")
    if commit_payload < 0 or commit_proof < 0 or final_patch < 0 or not commit_proof < final_patch:
        fail("commit parent/tree proof must precede final ref mutation")
    _require(block, '  --arg parent "$HEAD_SHA" \\\n', "commit parent is PR head")
    if block.count('  --arg parent "$HEAD_SHA" \\\n') != 1:
        fail("commit parent binding must occur exactly once")
    _require(block, "  -F force=false \\\n", "non-force ref update")
    if "-F force=true" in block or block.count("-F force=false") != 1:
        fail("final ref update must use force=false exactly once")

    tree_start = block.find("tree_payload=")
    tree_end = block.find("tree_sha=", tree_start + 1)
    if tree_start < 0 or tree_end < 0:
        fail("tree payload is missing")
    tree_payload = block[tree_start:tree_end]
    paths = tuple(re.findall(r'\{path:"([^"]+)",mode:"100644",type:"blob",sha:\$[A-Za-z0-9_]+\}', tree_payload))
    if paths != EXPECTED_PATHS:
        fail(f"tree overlay must contain exactly {EXPECTED_PATHS}, found {paths}")
    _require(tree_payload, "--arg base \"$head_tree_sha\"", "head tree overlay base")

    _require(block, 'get_ref_endpoint="repos/$REPOSITORY/git/ref/heads/$HEAD_REF"', "head ref observation")
    _require(block, 'update_ref_endpoint="repos/$REPOSITORY/git/refs/heads/$HEAD_REF"', "PR head ref update")
    if block.count("git/refs/heads/$HEAD_REF") != 1:
        fail("only the PR head ref may be updated")
    if block.count('"repos/$REPOSITORY/git/blobs"') != 1:
        fail("blob endpoint must be singular")
    if block.count('"repos/$REPOSITORY/git/trees"') != 1:
        fail("tree endpoint must be singular")
    if (
        block.count('"repos/$REPOSITORY/git/commits"') != 1
        or block.count('"repos/$REPOSITORY/git/commits/$HEAD_SHA"') != 1
        or block.count('"repos/$REPOSITORY/git/commits/$commit_sha"') != 1
    ):
        fail("head-tree lookup, commit create, and proof endpoints must be present")

    _require(block, '[[ "$BASE_REPOSITORY" == "$REPOSITORY" ]]', "base repository guard")
    _require(block, '[[ "$HEAD_REPOSITORY" != "$REPOSITORY" ]]', "same-repository mutation guard")
    if block.count('[[ "$HEAD_REPOSITORY" != "$REPOSITORY" ]]') != 1:
        fail("same-repository mutation guard must occur exactly once")

    blob_start = block.find("create_blob() {\n")
    blob_end = block.find("\ncargo_toml_blob=", blob_start)
    if blob_start < 0 or blob_end < 0:
        fail("blob creation function is missing")
    blob_block = block[blob_start:blob_end]
    _require(
        blob_block,
        'base64 --wrap=0 "$path" |\n'
        '    jq -Rs \'{content:.,encoding:"base64"}\' |\n'
        '    gh api \\\n',
        "blob payload is JSON on stdin",
    )
    if blob_block.count("--input -") != 1:
        fail("blob creation must read exactly one stdin request")
    if re.search(r"(?<![A-Za-z0-9_])(?:-f|-F|--field|--raw-field)(?:[=\s])", blob_block):
        fail("blob content must not be passed through a gh argv field")


@dataclass(frozen=True)
class Scenario:
    name: str
    base_repository: str = REPOSITORY
    head_repository: str = REPOSITORY
    repository: str = REPOSITORY
    base_sha: str = BASE_SHA
    head_sha: str = HEAD_SHA
    head_ref: str = "feature/version"
    base_version: str = "1.0.8"
    head_version: str = "1.0.8"
    observed_head: str = HEAD_SHA
    conflict: bool = False
    content_mode: str = "valid"
    expect_success: bool = False
    expect_already_next: bool = False
    expect_independent_baseline_rejection: bool = False


@dataclass(frozen=True)
class RepoSnapshot:
    status: bytes
    diff: bytes
    cached_diff: bytes
    tracked_hashes: tuple[tuple[str, str], ...]
    external_hashes: tuple[tuple[str, str], ...] = ()


def _bounded_file_hashes(root: Path) -> tuple[tuple[str, str], ...]:
    """Capture files below one explicitly bounded non-repository root."""

    if not root.exists():
        return ()
    entries: list[tuple[str, str]] = []
    for path in sorted(root.rglob("*")):
        if path.is_dir() and not path.is_symlink():
            continue
        relative = path.relative_to(root).as_posix()
        if path.is_symlink():
            data = b"symlink:" + os.fsencode(os.readlink(path))
        elif path.is_file():
            data = path.read_bytes()
        else:
            data = b"<unsupported>"
        entries.append((relative, hashlib.sha256(data).hexdigest()))
    return tuple(entries)


def _repo_snapshot(
    extra_roots: tuple[tuple[str, Path], ...] = (),
) -> RepoSnapshot:
    """Capture repository state plus explicitly bounded mutation-probe roots."""

    def git(*arguments: str) -> bytes:
        completed = subprocess.run(
            ["git", *arguments],
            cwd=ROOT,
            check=True,
            capture_output=True,
        )
        return completed.stdout

    status = git("status", "--porcelain=v1", "--untracked-files=all")
    diff = git("diff", "--binary", "--no-ext-diff")
    cached_diff = git("diff", "--cached", "--binary", "--no-ext-diff")
    tracked = git("ls-files", "-z")
    tracked_hashes: list[tuple[str, str]] = []
    for raw_path in tracked.split(b"\0"):
        if not raw_path:
            continue
        relative = os.fsdecode(raw_path)
        path = ROOT / relative
        if path.is_symlink():
            data = b"symlink:" + os.fsencode(os.readlink(path))
        elif path.is_file():
            data = path.read_bytes()
        else:
            data = b"<missing>"
        tracked_hashes.append((relative, hashlib.sha256(data).hexdigest()))
    external_hashes = tuple(
        (f"{label}:{relative}", digest)
        for label, root in extra_roots
        for relative, digest in _bounded_file_hashes(root)
    )
    return RepoSnapshot(
        status,
        diff,
        cached_diff,
        tuple(tracked_hashes),
        external_hashes,
    )


@dataclass
class RunResult:
    scenario: Scenario
    returncode: int
    stdout: str
    stderr: str
    stdout_bytes: bytes
    stderr_bytes: bytes
    transcript: list[dict[str, Any]]
    transcript_bytes: bytes
    output_file: str
    output_bytes: bytes
    blob_digests: list[str]
    content_fixtures: dict[str, bytes]


def _version_files(version: str) -> dict[str, bytes]:
    return {
        "Cargo.toml": (
            '[package]\nname = "codex_info"\nversion = "' + version + '"\nedition = "2021"\n'
        ).encode(),
        "Cargo.lock": (
            'version = 3\n\n[[package]]\nname = "codex_info"\nversion = "' + version + '"\n'
        ).encode(),
        "windows-client/Directory.Build.props": (
            '<Project><PropertyGroup><Version>' + version + "</Version></PropertyGroup></Project>\n"
        ).encode(),
    }


_INDEPENDENT_BASELINE_VERSION = "1.0.8"
_INDEPENDENT_BASELINE_NEXT_VERSION = "1.0.9"
_INDEPENDENT_BASELINE_FILES = {
    "Cargo.toml": b'[package]\nname = "codex_info"\nversion = "1.0.8"\nedition = "2021"\n',
    "Cargo.lock": b'version = 3\n\n[[package]]\nname = "codex_info"\nversion = "1.0.8"\n',
    "windows-client/Directory.Build.props": b"<Project><PropertyGroup><Version>1.0.8</Version></PropertyGroup></Project>\n",
}
_INDEPENDENT_BASELINE_SHA256 = {
    "Cargo.toml": "c552f81cf29303f0dc43622e4c254c95f18247748141fee454d508b20c222001",
    "Cargo.lock": "6130afadadfc3b92f58cf73bda7bd3bdd251bd62cbc0312b3a367b39a1bb5010",
    "windows-client/Directory.Build.props": "3de8c30e1c2544845f6214e0cef0d46a1ec1cdd8f243601e3c847f935c4fdbee",
}


def _independent_baseline_files() -> dict[str, bytes]:
    """Return fixed expected head bytes whose source is not the Contents API."""

    for path in EXPECTED_PATHS:
        data = _INDEPENDENT_BASELINE_FILES.get(path)
        expected_hash = _INDEPENDENT_BASELINE_SHA256.get(path)
        if data is None or expected_hash is None:
            fail(f"independent baseline is missing {path}")
        if hashlib.sha256(data).hexdigest() != expected_hash:
            fail(f"independent baseline hash changed for {path}")
    return dict(_INDEPENDENT_BASELINE_FILES)


def _write_fake_gh(directory: Path) -> Path:
    """Create the fake executable used by the extracted production block."""

    fake = directory / "gh"
    fake.write_text(
        textwrap.dedent(
            r'''
            #!/usr/bin/env python3
            import base64
            import hashlib
            import json
            import os
            import sys

            def die(message):
                print(message, file=sys.stderr)
                raise SystemExit(1)

            config_path = os.environ.get("CI_TRUST_FAKE_CONFIG")
            if not config_path:
                die("fake gh config is missing")
            with open(config_path, encoding="utf-8") as handle:
                config = json.load(handle)
            transcript_path = config["transcript"]
            with open(transcript_path, encoding="utf-8") as handle:
                transcript = json.load(handle)
            args = sys.argv[1:]
            if not args or args[0] != "api":
                die("fake gh only supports api")
            method = "GET"
            endpoint = None
            jq = None
            fields = {}
            input_mode = False
            index = 1
            while index < len(args):
                arg = args[index]
                if arg == "--method":
                    method = args[index + 1]
                    index += 2
                    continue
                if arg in ("-f", "-F"):
                    value = args[index + 1]
                    if "=" not in value:
                        die("malformed field")
                    key, field_value = value.split("=", 1)
                    fields[key] = field_value
                    index += 2
                    continue
                if arg == "--jq":
                    jq = args[index + 1]
                    index += 2
                    continue
                if arg == "--input":
                    if index + 1 >= len(args) or args[index + 1] != "-":
                        die("--input must read stdin")
                    input_mode = True
                    index += 2
                    continue
                if arg in ("-H", "--hostname"):
                    index += 2
                    continue
                if arg.startswith("-"):
                    index += 1
                    continue
                if endpoint is None:
                    endpoint = arg
                index += 1
            if endpoint is None:
                die("endpoint is missing")

            entry = {"method": method, "endpoint": endpoint}
            if fields:
                safe_fields = {key: value for key, value in fields.items() if key != "content"}
                if "content" in fields:
                    try:
                        decoded = base64.b64decode(fields["content"], validate=True)
                    except Exception:
                        die("blob content was not base64")
                    safe_fields["content_sha256"] = hashlib.sha256(decoded).hexdigest()
                entry["fields"] = safe_fields

            payload = None
            if input_mode:
                try:
                    payload = json.load(sys.stdin)
                except Exception:
                    die("request input was not JSON")
                entry["payload"] = payload
            transcript.append(entry)
            with open(transcript_path, "w", encoding="utf-8") as handle:
                json.dump(transcript, handle, separators=(",", ":"))

            if method == "GET" and "/contents/" in endpoint:
                if config.get("content_mode") == "malformed":
                    print("not-json")
                    raise SystemExit(0)
                try:
                    path_part, ref = endpoint.split("?ref=", 1)
                    pieces = path_part.split("/")
                    if len(pieces) < 5 or pieces[0] != "repos" or pieces[3] != "contents":
                        raise ValueError
                    repository = pieces[1] + "/" + pieces[2]
                    path = "/".join(pieces[4:])
                except ValueError:
                    die("malformed contents endpoint")
                key = repository + "|" + ref + "|" + path
                response = config["contents"].get(key)
                if response is None:
                    die("unexpected contents request")
                print(json.dumps(response, separators=(",", ":")))
                raise SystemExit(0)

            if method == "GET" and endpoint.endswith("/git/ref/heads/" + config["head_ref"]):
                response = {"object": {"sha": config["observed_head"]}}
                if jq == ".object.sha":
                    print(config["observed_head"])
                else:
                    print(json.dumps(response, separators=(",", ":")))
                raise SystemExit(0)
            if method == "GET" and "/git/commits/" in endpoint:
                commit_id = endpoint.rsplit("/", 1)[-1]
                if commit_id == config["head_sha"]:
                    response = {"tree": {"sha": config["head_tree_sha"]}}
                elif commit_id == config["commit_sha"]:
                    response = {
                        "tree": {"sha": config["tree_sha"]},
                        "parents": [{"sha": config["head_sha"]}],
                    }
                else:
                    die("unexpected commit lookup")
                if jq == ".tree.sha":
                    print(response["tree"]["sha"])
                else:
                    print(json.dumps(response, separators=(",", ":")))
                raise SystemExit(0)

            if method == "POST" and endpoint.endswith("/git/blobs"):
                index = len([item for item in transcript if item["method"] == "POST" and item["endpoint"].endswith("/git/blobs")]) - 1
                if index >= len(config["blob_shas"]):
                    die("too many blob writes")
                if fields:
                    die("blob request must not pass fields through argv")
                if not input_mode:
                    die("blob request must use stdin JSON")
                if (
                    not isinstance(payload, dict)
                    or set(payload) != {"content", "encoding"}
                    or payload.get("encoding") != "base64"
                    or not isinstance(payload.get("content"), str)
                ):
                    die("blob request must use exact base64 JSON payload")
                content_b64 = payload["content"]
                try:
                    decoded = base64.b64decode(content_b64, validate=True)
                except Exception:
                    die("blob stdin content was not base64")
                response = {"sha": config["blob_shas"][index]}
                transcript[-1]["content_b64"] = content_b64
                transcript[-1]["content_sha256"] = hashlib.sha256(decoded).hexdigest()
                transcript[-1]["response_sha"] = response["sha"]
                with open(transcript_path, "w", encoding="utf-8") as handle:
                    json.dump(transcript, handle, separators=(",", ":"))
                print(response["sha"] if jq == ".sha" else json.dumps(response))
                raise SystemExit(0)
            if method == "POST" and endpoint.endswith("/git/trees"):
                print(config["tree_sha"] if jq == ".sha" else json.dumps({"sha": config["tree_sha"]}))
                raise SystemExit(0)
            if method == "POST" and endpoint.endswith("/git/commits"):
                print(config["commit_sha"] if jq == ".sha" else json.dumps({"sha": config["commit_sha"]}))
                raise SystemExit(0)
            if method == "PATCH" and endpoint.endswith("/git/refs/heads/" + config["head_ref"]):
                if config.get("conflict"):
                    transcript[-1]["outcome"] = "conflict"
                    with open(transcript_path, "w", encoding="utf-8") as handle:
                        json.dump(transcript, handle, separators=(",", ":"))
                    die("reference update conflict")
                response = {"object": {"sha": config["commit_sha"]}}
                print(config["commit_sha"] if jq == ".object.sha" else json.dumps(response))
                raise SystemExit(0)
            die("unexpected fake GitHub API request")
            ''',
        ).lstrip("\\\n"),
        encoding="utf-8",
    )
    fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
    return fake


def _content_fixtures(scenario: Scenario) -> dict[str, bytes]:
    base_files = _version_files(scenario.base_version)
    head_files = _version_files(scenario.head_version)
    if scenario.content_mode == "cross-file-mismatch":
        head_files["Cargo.lock"] = _version_files(scenario.base_version.replace("8", "7"))["Cargo.lock"]
    fixtures: dict[str, bytes] = {}
    for repository, ref, files in (
        (scenario.repository, scenario.base_sha, base_files),
        (scenario.head_repository, scenario.head_sha, head_files),
    ):
        for path, value in files.items():
            fixtures[repository + "|" + ref + "|" + path] = value
    return fixtures


def _contents_config(scenario: Scenario, fixtures: dict[str, bytes]) -> dict[str, Any]:
    contents: dict[str, Any] = {}
    for key, value in fixtures.items():
        path = key.rsplit("|", 1)[-1]
        response: dict[str, Any] = {
            "type": "file",
            "encoding": "base64",
            "content": base64.b64encode(value).decode("ascii"),
        }
        if scenario.content_mode == "missing" and path == "Cargo.lock":
            response.pop("content")
        elif scenario.content_mode == "malformed-file" and path == "Cargo.lock":
            response["type"] = "directory"
        contents[key] = response
    return contents


def _fixture_derived_expected_next_files(
    scenario: Scenario, fixtures: dict[str, bytes]
) -> dict[str, bytes]:
    """Build the intentionally colluding expectation from API fixture bytes."""

    expected: dict[str, bytes] = {}
    old = scenario.base_version.encode("ascii")
    major, minor, patch = scenario.base_version.split(".")
    new = f"{major}.{minor}.{int(patch) + 1}".encode("ascii")
    for path in EXPECTED_PATHS:
        key = scenario.head_repository + "|" + scenario.head_sha + "|" + path
        source = fixtures[key]
        if source.count(old) != 1:
            fail(f"{scenario.name}: expected one source version occurrence in {path}")
        expected[path] = source.replace(old, new, 1)
    return expected


def _literal_expected_next_files(
    scenario: Scenario, fixtures: dict[str, bytes]
) -> dict[str, bytes]:
    """Build expected upload bytes from a fixed literal/hash baseline only."""

    del fixtures
    baseline = _independent_baseline_files()
    old = _INDEPENDENT_BASELINE_VERSION.encode("ascii")
    new = _INDEPENDENT_BASELINE_NEXT_VERSION.encode("ascii")
    expected: dict[str, bytes] = {}
    for path in EXPECTED_PATHS:
        source = baseline[path]
        if source.count(old) != 1:
            fail(f"{scenario.name}: independent baseline lacks one version occurrence in {path}")
        expected[path] = source.replace(old, new, 1)
    return expected


def run_case(block: str, scenario: Scenario) -> RunResult:
    with tempfile.TemporaryDirectory(prefix="ci-trust-fixture-") as temporary:
        temporary_path = Path(temporary)
        fake_bin = temporary_path / "bin"
        fake_bin.mkdir()
        _write_fake_gh(fake_bin)
        transcript_path = temporary_path / "transcript.json"
        transcript_path.write_text("[]", encoding="utf-8")
        config_path = temporary_path / "config.json"
        content_fixtures = _content_fixtures(scenario)
        config = {
            "transcript": str(transcript_path),
            "contents": _contents_config(scenario, content_fixtures),
            "content_mode": scenario.content_mode,
            "observed_head": scenario.observed_head,
            "head_ref": scenario.head_ref,
            "head_sha": scenario.head_sha,
            "head_tree_sha": HEAD_TREE_SHA,
            "blob_shas": list(BLOB_SHAS),
            "tree_sha": TREE_SHA,
            "commit_sha": COMMIT_SHA,
            "conflict": scenario.conflict,
        }
        config_path.write_text(json.dumps(config), encoding="utf-8")
        output_path = temporary_path / "github-output"
        block_path = temporary_path / "run.sh"
        block_path.write_text(block, encoding="utf-8")
        runner_temp = temporary_path / "runner"
        runner_temp.mkdir()
        environment = os.environ.copy()
        environment.update(
            {
                "PATH": str(fake_bin) + os.pathsep + environment.get("PATH", ""),
                "CI_TRUST_FAKE_CONFIG": str(config_path),
                "GH_TOKEN": SECRET,
                "REPOSITORY": scenario.repository,
                "BASE_REPOSITORY": scenario.base_repository,
                "HEAD_REPOSITORY": scenario.head_repository,
                "BASE_SHA": scenario.base_sha,
                "HEAD_SHA": scenario.head_sha,
                "HEAD_REF": scenario.head_ref,
                "RUNNER_TEMP": str(runner_temp),
                "GITHUB_OUTPUT": str(output_path),
                "GITHUB_ACTOR": "untrusted-bot",
            }
        )
        before_snapshot = _repo_snapshot()
        before_runner_snapshot = _repo_snapshot((("runner-temp", runner_temp),))
        completed = subprocess.run(
            ["bash", str(block_path)],
            cwd=ROOT,
            env=environment,
            capture_output=True,
            timeout=30,
        )
        after_snapshot = _repo_snapshot()
        after_runner_snapshot = _repo_snapshot((("runner-temp", runner_temp),))
        if after_snapshot != before_snapshot:
            fail(f"{scenario.name}: production run mutated bounded repository state")
        before_runner_files = dict(before_runner_snapshot.external_hashes)
        after_runner_files = dict(after_runner_snapshot.external_hashes)
        runner_changes = {
            key
            for key in set(before_runner_files) | set(after_runner_files)
            if before_runner_files.get(key) != after_runner_files.get(key)
        }
        allowed_runner_changes = {
            f"runner-temp:{relative}" for relative in _ALLOWED_RUNNER_TEMP_FILES
        }
        unexpected_runner_changes = runner_changes - allowed_runner_changes
        if unexpected_runner_changes:
            fail(
                f"{scenario.name}: production run escaped bounded runner temp paths: "
                f"{sorted(unexpected_runner_changes)!r}"
            )
        transcript_bytes = transcript_path.read_bytes()
        transcript = json.loads(transcript_bytes)
        output_bytes = output_path.read_bytes() if output_path.exists() else b""
        output = output_bytes.decode("utf-8", errors="replace")
        stdout_bytes = completed.stdout
        stderr_bytes = completed.stderr
        stdout = stdout_bytes.decode("utf-8", errors="replace")
        stderr = stderr_bytes.decode("utf-8", errors="replace")
        next_dir = runner_temp / "version-prepare" / "next"
        blob_digests = []
        if next_dir.exists():
            for path in EXPECTED_PATHS:
                file_path = next_dir / path
                if file_path.exists():
                    blob_digests.append(hashlib.sha256(file_path.read_bytes()).hexdigest())
        return RunResult(
            scenario=scenario,
            returncode=completed.returncode,
            stdout=stdout,
            stderr=stderr,
            stdout_bytes=stdout_bytes,
            stderr_bytes=stderr_bytes,
            transcript=transcript,
            transcript_bytes=transcript_bytes,
            output_file=output,
            output_bytes=output_bytes,
            blob_digests=blob_digests,
            content_fixtures=content_fixtures,
        )


def _writes(result: RunResult) -> list[dict[str, Any]]:
    return [
        entry
        for entry in result.transcript
        if entry.get("method") in ("POST", "PATCH")
    ]


def _assert_no_writes(result: RunResult) -> None:
    if _writes(result):
        fail(f"{result.scenario.name}: unexpected writes: {_writes(result)!r}")


def _assert_failure(result: RunResult) -> None:
    if result.returncode == 0:
        fail(f"{result.scenario.name}: expected failure")


def _assert_no_secret(result: RunResult) -> None:
    secret = SECRET.encode("utf-8")
    artifacts = (
        ("GITHUB_OUTPUT", result.output_bytes),
        ("stdout", result.stdout_bytes),
        ("stderr", result.stderr_bytes),
        ("fake-gh transcript", result.transcript_bytes),
    )
    for label, data in artifacts:
        if secret in data:
            fail(f"{result.scenario.name}: GH_TOKEN value leaked in {label}")


def _assert_contents_provenance(result: RunResult) -> None:
    scenario = result.scenario
    observed = [
        entry["endpoint"]
        for entry in result.transcript
        if entry.get("method") == "GET" and "/contents/" in entry.get("endpoint", "")
    ]
    base_expected = [
        f"repos/{scenario.repository}/contents/{path}?ref={scenario.base_sha}"
        for path in EXPECTED_PATHS
    ]
    head_expected = [
        f"repos/{scenario.head_repository}/contents/{path}?ref={scenario.head_sha}"
        for path in EXPECTED_PATHS
    ]
    allowed = set(base_expected + head_expected)
    if any(endpoint not in allowed for endpoint in observed):
        fail(f"{scenario.name}: Contents API endpoint escaped provenance set: {observed!r}")

    full_fetch_expected = (
        scenario.content_mode in ("valid", "cross-file-mismatch", "poisoned-baseline")
        and scenario.base_repository == scenario.repository
        and re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", scenario.repository)
        and re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", scenario.head_repository)
        and re.fullmatch(r"[0-9a-fA-F]{40}", scenario.base_sha)
        and re.fullmatch(r"[0-9a-fA-F]{40}", scenario.head_sha)
        and not scenario.head_ref.startswith("/")
        and ".." not in scenario.head_ref
        and "?" not in scenario.head_ref
        and "#" not in scenario.head_ref
    )
    if full_fetch_expected and observed != base_expected + head_expected:
        fail(f"{scenario.name}: expected exact base/head Contents API fetches, found {observed!r}")


def _assert_blob_payloads(
    result: RunResult, expected_files: dict[str, bytes], label: str
) -> None:
    blob_entries = [
        entry
        for entry in _writes(result)
        if entry["method"] == "POST" and entry["endpoint"].endswith("/git/blobs")
    ]
    if len(blob_entries) != len(EXPECTED_PATHS):
        fail(f"{result.scenario.name}: {label} expected three blob payloads")
    for path, blob_entry in zip(EXPECTED_PATHS, blob_entries):
        payload = blob_entry.get("payload")
        if (
            not isinstance(payload, dict)
            or set(payload) != {"content", "encoding"}
            or payload.get("encoding") != "base64"
            or not isinstance(payload.get("content"), str)
        ):
            fail(f"{result.scenario.name}: {label} blob transcript lacks exact stdin payload for {path}")
        if payload["content"] != blob_entry.get("content_b64"):
            fail(f"{result.scenario.name}: {label} blob transcript content mismatch for {path}")
        try:
            actual_bytes = base64.b64decode(payload["content"], validate=True)
        except (KeyError, ValueError):
            fail(f"{result.scenario.name}: {label} blob transcript lacks valid content for {path}")
        if actual_bytes != expected_files[path]:
            fail(f"{result.scenario.name}: {label} bytes differ for {path}")
        if blob_entry.get("content_sha256") != hashlib.sha256(actual_bytes).hexdigest():
            fail(f"{result.scenario.name}: {label} blob transcript digest mismatch for {path}")


def check_runtime(block: str) -> int:
    cases = [
        Scenario("normal", expect_success=True),
        Scenario(
            "already-next-reentry",
            head_repository="contributor/repository",
            head_version="1.0.9",
            expect_success=True,
            expect_already_next=True,
        ),
        Scenario("base-repository-mismatch", base_repository="other/repository"),
        Scenario("fork-head-repository", head_repository="contributor/repository"),
        Scenario("malformed-base-sha", base_sha="a" * 39),
        Scenario("malformed-repository", repository="owner/bad repo"),
        Scenario("malformed-head-ref", head_ref="/unsafe"),
        Scenario("head-race", observed_head="d" * 40),
        Scenario("final-force-false-conflict", conflict=True),
        Scenario("contents-malformed-json", content_mode="malformed"),
        Scenario("contents-missing-field", content_mode="missing"),
        Scenario("contents-cross-file-mismatch", content_mode="cross-file-mismatch"),
        Scenario(
            "contents-poisoned-independent-baseline",
            base_version="1.0.7",
            head_version="1.0.7",
            content_mode="poisoned-baseline",
            expect_success=True,
            expect_independent_baseline_rejection=True,
        ),
    ]
    for scenario in cases:
        result = run_case(block, scenario)
        _assert_no_secret(result)
        _assert_contents_provenance(result)
        if scenario.expect_success:
            if result.returncode != 0:
                fail(f"{scenario.name}: unexpected failure: {result.stderr}")
            if scenario.expect_independent_baseline_rejection:
                colluding_expected = _fixture_derived_expected_next_files(
                    scenario, result.content_fixtures
                )
                independent_expected = _literal_expected_next_files(
                    scenario, result.content_fixtures
                )
                if colluding_expected == independent_expected:
                    fail(f"{scenario.name}: poisoned fixture did not diverge from baseline")
                _assert_blob_payloads(result, colluding_expected, "fixture-derived")
                try:
                    _assert_blob_payloads(result, independent_expected, "independent baseline")
                except FixtureError:
                    pass
                else:
                    fail(f"{scenario.name}: poisoned Contents API escaped independent baseline")
                continue
            if scenario.expect_already_next:
                _assert_no_writes(result)
                if result.output_file != "check_conclusion=success\nversion=1.0.9\n":
                    fail(f"{scenario.name}: unexpected output {result.output_file!r}")
                continue
            writes = _writes(result)
            if len([entry for entry in writes if entry["method"] == "POST" and entry["endpoint"].endswith("/git/blobs")]) != 3:
                fail("normal: expected three blob writes")
            tree_writes = [entry for entry in writes if entry["endpoint"].endswith("/git/trees")]
            commit_writes = [entry for entry in writes if entry["endpoint"].endswith("/git/commits")]
            ref_writes = [entry for entry in writes if entry["method"] == "PATCH"]
            if len(tree_writes) != 1 or len(commit_writes) != 1 or len(ref_writes) != 1:
                fail(f"normal: unexpected write transcript {writes!r}")
            tree = tree_writes[0].get("payload")
            if not isinstance(tree, dict) or tree.get("base_tree") != HEAD_TREE_SHA:
                fail("normal: tree base was not the observed PR head tree")
            tree_entries = tree.get("tree")
            if not isinstance(tree_entries, list) or tuple(item.get("path") for item in tree_entries) != EXPECTED_PATHS:
                fail(f"normal: tree overlay paths are not exact: {tree_entries!r}")
            expected_tree_entries = [
                {"path": path, "mode": "100644", "type": "blob", "sha": sha}
                for path, sha in zip(EXPECTED_PATHS, BLOB_SHAS)
            ]
            if tree_entries != expected_tree_entries:
                fail(f"normal: tree overlay differs: {tree_entries!r}")
            blob_entries = [entry for entry in writes if entry["endpoint"].endswith("/git/blobs")]
            expected_files = _literal_expected_next_files(scenario, result.content_fixtures)
            for path, blob_entry, blob_sha in zip(EXPECTED_PATHS, blob_entries, BLOB_SHAS):
                if blob_entry.get("response_sha") != blob_sha:
                    fail(f"normal: blob response SHA mismatch for {path}")
                try:
                    actual_bytes = base64.b64decode(blob_entry["content_b64"], validate=True)
                except (KeyError, ValueError):
                    fail(f"normal: blob transcript lacks valid content for {path}")
                if actual_bytes != expected_files[path]:
                    fail(f"normal: uploaded bytes differ from literal expected transform for {path}")
            response_shas = {entry.get("response_sha") for entry in blob_entries}
            if any(item.get("sha") not in response_shas for item in tree_entries):
                fail("normal: tree path-to-blob mapping was not backed by blob responses")
            commit = commit_writes[0].get("payload")
            if commit != {
                "message": "chore: prepare version 1.0.9",
                "tree": TREE_SHA,
                "parents": [HEAD_SHA],
            }:
                fail(f"normal: commit payload differs: {commit!r}")
            patch = ref_writes[0]
            if patch["endpoint"] != "repos/owner/repository/git/refs/heads/feature/version":
                fail(f"normal: wrong ref endpoint: {patch!r}")
            if patch.get("fields") != {"sha": COMMIT_SHA, "force": "false"}:
                fail(f"normal: ref update was not strict: {patch!r}")
            if result.output_file:
                fail(f"normal: unexpected output {result.output_file!r}")
            if [entry.get("content_sha256") for entry in blob_entries] != result.blob_digests:
                fail("normal: blob transcript does not match prepared files")
        elif scenario.name == "final-force-false-conflict":
            _assert_failure(result)
            writes = _writes(result)
            if len([entry for entry in writes if entry["method"] == "POST" and entry["endpoint"].endswith("/git/blobs")]) != 3:
                fail("conflict: expected three orphanable blob writes")
            if len([entry for entry in writes if entry["endpoint"].endswith("/git/trees")]) != 1:
                fail("conflict: expected one tree write")
            if len([entry for entry in writes if entry["endpoint"].endswith("/git/commits")]) != 1:
                fail("conflict: expected one commit write")
            patches = [entry for entry in writes if entry["method"] == "PATCH"]
            if len(patches) != 1 or patches[0].get("outcome") != "conflict":
                fail(f"conflict: ref retry or success was observed: {patches!r}")
            if patches[0].get("fields", {}).get("force") != "false":
                fail("conflict: final update was not force=false")
        else:
            _assert_failure(result)
            _assert_no_writes(result)
    return len(cases)


def check_mutations(source: str) -> int:
    work_assignment = '          work="$RUNNER_TEMP/version-prepare"\n'
    fork_guard = (
        '          if [[ "$HEAD_REPOSITORY" != "$REPOSITORY" ]]; then\n'
        "            echo 'automatic version preparation requires a same-repository PR' >&2\n"
        "            exit 1\n"
        "          fi\n"
    )
    race_guard = (
        '          [[ "$observed_head" == "$HEAD_SHA" ]] || {\n'
        '            echo "PR head moved before version preparation: expected $HEAD_SHA, found $observed_head" >&2\n'
        "            exit 1\n"
        "          }\n"
    )
    early_exit = (
        '          if [[ "$head_version" == "$next_version" ]]; then\n'
        "            printf 'check_conclusion=success\\nversion=%s\\n' \"$head_version\" >> \"$GITHUB_OUTPUT\"\n"
        "            exit 0\n"
        "          fi\n"
    )
    tree_line = '              {path:"windows-client/Directory.Build.props",mode:"100644",type:"blob",sha:$windows_props_sha}\n'
    blob_stdin = (
        '          create_blob() {\n'
        '            local path="$1"\n'
        '            base64 --wrap=0 "$path" |\n'
        '              jq -Rs \'{content:.,encoding:"base64"}\' |\n'
        '              gh api \\\n'
        '                --method POST \\\n'
        "                -H 'Accept: application/vnd.github+json' \\\n"
        "                -H 'X-GitHub-Api-Version: 2022-11-28' \\\n"
        '                "repos/$REPOSITORY/git/blobs" \\\n'
        '                --input - --jq \'.sha\'\n'
        '          }\n'
    )
    blob_argv = (
        '          create_blob() {\n'
        '            local path="$1"\n'
        '            local encoded\n'
        '            encoded="$(base64 --wrap=0 "$path")"\n'
        '            gh api \\\n'
        '              --method POST \\\n'
        "              -H 'Accept: application/vnd.github+json' \\\n"
        "              -H 'X-GitHub-Api-Version: 2022-11-28' \\\n"
        '              "repos/$REPOSITORY/git/blobs" \\\n'
        '              -f "content=$encoded" \\\n'
        '              -f encoding=base64 \\\n'
        "              --jq '.sha'\n"
        '          }\n'
    )
    mutations = [
        ("trusted-checkout-pr-head", "          ref: refs/heads/main\n", "          ref: ${{ github.event.pull_request.head.sha }}\n"),
        (
            "scope-permission-missing",
            "      pull-requests: read\n",
            "",
        ),
        (
            "scope-read-became-write",
            '          gh api --method GET "repos/$REPOSITORY/pulls/$PR_NUMBER" > "$scope_root/pull-request.json"\n',
            '          gh api --method POST "repos/$REPOSITORY/pulls/$PR_NUMBER" > "$scope_root/pull-request.json"\n',
        ),
        (
            "scope-classifier-from-other-path",
            '          scope="$(python3 scripts/ci_change_scope.py \\\n',
            '          scope="$(python3 scripts/product_version.py \\\n',
        ),
        (
            "scope-binary-impact-guard-deleted",
            "        if: steps.scope.outputs.binary_impact == 'true'\n",
            "",
        ),
        (
            "scope-no-binary-impact-became-binary-impact",
            "            no-binary-impact) binary_impact=false ;;\n",
            "            no-binary-impact) binary_impact=true ;;\n",
        ),
        (
            "scope-failure-ignored",
            "        id: scope\n        shell: bash\n",
            "        id: scope\n        continue-on-error: true\n        shell: bash\n",
        ),
        ("force-true", "            -F force=false \\\n", "            -F force=true \\\n"),
        ("fork-guard-deleted", fork_guard, ""),
        ("head-race-guard-deleted", race_guard, ""),
        (
            "fourth-tree-path",
            tree_line,
            tree_line + '              {path:"README.md",mode:"100644",type:"blob",sha:$windows_props_sha},\n',
        ),
        (
            "parent-base",
            '            --arg parent "$HEAD_SHA" \\\n',
            '            --arg parent "$BASE_SHA" \\\n',
        ),
        ("early-next-exit-deleted", early_exit, ""),
        (
            "python-inline-write",
            work_assignment,
            work_assignment
            + "          python3 -c 'from pathlib import Path; Path(\"Cargo.toml\").write_text(\"poison\")'\n",
        ),
        (
            "python-script-write",
            work_assignment,
            work_assignment + "          python3 /tmp/ci-trust-fixture.py\n",
        ),
        (
            "runner-temp-write",
            work_assignment,
            work_assignment + '          printf probe > "$RUNNER_TEMP/version-prepare/probe"\n',
        ),
        (
            "repository-write",
            work_assignment,
            work_assignment + '          printf probe > "$GITHUB_WORKSPACE/probe"\n',
        ),
        ("blob-content-argv", blob_stdin, blob_argv),
    ]
    for name, old, new in mutations:
        mutated = _replace_once(source, old, new, name)
        try:
            validate_static(mutated)
        except FixtureError:
            continue
        fail(f"mutation escaped static oracle: {name}")
    return len(mutations)


def check_release_permission_mutations(source: str) -> int:
    """Exercise the finite release-job permission contract."""

    expected = (
        "    permissions:\n"
        "      contents: write\n"
        "      actions: read\n"
        "      pull-requests: read\n"
    )
    mutations = (
        (
            "release-pull-requests-missing",
            expected,
            expected.replace("      pull-requests: read\n", ""),
        ),
        (
            "release-pull-requests-write",
            expected,
            expected.replace("pull-requests: read", "pull-requests: write"),
        ),
        (
            "release-permission-excess",
            expected,
            expected.replace(
                "      actions: read\n",
                "      actions: read\n      packages: read\n",
            ),
        ),
    )
    for name, old, new in mutations:
        mutated = _replace_once(source, old, new, name)
        try:
            validate_release_permissions(mutated)
        except FixtureError:
            continue
        fail(f"release permission mutation escaped static oracle: {name}")
    return len(mutations)


def check_release_scope_mutations(source: str) -> int:
    """Exercise the finite trusted-main release-classifier boundary."""

    scope_marker = "      - name: Classify merged pull request scope"
    mutation_marker = "      - name: Resolve accepted PR quality run"

    def mutate_scope(old: str, new: str, label: str) -> str:
        start = source.index(scope_marker)
        end = source.index(mutation_marker, start)
        scope = source[start:end]
        return source[:start] + _replace_once(scope, old, new, label) + source[end:]

    cases = (
        (
            "release-scope-read-became-write",
            '          gh api --method GET "repos/$REPOSITORY/pulls/$PR_NUMBER" > "$scope_root/pull-request.json"\n',
            '          gh api --method POST "repos/$REPOSITORY/pulls/$PR_NUMBER" > "$scope_root/pull-request.json"\n',
        ),
        (
            "release-scope-classifier-changed",
            '          scope="$(python3 scripts/ci_change_scope.py \\\n',
            '          scope="$(python3 scripts/product_version.py \\\n',
        ),
        (
            "release-scope-no-binary-impact-became-binary-impact",
            "            no-binary-impact) binary_impact=false ;;\n",
            "            no-binary-impact) binary_impact=true ;;\n",
        ),
        (
            "release-scope-failure-ignored",
            "        id: scope\n        env:\n",
            "        id: scope\n        continue-on-error: true\n        env:\n",
        ),
    )
    for name, old, new in cases:
        mutated = mutate_scope(old, new, name)
        try:
            validate_release_scope(mutated)
        except FixtureError:
            continue
        fail(f"release scope mutation escaped static oracle: {name}")

    release_start = source.index("  release:\n")
    checkout_end = source.index(scope_marker, release_start)
    checkout = source[release_start:checkout_end]
    mutated_checkout = _replace_once(
        checkout,
        "          ref: refs/heads/main\n",
        "          ref: ${{ github.event.pull_request.head.sha }}\n",
        "release-scope-pr-checkout",
    )
    mutated = source[:release_start] + mutated_checkout + source[checkout_end:]
    try:
        validate_release_scope(mutated)
    except FixtureError:
        pass
    else:
        fail("release scope PR checkout mutation escaped static oracle")
    return len(cases) + 1


def self_test() -> tuple[int, int]:
    try:
        source = WORKFLOW.read_text(encoding="utf-8")
        validate_static(source)
        block = extract_run_block(source)
        runtime_cases = check_runtime(block)
        mutation_cases = check_mutations(source)
        release_source = RELEASE_WORKFLOW.read_text(encoding="utf-8")
        validate_release_permissions(release_source)
        validate_release_scope(release_source)
        mutation_cases += check_release_permission_mutations(release_source)
        mutation_cases += check_release_scope_mutations(release_source)
        if runtime_cases <= 0 or mutation_cases <= 0:
            fail("case counts must be positive")
        return runtime_cases, mutation_cases
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError, FixtureError) as error:
        raise FixtureError(str(error)) from error


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    arguments = parser.parse_args(argv)
    if not arguments.self_test:
        parser.error("--self-test is required")
    try:
        runtime_cases, mutation_cases = self_test()
    except FixtureError as error:
        print(f"ci-trust-fixture: FAIL {error}", file=sys.stderr)
        return 1
    print(f"ci-trust-fixture: PASS runtime_cases={runtime_cases} mutation_cases={mutation_cases}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

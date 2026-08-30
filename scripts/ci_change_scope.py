#!/usr/bin/env python3
"""Fail-closed pull-request path classifier for selective CI ownership."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import re
import sys
from typing import Any, Sequence


OWNER_ORDER = ("DOCS", "GOVERNANCE", "LINUX_BACKEND", "LINUX_UI", "WINDOWS")
PRODUCT_OWNERS = frozenset({"LINUX_BACKEND", "LINUX_UI", "WINDOWS"})
KNOWN_FILE_STATUSES = frozenset(
    {"added", "removed", "modified", "renamed", "copied", "changed", "unchanged"}
)
FULL_SHA = re.compile(r"^[0-9a-fA-F]{40}$")
REPOSITORY = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
ALLOWED_BASE_REFS = frozenset({"main", "feat/next"})

DOC_EXACT = frozenset({"README.md", "README.en.md", "DESIGN.md", "SECURITY.md"})
GOVERNANCE_EXACT = frozenset(
    {
        ".gitignore",
        "AGENTS.md",
        "scripts/ci_change_scope.py",
        "scripts/ci_trust_fixture.py",
        "scripts/feat_integration_check_reporter.py",
        "scripts/final_acceptance_gate.sh",
        "scripts/final_acceptance_gate_test.sh",
        "scripts/final_head_check_reporter.py",
        "scripts/pre_pr_gate.sh",
        "scripts/product_version.py",
        "scripts/quality_artifact_gate.sh",
        "scripts/release_candidate_gate.sh",
        "scripts/release_candidate_gate_test.sh",
        "scripts/release_quality_run_resolver.py",
        "scripts/release_state_gate.py",
        "scripts/regression_guard.sh",
        "scripts/requirements_ledger_gate.sh",
        "scripts/selected_quality_gate.py",
        "scripts/test_ci_change_scope.py",
        "scripts/test_codeql_workflow.py",
        "scripts/test_feat_integration_check_reporter.py",
        "scripts/test_final_head_check_reporter.py",
        "scripts/test_product_version.py",
        "scripts/test_release_quality_run_resolver.py",
        "scripts/test_selected_quality_gate.py",
        "scripts/test_version_prepare_workflow.py",
        "scripts/windows_client_contract_gate.sh",
        "scripts/workflow_quality_gate.py",
    }
)
WINDOWS_EXACT = frozenset(
    {
        "scripts/capture_windows_window.ps1",
        "scripts/windows_window_move_message_smoke.ps1",
        "scripts/windows_window_move_smoke.ps1",
    }
)
LINUX_BACKEND_EXACT = frozenset(
    {
        "run.sh",
        "scripts/cli_contract_e2e.sh",
        "scripts/data_protection_gate.sh",
        "scripts/db_protection_e2e.sh",
        "scripts/install_systemd_recorder.sh",
        "scripts/record_daemon_e2e.sh",
    }
)
LINUX_UI_EXACT = frozenset(
    {"scripts/x11_graph_visual_gate.sh", "scripts/x11_startup_visual_gate.sh"}
)
LINUX_SHARED_EXACT = frozenset(
    {"Cargo.toml", "Cargo.lock", "build.rs", "deny.toml", "src/main.rs"}
)
LEGAL_SHARED_EXACT = frozenset(
    {"COPYRIGHT", "LICENSE", "LICENSE.ja.md", "THIRD_PARTY_NOTICES.md"}
)


class ScopeError(ValueError):
    """Raised when GitHub data cannot prove a complete, bound classification."""


@dataclass(frozen=True)
class Selection:
    owners: tuple[str, ...]
    codeql_languages: tuple[str, ...]

    @property
    def binary_impact(self) -> bool:
        return bool(PRODUCT_OWNERS.intersection(self.owners))

    def legacy_scope(self) -> str:
        return "binary-impact" if self.binary_impact else "no-binary-impact"

    def as_json(self) -> str:
        return json.dumps(
            {
                "binary_impact": self.binary_impact,
                "owners": list(self.owners),
                "codeql_languages": list(self.codeql_languages),
            },
            separators=(",", ":"),
            sort_keys=True,
        )


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ScopeError("duplicate JSON object key")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_reject_duplicate_keys)
    except ScopeError:
        raise
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ScopeError("JSON input is unreadable or malformed") from exc


def _mapping(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ScopeError(f"{label} is not an object")
    return value


def _string(mapping: dict[str, Any], key: str, label: str) -> str:
    value = mapping.get(key)
    if not isinstance(value, str) or not value:
        raise ScopeError(f"{label} is missing or malformed")
    return value


def _validate_path(value: Any) -> str:
    if not isinstance(value, str) or not value or "\x00" in value or value.startswith("/"):
        raise ScopeError("changed file path is malformed")
    if any(part in {"", ".", ".."} for part in value.split("/")):
        raise ScopeError("changed file path is not normalized")
    return value


def owners_for_path(path: str) -> frozenset[str]:
    """Return the finite owner set for one normalized repository path."""
    path = _validate_path(path)
    if path in DOC_EXACT or path.startswith(("docs/", "wiki/")):
        return frozenset({"DOCS"})
    if path in GOVERNANCE_EXACT or path.startswith((".github/", ".vscode/", ".codex-tasks/")):
        return frozenset({"GOVERNANCE"})
    if path in WINDOWS_EXACT or path.startswith("windows-client/"):
        return frozenset({"WINDOWS"})
    if path in LINUX_BACKEND_EXACT or path.startswith(("tests/", "packaging/")):
        return frozenset({"LINUX_BACKEND"})
    if path.startswith("src/") and path != "src/main.rs":
        return frozenset({"LINUX_BACKEND"})
    if path in LINUX_UI_EXACT or path.startswith(("ui/", "assets/")):
        return frozenset({"LINUX_UI"})
    if path in LINUX_SHARED_EXACT or path.startswith(".cargo/"):
        return frozenset({"LINUX_BACKEND", "LINUX_UI"})
    if path.startswith("protocol/"):
        return frozenset({"LINUX_BACKEND", "WINDOWS"})
    if path in LEGAL_SHARED_EXACT or path.startswith("LICENSES/"):
        return frozenset({"LINUX_BACKEND", "LINUX_UI", "WINDOWS"})
    raise ScopeError(f"changed path has no CI owner: {path}")


def selection_for_paths(paths: Sequence[str]) -> Selection:
    owners: set[str] = set()
    for path in paths:
        owners.update(owners_for_path(path))
    if not owners:
        raise ScopeError("pull request contains no classifiable paths")
    languages: set[str] = set()
    if "GOVERNANCE" in owners:
        languages.update(("actions", "python"))
    if "WINDOWS" in owners:
        languages.add("csharp")
    if {"LINUX_BACKEND", "LINUX_UI"}.intersection(owners):
        languages.add("rust")
    return Selection(
        owners=tuple(owner for owner in OWNER_ORDER if owner in owners),
        codeql_languages=tuple(
            language
            for language in ("actions", "csharp", "python", "rust")
            if language in languages
        ),
    )


def _validate_identity(
    pull_request: Any,
    *,
    expected_repository: str,
    expected_head_repository: str,
    expected_number: int,
    expected_base_ref: str,
    expected_head_ref: str | None,
    expected_base_sha: str,
    expected_head_sha: str,
    expected_state: str | None,
) -> int:
    if not REPOSITORY.fullmatch(expected_repository) or not REPOSITORY.fullmatch(expected_head_repository):
        raise ScopeError("expected repository is malformed")
    if type(expected_number) is not int or expected_number <= 0:
        raise ScopeError("expected pull request number is malformed")
    if expected_base_ref not in ALLOWED_BASE_REFS:
        raise ScopeError("expected base ref is not an allowed integration branch")
    if not FULL_SHA.fullmatch(expected_base_sha) or not FULL_SHA.fullmatch(expected_head_sha):
        raise ScopeError("expected pull request SHA is malformed")

    root = _mapping(pull_request, "pull request")
    if root.get("number") != expected_number:
        raise ScopeError("pull request number mismatch")
    state = root.get("state")
    if state not in {"open", "closed"} or (expected_state is not None and state != expected_state):
        raise ScopeError("pull request state mismatch")
    base = _mapping(root.get("base"), "pull request base")
    head = _mapping(root.get("head"), "pull request head")
    if _string(_mapping(base.get("repo"), "base repository"), "full_name", "base repository") != expected_repository:
        raise ScopeError("pull request base repository mismatch")
    if _string(base, "ref", "base ref") != expected_base_ref:
        raise ScopeError("pull request base ref mismatch")
    if _string(base, "sha", "base SHA") != expected_base_sha:
        raise ScopeError("pull request base SHA mismatch")
    if _string(_mapping(head.get("repo"), "head repository"), "full_name", "head repository") != expected_head_repository:
        raise ScopeError("pull request head repository mismatch")
    if expected_head_ref is not None and _string(head, "ref", "head ref") != expected_head_ref:
        raise ScopeError("pull request head ref mismatch")
    if _string(head, "sha", "head SHA") != expected_head_sha:
        raise ScopeError("pull request head SHA mismatch")
    changed_files = root.get("changed_files")
    if type(changed_files) is not int or changed_files <= 0:
        raise ScopeError("pull request changed_files is missing or empty")
    return changed_files


def _changed_paths(files_pages: Any, expected_count: int) -> tuple[str, ...]:
    if not isinstance(files_pages, list) or not files_pages:
        raise ScopeError("pull request file pagination is missing")
    paths: list[str] = []
    current_names: set[str] = set()
    records = 0
    for page in files_pages:
        if not isinstance(page, list):
            raise ScopeError("pull request file page is malformed")
        for raw_file in page:
            file_info = _mapping(raw_file, "pull request file")
            records += 1
            filename = _validate_path(file_info.get("filename"))
            if filename in current_names:
                raise ScopeError("pull request file pagination contains a duplicate")
            current_names.add(filename)
            status = file_info.get("status")
            if not isinstance(status, str) or status not in KNOWN_FILE_STATUSES:
                raise ScopeError("pull request file status is malformed")
            paths.append(filename)
            previous = file_info.get("previous_filename")
            if status in {"renamed", "copied"} and previous is None:
                raise ScopeError("rename or copy is missing its previous path")
            if status not in {"renamed", "copied"} and previous is not None:
                raise ScopeError("non-rename file unexpectedly contains a previous path")
            if previous is not None:
                paths.append(_validate_path(previous))
    if records != expected_count:
        raise ScopeError("pull request file pagination is incomplete")
    return tuple(paths)


def classify_payloads(
    pull_request: Any,
    files_pages: Any,
    *,
    expected_repository: str,
    expected_head_repository: str,
    expected_number: int,
    expected_base_sha: str,
    expected_head_sha: str,
    expected_base_ref: str = "main",
    expected_head_ref: str | None = None,
    expected_state: str | None = None,
) -> Selection:
    changed_files = _validate_identity(
        pull_request,
        expected_repository=expected_repository,
        expected_head_repository=expected_head_repository,
        expected_number=expected_number,
        expected_base_ref=expected_base_ref,
        expected_head_ref=expected_head_ref,
        expected_base_sha=expected_base_sha,
        expected_head_sha=expected_head_sha,
        expected_state=expected_state,
    )
    return selection_for_paths(_changed_paths(files_pages, changed_files))


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pull-request", type=Path, required=True)
    parser.add_argument("--files", type=Path, required=True)
    parser.add_argument("--expected-repository", required=True)
    parser.add_argument("--expected-head-repository", required=True)
    parser.add_argument("--expected-head-ref")
    parser.add_argument("--expected-number", type=int, required=True)
    parser.add_argument("--expected-base-ref", default="main", choices=sorted(ALLOWED_BASE_REFS))
    parser.add_argument("--expected-base-sha", required=True)
    parser.add_argument("--expected-head-sha", required=True)
    parser.add_argument("--expected-state", choices=("open", "closed"))
    parser.add_argument("--format", choices=("legacy", "json"), default="legacy")
    args = parser.parse_args(argv)
    try:
        result = classify_payloads(
            load_json(args.pull_request),
            load_json(args.files),
            expected_repository=args.expected_repository,
            expected_head_repository=args.expected_head_repository,
            expected_head_ref=args.expected_head_ref,
            expected_number=args.expected_number,
            expected_base_ref=args.expected_base_ref,
            expected_base_sha=args.expected_base_sha,
            expected_head_sha=args.expected_head_sha,
            expected_state=args.expected_state,
        )
    except ScopeError as exc:
        print(f"ci-change-scope: FAIL {exc}", file=sys.stderr)
        return 1
    print(result.as_json() if args.format == "json" else result.legacy_scope())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Classify a main pull request as product or allowlisted non-product work.

The classifier is deliberately pure: workflows obtain pull-request JSON from
GitHub, while this module validates identity, pagination completeness, rename
boundaries, and paths without network access or repository mutation.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any, Sequence


NON_PRODUCT_EXACT_PATHS = frozenset({"AGENTS.md", "README.md"})
NON_PRODUCT_PREFIXES = ("docs/", ".github/ISSUE_TEMPLATE/")
KNOWN_FILE_STATUSES = frozenset(
    {"added", "removed", "modified", "renamed", "copied", "changed", "unchanged"}
)
FULL_SHA = re.compile(r"^[0-9a-fA-F]{40}$")
REPOSITORY = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")


class ScopeError(ValueError):
    """Raised when GitHub data cannot prove a complete, bound classification."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ScopeError("duplicate JSON object key")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        raw = path.read_text(encoding="utf-8")
        return json.loads(raw, object_pairs_hook=_reject_duplicate_keys)
    except ScopeError:
        raise
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ScopeError("JSON input is unreadable or malformed") from exc


def _require_mapping(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ScopeError(f"{label} is not an object")
    return value


def _require_string(mapping: dict[str, Any], key: str, label: str) -> str:
    value = mapping.get(key)
    if not isinstance(value, str) or not value:
        raise ScopeError(f"{label} is missing or malformed")
    return value


def _validate_expected_identity(
    expected_repository: str,
    expected_head_repository: str,
    expected_number: int,
    expected_base_sha: str,
    expected_head_sha: str,
) -> None:
    if not REPOSITORY.fullmatch(expected_repository):
        raise ScopeError("expected base repository is malformed")
    if not REPOSITORY.fullmatch(expected_head_repository):
        raise ScopeError("expected head repository is malformed")
    if type(expected_number) is not int or expected_number <= 0:
        raise ScopeError("expected pull request number is malformed")
    if not FULL_SHA.fullmatch(expected_base_sha) or not FULL_SHA.fullmatch(expected_head_sha):
        raise ScopeError("expected pull request SHA is malformed")


def _validate_pull_request(
    pull_request: Any,
    *,
    expected_repository: str,
    expected_head_repository: str,
    expected_number: int,
    expected_base_sha: str,
    expected_head_sha: str,
) -> int:
    root = _require_mapping(pull_request, "pull request")
    if type(root.get("number")) is not int or root["number"] != expected_number:
        raise ScopeError("pull request number mismatch")

    base = _require_mapping(root.get("base"), "pull request base")
    base_repository = _require_mapping(base.get("repo"), "pull request base repository")
    if _require_string(base_repository, "full_name", "base repository name") != expected_repository:
        raise ScopeError("pull request base repository mismatch")
    if _require_string(base, "ref", "base ref") != "main":
        raise ScopeError("pull request base ref is not main")
    if _require_string(base, "sha", "base SHA") != expected_base_sha:
        raise ScopeError("pull request base SHA mismatch")

    head = _require_mapping(root.get("head"), "pull request head")
    head_repository = _require_mapping(head.get("repo"), "pull request head repository")
    if _require_string(head_repository, "full_name", "head repository name") != expected_head_repository:
        raise ScopeError("pull request head repository mismatch")
    if _require_string(head, "sha", "head SHA") != expected_head_sha:
        raise ScopeError("pull request head SHA mismatch")

    changed_files = root.get("changed_files")
    if type(changed_files) is not int or changed_files <= 0:
        raise ScopeError("pull request changed_files is missing or empty")
    return changed_files


def _validate_path(value: Any) -> str:
    if not isinstance(value, str) or not value or "\x00" in value or value.startswith("/"):
        raise ScopeError("changed file path is malformed")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise ScopeError("changed file path is not normalized")
    return value


def _is_non_product_path(path: str) -> bool:
    return path in NON_PRODUCT_EXACT_PATHS or path.startswith(NON_PRODUCT_PREFIXES)


def _changed_paths(files_pages: Any, expected_count: int) -> tuple[str, ...]:
    if not isinstance(files_pages, list) or not files_pages:
        raise ScopeError("pull request file pagination is missing")

    paths: list[str] = []
    filenames: set[str] = set()
    records = 0
    for page in files_pages:
        if not isinstance(page, list):
            raise ScopeError("pull request file page is malformed")
        for raw_file in page:
            file_info = _require_mapping(raw_file, "pull request file")
            records += 1
            filename = _validate_path(file_info.get("filename"))
            if filename in filenames:
                raise ScopeError("pull request file pagination contains a duplicate")
            filenames.add(filename)

            status = file_info.get("status")
            if not isinstance(status, str) or status not in KNOWN_FILE_STATUSES:
                raise ScopeError("pull request file status is malformed")
            paths.append(filename)

            previous = file_info.get("previous_filename")
            if status in {"renamed", "copied"} and previous is None:
                raise ScopeError("rename or copy is missing its previous path")
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
) -> str:
    _validate_expected_identity(
        expected_repository,
        expected_head_repository,
        expected_number,
        expected_base_sha,
        expected_head_sha,
    )
    changed_files = _validate_pull_request(
        pull_request,
        expected_repository=expected_repository,
        expected_head_repository=expected_head_repository,
        expected_number=expected_number,
        expected_base_sha=expected_base_sha,
        expected_head_sha=expected_head_sha,
    )
    paths = _changed_paths(files_pages, changed_files)
    return "non-product" if all(_is_non_product_path(path) for path in paths) else "product"


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pull-request", type=Path, required=True)
    parser.add_argument("--files", type=Path, required=True)
    parser.add_argument("--expected-repository", required=True)
    parser.add_argument("--expected-head-repository", required=True)
    parser.add_argument("--expected-number", type=int, required=True)
    parser.add_argument("--expected-base-sha", required=True)
    parser.add_argument("--expected-head-sha", required=True)
    args = parser.parse_args(argv)
    try:
        result = classify_payloads(
            load_json(args.pull_request),
            load_json(args.files),
            expected_repository=args.expected_repository,
            expected_head_repository=args.expected_head_repository,
            expected_number=args.expected_number,
            expected_base_sha=args.expected_base_sha,
            expected_head_sha=args.expected_head_sha,
        )
    except ScopeError as exc:
        print(f"ci-change-scope: FAIL {exc}", file=sys.stderr)
        return 1
    print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

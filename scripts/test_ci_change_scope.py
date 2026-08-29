#!/usr/bin/env python3
"""Finite independent fixtures for the main-PR change-scope classifier."""

from __future__ import annotations

from contextlib import redirect_stdout
import io
import json
from pathlib import Path
import tempfile
import unittest

import ci_change_scope


REPOSITORY = "owner/repository"
HEAD_REPOSITORY = "owner/fork"
BASE_SHA = "a" * 40
HEAD_SHA = "b" * 40
NUMBER = 24


def pull_request(changed_files: int) -> dict[str, object]:
    return {
        "number": NUMBER,
        "changed_files": changed_files,
        "base": {"ref": "main", "sha": BASE_SHA, "repo": {"full_name": REPOSITORY}},
        "head": {"ref": "feature", "sha": HEAD_SHA, "repo": {"full_name": HEAD_REPOSITORY}},
    }


def changed_file(filename: str, status: str = "modified", **extra: object) -> dict[str, object]:
    return {"filename": filename, "status": status, **extra}


def classify(files: list[list[dict[str, object]]], changed_files: int | None = None) -> str:
    count = sum(len(page) for page in files) if changed_files is None else changed_files
    return ci_change_scope.classify_payloads(
        pull_request(count),
        files,
        expected_repository=REPOSITORY,
        expected_head_repository=HEAD_REPOSITORY,
        expected_number=NUMBER,
        expected_base_sha=BASE_SHA,
        expected_head_sha=HEAD_SHA,
    )


class ChangeScopeTests(unittest.TestCase):
    def test_exact_and_nested_allowlist_is_non_product(self) -> None:
        files = [[
            changed_file("AGENTS.md"),
            changed_file("README.md"),
            changed_file("docs/operations/runbook.md"),
            changed_file(".github/ISSUE_TEMPLATE/work-item.md"),
        ]]
        self.assertEqual(classify(files), "non-product")

    def test_multiple_pages_are_complete(self) -> None:
        files = [[changed_file("docs/a.md")], [changed_file("docs/b.md")]]
        self.assertEqual(classify(files), "non-product")

    def test_product_path_is_product(self) -> None:
        self.assertEqual(classify([[changed_file("src/main.rs")]]), "product")

    def test_mixed_paths_are_product(self) -> None:
        files = [[changed_file("docs/a.md"), changed_file("Cargo.toml")]]
        self.assertEqual(classify(files), "product")

    def test_unknown_root_path_is_product(self) -> None:
        self.assertEqual(classify([[changed_file("README.en.md")]]), "product")

    def test_workflow_change_is_product(self) -> None:
        path = ".github/workflows/windows-client.yml"
        self.assertEqual(classify([[changed_file(path)]]), "product")

    def test_allowlisted_rename_is_non_product(self) -> None:
        file_info = changed_file(
            "docs/new.md", "renamed", previous_filename="docs/old.md"
        )
        self.assertEqual(classify([[file_info]]), "non-product")

    def test_product_rename_source_is_product(self) -> None:
        file_info = changed_file(
            "docs/main.md", "renamed", previous_filename="src/main.rs"
        )
        self.assertEqual(classify([[file_info]]), "product")

    def test_product_rename_destination_is_product(self) -> None:
        file_info = changed_file(
            "src/main.rs", "renamed", previous_filename="docs/main.md"
        )
        self.assertEqual(classify([[file_info]]), "product")

    def test_missing_rename_source_fails(self) -> None:
        with self.assertRaises(ci_change_scope.ScopeError):
            classify([[changed_file("docs/new.md", "renamed")]])

    def test_incomplete_pagination_fails(self) -> None:
        with self.assertRaises(ci_change_scope.ScopeError):
            classify([[changed_file("docs/a.md")]], changed_files=2)

    def test_duplicate_file_record_fails(self) -> None:
        files = [[changed_file("docs/a.md")], [changed_file("docs/a.md")]]
        with self.assertRaises(ci_change_scope.ScopeError):
            classify(files)

    def test_empty_change_set_fails(self) -> None:
        with self.assertRaises(ci_change_scope.ScopeError):
            classify([], changed_files=0)

    def test_identity_mismatch_fails(self) -> None:
        candidate = pull_request(1)
        candidate["number"] = NUMBER + 1
        with self.assertRaises(ci_change_scope.ScopeError):
            ci_change_scope.classify_payloads(
                candidate,
                [[changed_file("docs/a.md")]],
                expected_repository=REPOSITORY,
                expected_head_repository=HEAD_REPOSITORY,
                expected_number=NUMBER,
                expected_base_sha=BASE_SHA,
                expected_head_sha=HEAD_SHA,
            )

    def test_malformed_or_duplicate_json_fails(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ci-change-scope-") as temporary:
            malformed = Path(temporary) / "malformed.json"
            duplicate = Path(temporary) / "duplicate.json"
            malformed.write_text("{", encoding="utf-8")
            duplicate.write_text('{"number":24,"number":25}', encoding="utf-8")
            with self.assertRaises(ci_change_scope.ScopeError):
                ci_change_scope.load_json(malformed)
            with self.assertRaises(ci_change_scope.ScopeError):
                ci_change_scope.load_json(duplicate)

    def test_cli_output_is_exact(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ci-change-scope-cli-") as temporary:
            root = Path(temporary)
            pull_path = root / "pull.json"
            files_path = root / "files.json"
            pull_path.write_text(json.dumps(pull_request(1)), encoding="utf-8")
            files_path.write_text(
                json.dumps([[changed_file("docs/a.md")]]), encoding="utf-8"
            )
            argv = [
                "--pull-request", str(pull_path),
                "--files", str(files_path),
                "--expected-repository", REPOSITORY,
                "--expected-head-repository", HEAD_REPOSITORY,
                "--expected-number", str(NUMBER),
                "--expected-base-sha", BASE_SHA,
                "--expected-head-sha", HEAD_SHA,
            ]
            output = io.StringIO()
            with redirect_stdout(output):
                self.assertEqual(ci_change_scope.main(argv), 0)
            self.assertEqual(output.getvalue(), "non-product\n")


if __name__ == "__main__":
    unittest.main()

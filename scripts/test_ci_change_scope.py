#!/usr/bin/env python3
"""Finite tests for the fail-closed selective CI classifier."""

from __future__ import annotations

import subprocess
import unittest

from ci_change_scope import ScopeError, classify_payloads, owners_for_path, selection_for_paths


REPOSITORY = "example/project"
HEAD_REPOSITORY = "example/project"
BASE_SHA = "1" * 40
HEAD_SHA = "2" * 40


def pull_request(*, base_ref: str = "main", changed_files: int = 1) -> dict:
    return {
        "number": 51,
        "state": "open",
        "changed_files": changed_files,
        "base": {"ref": base_ref, "sha": BASE_SHA, "repo": {"full_name": REPOSITORY}},
        "head": {"ref": "codex/change", "sha": HEAD_SHA, "repo": {"full_name": HEAD_REPOSITORY}},
    }


def classify(files: list[dict], *, base_ref: str = "main", pages: list[list[dict]] | None = None):
    return classify_payloads(
        pull_request(base_ref=base_ref, changed_files=len(files)),
        pages if pages is not None else [files],
        expected_repository=REPOSITORY,
        expected_head_repository=HEAD_REPOSITORY,
        expected_head_ref="codex/change",
        expected_number=51,
        expected_base_ref=base_ref,
        expected_base_sha=BASE_SHA,
        expected_head_sha=HEAD_SHA,
        expected_state="open",
    )


class OwnerTableTests(unittest.TestCase):
    def test_single_owner_paths(self) -> None:
        cases = {
            "docs/PRODUCT_REQUIREMENTS.md": ("DOCS",),
            ".github/workflows/feat-integration.yml": ("GOVERNANCE",),
            "src/server.rs": ("LINUX_BACKEND",),
            "ui/app.slint": ("LINUX_UI",),
            "windows-client/src/App.cs": ("WINDOWS",),
        }
        for path, expected in cases.items():
            with self.subTest(path=path):
                self.assertEqual(tuple(sorted(owners_for_path(path))), expected)

    def test_shared_paths(self) -> None:
        self.assertEqual(
            owners_for_path("Cargo.lock"), frozenset({"LINUX_BACKEND", "LINUX_UI"})
        )
        self.assertEqual(
            owners_for_path("protocol/status.schema.json"),
            frozenset({"LINUX_BACKEND", "WINDOWS"}),
        )
        self.assertEqual(
            owners_for_path("LICENSES/dependency.txt"),
            frozenset({"LINUX_BACKEND", "LINUX_UI", "WINDOWS"}),
        )

    def test_every_tracked_head_path_has_an_owner(self) -> None:
        paths = subprocess.run(
            ["git", "-c", "core.quotepath=false", "ls-files", "-z"],
            check=True,
            capture_output=True,
        ).stdout.decode("utf-8").rstrip("\0").split("\0")
        self.assertGreater(len(paths), 0)
        for path in paths:
            with self.subTest(path=path):
                self.assertTrue(owners_for_path(path))

    def test_unknown_and_non_normalized_paths_fail_closed(self) -> None:
        for path in ("future/unknown.bin", "../Cargo.toml", "/Cargo.toml", "a//b"):
            with self.subTest(path=path), self.assertRaises(ScopeError):
                owners_for_path(path)


class SelectionTests(unittest.TestCase):
    def test_docs_has_no_binary_or_codeql(self) -> None:
        result = selection_for_paths(["README.md"])
        self.assertEqual(result.owners, ("DOCS",))
        self.assertEqual(result.codeql_languages, ())
        self.assertFalse(result.binary_impact)

    def test_governance_selects_actions_and_python(self) -> None:
        result = selection_for_paths([".github/workflows/codeql.yml"])
        self.assertEqual(result.owners, ("GOVERNANCE",))
        self.assertEqual(result.codeql_languages, ("actions", "python"))
        self.assertFalse(result.binary_impact)

    def test_mixed_selection_is_a_deduplicated_finite_union(self) -> None:
        result = selection_for_paths(
            ["README.md", "src/server.rs", "ui/app.slint", "windows-client/src/App.cs"]
        )
        self.assertEqual(
            result.owners, ("DOCS", "LINUX_BACKEND", "LINUX_UI", "WINDOWS")
        )
        self.assertEqual(result.codeql_languages, ("csharp", "rust"))
        self.assertTrue(result.binary_impact)


class PayloadTests(unittest.TestCase):
    def test_closed_event_remains_classifiable_for_postmerge_release(self) -> None:
        pr = pull_request()
        pr["state"] = "closed"
        result = classify_payloads(
            pr,
            [[{"filename": "README.md", "status": "modified"}]],
            expected_repository=REPOSITORY,
            expected_head_repository=HEAD_REPOSITORY,
            expected_number=51,
            expected_base_sha=BASE_SHA,
            expected_head_sha=HEAD_SHA,
        )
        self.assertEqual(result.owners, ("DOCS",))

    def test_main_and_feat_next_are_the_only_base_refs(self) -> None:
        for ref in ("main", "feat/next"):
            with self.subTest(ref=ref):
                self.assertEqual(
                    classify([{"filename": "README.md", "status": "modified"}], base_ref=ref).owners,
                    ("DOCS",),
                )
        with self.assertRaises(ScopeError):
            classify_payloads(
                pull_request(base_ref="develop"),
                [[{"filename": "README.md", "status": "modified"}]],
                expected_repository=REPOSITORY,
                expected_head_repository=HEAD_REPOSITORY,
                expected_number=51,
                expected_base_ref="develop",
                expected_base_sha=BASE_SHA,
                expected_head_sha=HEAD_SHA,
                expected_state="open",
            )

    def test_rename_and_copy_union_old_and_new_owners(self) -> None:
        for status in ("renamed", "copied"):
            result = classify(
                [{"filename": "docs/moved.md", "previous_filename": "src/old.rs", "status": status}]
            )
            self.assertEqual(result.owners, ("DOCS", "LINUX_BACKEND"))

    def test_deleted_base_only_path_is_classified(self) -> None:
        result = classify([{"filename": "windows-client/src/Old.cs", "status": "removed"}])
        self.assertEqual(result.owners, ("WINDOWS",))

    def test_complete_pagination_is_accepted(self) -> None:
        files = [
            {"filename": "README.md", "status": "modified"},
            {"filename": "src/server.rs", "status": "modified"},
        ]
        result = classify(files, pages=[[files[0]], [files[1]]])
        self.assertEqual(result.owners, ("DOCS", "LINUX_BACKEND"))

    def test_incomplete_duplicate_and_unknown_payloads_fail(self) -> None:
        cases = (
            (pull_request(changed_files=2), [[{"filename": "README.md", "status": "modified"}]]),
            (
                pull_request(changed_files=2),
                [[
                    {"filename": "README.md", "status": "modified"},
                    {"filename": "README.md", "status": "modified"},
                ]],
            ),
            (pull_request(), [[{"filename": "future/new.file", "status": "modified"}]]),
            (
                pull_request(),
                [[{"filename": "README.md", "previous_filename": "DESIGN.md", "status": "modified"}]],
            ),
        )
        for pr, pages in cases:
            with self.subTest(pages=pages), self.assertRaises(ScopeError):
                classify_payloads(
                    pr,
                    pages,
                    expected_repository=REPOSITORY,
                    expected_head_repository=HEAD_REPOSITORY,
                    expected_number=51,
                    expected_base_sha=BASE_SHA,
                    expected_head_sha=HEAD_SHA,
                    expected_state="open",
                )

    def test_identity_and_head_move_fail_closed(self) -> None:
        mutations = (
            ("state", "closed"),
            ("number", 52),
            ("head.sha", "3" * 40),
            ("head.ref", "codex/moved"),
            ("base.sha", "4" * 40),
        )
        for key, value in mutations:
            pr = pull_request()
            target = pr
            parts = key.split(".")
            for part in parts[:-1]:
                target = target[part]
            target[parts[-1]] = value
            with self.subTest(key=key), self.assertRaises(ScopeError):
                classify_payloads(
                    pr,
                    [[{"filename": "README.md", "status": "modified"}]],
                    expected_repository=REPOSITORY,
                    expected_head_repository=HEAD_REPOSITORY,
                    expected_head_ref="codex/change",
                    expected_number=51,
                    expected_base_sha=BASE_SHA,
                    expected_head_sha=HEAD_SHA,
                    expected_state="open",
                )


if __name__ == "__main__":
    unittest.main()

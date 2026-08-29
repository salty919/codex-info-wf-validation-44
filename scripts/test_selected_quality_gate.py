#!/usr/bin/env python3
"""Finite result-state tests for selected_quality_gate.py."""

from __future__ import annotations

import json
import unittest

from selected_quality_gate import QualitySelectionError, validate


JOBS = (
    "docs-quality",
    "governance-quality",
    "linux-backend-quality",
    "linux-ui-quality",
    "windows-quality",
    "codeql-quality",
)


def payload(owners: list[str], languages: list[str]) -> str:
    return json.dumps({"owners": owners, "codeql_languages": languages})


def results(*successful: str) -> str:
    return json.dumps({job: "success" if job in successful else "skipped" for job in JOBS})


class SelectedQualityTests(unittest.TestCase):
    def test_docs_only(self) -> None:
        validate(payload(["DOCS"], []), results("docs-quality"))

    def test_windows_only_with_csharp_codeql(self) -> None:
        validate(
            payload(["WINDOWS"], ["csharp"]),
            results("windows-quality", "codeql-quality"),
        )

    def test_mixed_union(self) -> None:
        validate(
            payload(["GOVERNANCE", "LINUX_UI"], ["actions", "python", "rust"]),
            results("governance-quality", "linux-ui-quality", "codeql-quality"),
        )

    def test_every_failure_cancel_missing_and_unexpected_success_is_rejected(self) -> None:
        baseline = json.loads(results("windows-quality", "codeql-quality"))
        mutations = []
        for value in ("failure", "cancelled", "skipped", "neutral", ""):
            candidate = dict(baseline)
            candidate["windows-quality"] = value
            mutations.append(candidate)
        candidate = dict(baseline)
        candidate["docs-quality"] = "success"
        mutations.append(candidate)
        candidate = dict(baseline)
        candidate.pop("codeql-quality")
        mutations.append(candidate)
        candidate = dict(baseline)
        candidate["foreign-quality"] = "success"
        mutations.append(candidate)
        for candidate in mutations:
            with self.subTest(candidate=candidate), self.assertRaises(QualitySelectionError):
                validate(payload(["WINDOWS"], ["csharp"]), json.dumps(candidate))

    def test_unknown_duplicate_or_empty_selection_is_rejected(self) -> None:
        bad = (
            payload([], []),
            payload(["WINDOWS", "WINDOWS"], ["csharp"]),
            payload(["FOREIGN"], []),
            payload(["DOCS"], ["ruby"]),
            payload(["WINDOWS"], ["csharp", "csharp"]),
            payload(["WINDOWS"], []),
            payload(["WINDOWS", "DOCS"], ["csharp"]),
        )
        for selection in bad:
            with self.subTest(selection=selection), self.assertRaises(QualitySelectionError):
                validate(selection, results("docs-quality"))


if __name__ == "__main__":
    unittest.main()

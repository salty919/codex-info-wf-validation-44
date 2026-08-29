#!/usr/bin/env python3
"""Tests for release resolution through final-head acceptance check-runs."""

from __future__ import annotations

import copy
import unittest
from collections.abc import Callable
from typing import Any

import release_quality_run_resolver as resolver

REPOSITORY = "salty919/codex_info_v2"
PR_NUMBER = 42
HEAD_SHA = "2" * 40
BASE_SHA = "1" * 40
MERGE_SHA = "3" * 40
RUN_ID = 987654


def pull_request() -> dict[str, Any]:
    return {
        "number": PR_NUMBER,
        "merged": True,
        "merged_at": "2026-08-29T12:30:00Z",
        "merge_commit_sha": MERGE_SHA,
        "head": {
            "sha": HEAD_SHA,
            "ref": "feat/next",
            "repo": {"full_name": REPOSITORY},
        },
        "base": {
            "sha": BASE_SHA,
            "ref": "main",
            "repo": {"full_name": REPOSITORY},
        },
    }


def event() -> dict[str, Any]:
    return {
        "action": "closed",
        "repository": {"full_name": REPOSITORY},
        "pull_request": pull_request(),
    }


def acceptance_check() -> dict[str, Any]:
    return {
        "id": 555,
        "name": "acceptance",
        "head_sha": HEAD_SHA,
        "status": "completed",
        "conclusion": "success",
        "external_id": (
            f"codex-quality-v1:pr={PR_NUMBER}:head={HEAD_SHA}:run={RUN_ID}"
        ),
        "details_url": f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}",
        "app": {"id": resolver.GITHUB_ACTIONS_APP_ID},
    }


def check_runs(*runs: dict[str, Any]) -> dict[str, Any]:
    selected = list(runs) if runs else [acceptance_check()]
    return {"total_count": len(selected), "check_runs": selected}


def workflow_run() -> dict[str, Any]:
    return {
        "id": RUN_ID,
        "run_attempt": 1,
        "path": ".github/workflows/version-prepare.yml",
        "event": "pull_request_target",
        "head_branch": "main",
        "head_sha": BASE_SHA,
        "repository": {"full_name": REPOSITORY},
        "status": "completed",
        "conclusion": "success",
        "html_url": f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}",
        "created_at": "2026-08-29T11:00:00Z",
        "updated_at": "2026-08-29T12:00:00Z",
    }


class ReleaseQualityRunResolverTests(unittest.TestCase):
    def test_selects_and_verifies_unique_successful_final_head_run(self) -> None:
        self.assertEqual(
            resolver.resolve_quality_run_id(event(), pull_request(), check_runs()),
            RUN_ID,
        )
        self.assertEqual(
            resolver.verify_quality_run(
                event(), pull_request(), check_runs(), workflow_run()
            ),
            RUN_ID,
        )

    def test_accepts_complete_paginated_check_response(self) -> None:
        unrelated = acceptance_check()
        unrelated["app"] = {"id": 999}
        pages = [
            {"total_count": 2, "check_runs": [unrelated]},
            {"total_count": 2, "check_runs": [acceptance_check()]},
        ]
        self.assertEqual(
            resolver.resolve_quality_run_id(event(), pull_request(), pages), RUN_ID
        )

    def test_rejects_duplicate_or_missing_actions_acceptance(self) -> None:
        for runs in (
            check_runs(),
            check_runs(acceptance_check(), acceptance_check()),
        ):
            with self.subTest(count=len(runs["check_runs"])):
                if len(runs["check_runs"]) == 1:
                    runs["check_runs"][0]["app"] = {"id": 999}
                with self.assertRaises(resolver.ResolutionError):
                    resolver.resolve_quality_run_id(event(), pull_request(), runs)

    def test_rejects_non_successful_or_wrong_candidate_check(self) -> None:
        mutations: tuple[Callable[[dict[str, Any]], None], ...] = (
            lambda value: value.update(conclusion="failure"),
            lambda value: value.update(status="in_progress", conclusion=None),
            lambda value: value.update(head_sha="4" * 40),
            lambda value: value.update(
                external_id=(f"codex-quality-v1:pr=99:head={HEAD_SHA}:run={RUN_ID}")
            ),
            lambda value: value.update(details_url="https://example.invalid/run"),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                accepted = acceptance_check()
                mutate(accepted)
                with self.assertRaises(resolver.ResolutionError):
                    resolver.resolve_quality_run_id(
                        event(), pull_request(), check_runs(accepted)
                    )

    def test_rejects_incomplete_check_pagination(self) -> None:
        incomplete = check_runs()
        incomplete["total_count"] = 2
        with self.assertRaisesRegex(resolver.ResolutionError, "incomplete"):
            resolver.resolve_quality_run_id(event(), pull_request(), incomplete)

    def test_rejects_event_and_live_pull_request_disagreement(self) -> None:
        live = pull_request()
        live["head"]["sha"] = "4" * 40
        with self.assertRaisesRegex(resolver.ResolutionError, "does not match"):
            resolver.resolve_quality_run_id(event(), live, check_runs())

    def test_rejects_untrusted_or_unsuccessful_workflow_run(self) -> None:
        mutations: tuple[Callable[[dict[str, Any]], None], ...] = (
            lambda value: value.update(id=RUN_ID + 1),
            lambda value: value.update(path=".github/workflows/windows-client.yml"),
            lambda value: value.update(event="workflow_dispatch"),
            lambda value: value.update(head_sha="4" * 40),
            lambda value: value.update(conclusion="failure"),
            lambda value: value.update(updated_at="2026-08-29T13:00:00Z"),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                run = workflow_run()
                mutate(run)
                with self.assertRaises(resolver.ResolutionError):
                    resolver.verify_quality_run(
                        event(), pull_request(), check_runs(), run
                    )

    def test_does_not_accept_non_main_or_fork_boundary(self) -> None:
        for target in ("head", "base"):
            with self.subTest(target=target):
                changed_event = event()
                changed_pr = pull_request()
                if target == "head":
                    changed_event["pull_request"]["head"]["repo"]["full_name"] = (
                        "other/repo"
                    )
                    changed_pr["head"]["repo"]["full_name"] = "other/repo"
                else:
                    changed_event["pull_request"]["base"]["ref"] = "develop"
                    changed_pr["base"]["ref"] = "develop"
                with self.assertRaisesRegex(resolver.ResolutionError, "outside"):
                    resolver.resolve_quality_run_id(
                        changed_event, changed_pr, check_runs()
                    )

    def test_fixture_mutations_do_not_leak(self) -> None:
        first = workflow_run()
        second = copy.deepcopy(first)
        second["status"] = "queued"
        self.assertEqual(first["status"], "completed")


if __name__ == "__main__":
    unittest.main()

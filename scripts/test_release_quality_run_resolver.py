#!/usr/bin/env python3
"""Finite fixture tests for the release quality-run resolver."""

from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
RESOLVER = ROOT / "scripts" / "release_quality_run_resolver.py"
HEAD_SHA = "a" * 40
MERGE_SHA = "b" * 40
BASE_SHA = "d" * 40
BASE_REPOSITORY = {"id": 202, "full_name": "salty919/codex_info_v2"}
HEAD_REPOSITORY = copy.deepcopy(BASE_REPOSITORY)


def event_fixture() -> dict[str, Any]:
    return {
        "action": "closed",
        "repository": copy.deepcopy(BASE_REPOSITORY),
        "pull_request": {
            "number": 42,
            "merged": True,
            "head": {
                "sha": HEAD_SHA,
                "ref": "feature/release",
                "repo": copy.deepcopy(HEAD_REPOSITORY),
            },
            "base": {
                "ref": "main",
                "sha": BASE_SHA,
                "repo": copy.deepcopy(BASE_REPOSITORY),
            },
            "merge_commit_sha": MERGE_SHA,
        }
    }


def pull_request_fixture() -> dict[str, Any]:
    result = copy.deepcopy(event_fixture()["pull_request"])
    result["merged"] = True
    return result


def runs_fixture(pull_requests: Any = None) -> dict[str, Any]:
    if pull_requests is None:
        pull_requests = []
    return {
        "workflow_runs": [
            {
                "id": 987654,
                "path": ".github/workflows/windows-client.yml",
                "event": "pull_request",
                "status": "completed",
                "conclusion": "success",
                "head_sha": HEAD_SHA,
                "head_commit": {"id": HEAD_SHA},
                "head_branch": "feature/release",
                "head_repository": copy.deepcopy(HEAD_REPOSITORY),
                "repository": copy.deepcopy(BASE_REPOSITORY),
                "ref": None,
                "referenced_workflows": [
                    {
                        "path": (
                            BASE_REPOSITORY["full_name"]
                            + "/.github/workflows/rust.yml@"
                            + "c" * 40
                        ),
                        "ref": "refs/pull/42/merge",
                        "sha": "c" * 40,
                    }
                ],
                "pull_requests": pull_requests,
            }
        ]
    }


def mutate(base: Any, change: Callable[[dict[str, Any]], None]) -> Any:
    result = copy.deepcopy(base)
    change(result)
    return result


class ReleaseQualityRunResolverTests(unittest.TestCase):
    def run_resolver(
        self, event: Any, pull_request: Any, workflow_runs: Any
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            paths = {
                "event": Path(directory) / "event.json",
                "pull-request": Path(directory) / "pull-request.json",
                "workflow-runs": Path(directory) / "workflow-runs.json",
            }
            values = {
                "event": event,
                "pull-request": pull_request,
                "workflow-runs": workflow_runs,
            }
            for name, path in paths.items():
                path.write_text(json.dumps(values[name]), encoding="utf-8")
            return subprocess.run(
                [
                    sys.executable,
                    str(RESOLVER),
                    "--event",
                    str(paths["event"]),
                    "--pull-request",
                    str(paths["pull-request"]),
                    "--workflow-runs",
                    str(paths["workflow-runs"]),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )

    def assert_rejected(self, event: Any, pull_request: Any, runs: Any) -> None:
        result = self.run_resolver(event, pull_request, runs)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_accepts_empty_and_nonempty_pull_requests(self) -> None:
        for pull_requests in ([], [{"number": 999, "malformed": True}]):
            with self.subTest(pull_requests=pull_requests):
                result = self.run_resolver(
                    event_fixture(), pull_request_fixture(), runs_fixture(pull_requests)
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "987654\n")

    def test_ignores_top_level_run_ref(self) -> None:
        for top_level_ref in (None, "refs/heads/not-the-pull-request-ref"):
            with self.subTest(top_level_ref=top_level_ref):
                runs = runs_fixture()
                runs["workflow_runs"][0]["ref"] = top_level_ref
                result = self.run_resolver(
                    event_fixture(), pull_request_fixture(), runs
                )
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_rejects_zero_or_two_runs(self) -> None:
        self.assert_rejected(
            event_fixture(), pull_request_fixture(), {"workflow_runs": []}
        )
        two_runs = runs_fixture()
        two_runs["workflow_runs"].append(copy.deepcopy(two_runs["workflow_runs"][0]))
        two_runs["workflow_runs"][1]["id"] = 987655
        self.assert_rejected(event_fixture(), pull_request_fixture(), two_runs)

    def test_ignores_unsuccessful_runs_around_valid_success(self) -> None:
        success = runs_fixture()["workflow_runs"][0]
        unsuccessful = copy.deepcopy(success)
        unsuccessful["id"] = 987653
        unsuccessful["conclusion"] = "failure"
        for name, ordered_runs in (
            ("failed before success", [unsuccessful, success]),
            ("failed after success", [success, unsuccessful]),
        ):
            with self.subTest(name=name):
                result = self.run_resolver(
                    event_fixture(),
                    pull_request_fixture(),
                    {"workflow_runs": ordered_runs},
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "987654\n")

        cancelled = copy.deepcopy(success)
        cancelled["id"] = 987652
        cancelled["conclusion"] = "cancelled"
        result = self.run_resolver(
            event_fixture(),
            pull_request_fixture(),
            {"workflow_runs": [cancelled, success]},
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "987654\n")

    def test_rejects_unsuccessful_only_runs(self) -> None:
        unsuccessful = runs_fixture()["workflow_runs"][0]
        unsuccessful["conclusion"] = "failure"
        self.assert_rejected(
            event_fixture(), pull_request_fixture(), {"workflow_runs": [unsuccessful]}
        )

    def test_rejects_non_dict_workflow_run(self) -> None:
        self.assert_rejected(
            event_fixture(), pull_request_fixture(), {"workflow_runs": [None]}
        )

    def test_rejects_event_and_pull_request_identity_mismatches(self) -> None:
        cases: dict[str, tuple[Any, Any]] = {
            "event number": (
                mutate(event_fixture(), lambda value: value["pull_request"].update(number=0)),
                pull_request_fixture(),
            ),
            "event action": (
                mutate(event_fixture(), lambda value: value.update(action="reopened")),
                pull_request_fixture(),
            ),
            "event repository id": (
                mutate(
                    event_fixture(),
                    lambda value: value["repository"].update(id=303),
                ),
                pull_request_fixture(),
            ),
            "event repository name": (
                mutate(
                    event_fixture(),
                    lambda value: value["repository"].update(
                        full_name="other/repository"
                    ),
                ),
                pull_request_fixture(),
            ),
            "event repository format": (
                mutate(
                    event_fixture(),
                    lambda value: value["repository"].update(
                        full_name="invalid owner/repository"
                    ),
                ),
                pull_request_fixture(),
            ),
            "event head SHA format": (
                mutate(
                    event_fixture(),
                    lambda value: value["pull_request"]["head"].update(sha="bad"),
                ),
                pull_request_fixture(),
            ),
            "event merge SHA format": (
                mutate(
                    event_fixture(),
                    lambda value: value["pull_request"].update(merge_commit_sha=None),
                ),
                pull_request_fixture(),
            ),
            "PR number": (
                event_fixture(),
                mutate(pull_request_fixture(), lambda value: value.update(number=43)),
            ),
            "PR head SHA": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["head"].update(sha="c" * 40),
                ),
            ),
            "PR head ref": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["head"].update(ref="other"),
                ),
            ),
            "PR head repository": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["head"]["repo"].update(id=303),
                ),
            ),
            "PR base ref": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["base"].update(ref="develop"),
                ),
            ),
            "event base SHA missing": (
                mutate(
                    event_fixture(),
                    lambda value: value["pull_request"]["base"].pop("sha"),
                ),
                pull_request_fixture(),
            ),
            "event base SHA malformed": (
                mutate(
                    event_fixture(),
                    lambda value: value["pull_request"]["base"].update(sha="bad"),
                ),
                pull_request_fixture(),
            ),
            "PR base SHA mismatch": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["base"].update(sha="c" * 40),
                ),
            ),
            "PR base SHA malformed": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["base"].update(sha="bad"),
                ),
            ),
            "PR base SHA case": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["base"].update(sha="C" * 40),
                ),
            ),
            "PR base repository": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value["base"]["repo"].update(
                        full_name="other/repository"
                    ),
                ),
            ),
            "PR merge SHA": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value.update(merge_commit_sha="c" * 40),
                ),
            ),
            "PR not merged": (
                event_fixture(),
                mutate(pull_request_fixture(), lambda value: value.update(merged=False)),
            ),
        }
        for name, (event, pull_request) in cases.items():
            with self.subTest(name=name):
                self.assert_rejected(event, pull_request, runs_fixture())

    def test_rejects_workflow_run_identity_mismatches(self) -> None:
        fields: dict[str, Callable[[dict[str, Any]], None]] = {
            "path": lambda run: run.update(path="other.yml"),
            "event": lambda run: run.update(event="push"),
            "status": lambda run: run.update(status="in_progress"),
            "conclusion": lambda run: run.update(conclusion="failure"),
            "head SHA": lambda run: run.update(head_sha="c" * 40),
            "head commit": lambda run: run["head_commit"].update(id="c" * 40),
            "head branch": lambda run: run.update(head_branch="other"),
            "head repository id": lambda run: run["head_repository"].update(id=303),
            "head repository name": lambda run: run["head_repository"].update(
                full_name="other/repository"
            ),
            "base repository id": lambda run: run["repository"].update(id=303),
            "base repository name": lambda run: run["repository"].update(
                full_name="other/repository"
            ),
            "referenced workflows missing": lambda run: run.pop(
                "referenced_workflows"
            ),
            "referenced workflows null": lambda run: run.update(
                referenced_workflows=None
            ),
            "referenced workflows malformed": lambda run: run.update(
                referenced_workflows=[None]
            ),
            "referenced workflows zero": lambda run: run.update(
                referenced_workflows=[]
            ),
            "referenced workflows two": lambda run: run.update(
                referenced_workflows=[
                    run["referenced_workflows"][0],
                    copy.deepcopy(run["referenced_workflows"][0]),
                ]
            ),
            "referenced workflow PR": lambda run: run[
                "referenced_workflows"
            ][0].update(ref="refs/pull/43/merge"),
            "referenced workflow repository": lambda run: run[
                "referenced_workflows"
            ][0].update(
                path="other/repository/.github/workflows/rust.yml@" + "c" * 40
            ),
            "referenced workflow path": lambda run: run[
                "referenced_workflows"
            ][0].update(
                path=BASE_REPOSITORY["full_name"]
                + "/.github/workflows/other.yml@"
                + "c" * 40
            ),
            "referenced workflow suffix": lambda run: run[
                "referenced_workflows"
            ][0].update(
                path=BASE_REPOSITORY["full_name"]
                + "/.github/workflows/rust.yml@"
                + "d" * 40
            ),
            "referenced workflow SHA": lambda run: run[
                "referenced_workflows"
            ][0].update(sha="not-a-sha"),
            "referenced workflow SHA case": lambda run: run[
                "referenced_workflows"
            ][0].update(sha="C" * 40),
        }
        for name, change in fields.items():
            with self.subTest(name=name):
                runs = runs_fixture()
                change(runs["workflow_runs"][0])
                self.assert_rejected(event_fixture(), pull_request_fixture(), runs)

    def test_rejects_malformed_and_not_found_responses(self) -> None:
        cases = (
            ("event 404", {"message": "Not Found"}, pull_request_fixture(), runs_fixture()),
            ("PR 404", event_fixture(), {"message": "Not Found"}, runs_fixture()),
            ("runs 404", event_fixture(), pull_request_fixture(), {"message": "Not Found"}),
            ("malformed pages", event_fixture(), pull_request_fixture(), [{"message": "Not Found"}]),
        )
        for name, event, pull_request, runs in cases:
            with self.subTest(name=name):
                self.assert_rejected(event, pull_request, runs)

    def test_rejects_invalid_run_ids(self) -> None:
        for invalid_id in (0, -1, True, "987654"):
            with self.subTest(invalid_id=invalid_id):
                runs = runs_fixture()
                runs["workflow_runs"][0]["id"] = invalid_id
                self.assert_rejected(event_fixture(), pull_request_fixture(), runs)


if __name__ == "__main__":
    unittest.main()

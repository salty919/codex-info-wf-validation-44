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
MERGED_AT = "2026-08-29T00:05:00Z"
RUN_83_CREATED_AT = "2026-08-29T00:01:00Z"
RUN_83_UPDATED_AT = "2026-08-29T00:02:00Z"
RUN_84_CREATED_AT = "2026-08-29T00:03:00Z"
RUN_84_UPDATED_AT = "2026-08-29T00:04:00Z"
BASE_REPOSITORY = {"id": 202, "full_name": "salty919/codex_info_v2"}
HEAD_REPOSITORY = copy.deepcopy(BASE_REPOSITORY)


def event_fixture() -> dict[str, Any]:
    return {
        "action": "closed",
        "repository": copy.deepcopy(BASE_REPOSITORY),
        "pull_request": {
            "number": 42,
            "merged": True,
            "merged_at": MERGED_AT,
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


def workflow_run_fixture(
    *,
    run_id: int = 987654,
    run_number: int = 84,
    run_attempt: int = 1,
    status: str = "completed",
    conclusion: str | None = "success",
    created_at: str = RUN_84_CREATED_AT,
    updated_at: str = RUN_84_UPDATED_AT,
    pull_requests: Any = None,
) -> dict[str, Any]:
    if pull_requests is None:
        pull_requests = []
    return {
        "id": run_id,
        "run_number": run_number,
        "run_attempt": run_attempt,
        "path": ".github/workflows/windows-client.yml",
        "event": "workflow_dispatch",
        "status": status,
        "conclusion": conclusion,
        "created_at": created_at,
        "updated_at": updated_at,
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
                    + HEAD_SHA
                ),
                "ref": "refs/heads/feature/release",
                "sha": HEAD_SHA,
            },
            {
                "path": (
                    BASE_REPOSITORY["full_name"]
                    + "/.github/workflows/codeql.yml@"
                    + HEAD_SHA
                ),
                "ref": "refs/heads/feature/release",
                "sha": HEAD_SHA,
            },
        ],
        "pull_requests": pull_requests,
    }


def runs_response(
    workflow_runs: list[Any], *, total_count: int | None = None
) -> dict[str, Any]:
    if total_count is None:
        total_count = len(workflow_runs)
    return {"total_count": total_count, "workflow_runs": workflow_runs}


def runs_fixture(
    pull_requests: Any = None,
    *,
    run_id: int = 987654,
    run_number: int = 84,
    run_attempt: int = 1,
    status: str = "completed",
    conclusion: str | None = "success",
    created_at: str = RUN_84_CREATED_AT,
    updated_at: str = RUN_84_UPDATED_AT,
) -> dict[str, Any]:
    return runs_response(
        [
            workflow_run_fixture(
                run_id=run_id,
                run_number=run_number,
                run_attempt=run_attempt,
                status=status,
                conclusion=conclusion,
                created_at=created_at,
                updated_at=updated_at,
                pull_requests=pull_requests,
            )
        ]
    )


def different_pull_request_run_fixture() -> dict[str, Any]:
    run = workflow_run_fixture(run_id=987655, run_number=85)
    run["head_branch"] = "other/release"
    for reference in run["referenced_workflows"]:
        reference["ref"] = "refs/heads/other/release"
    return run


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

    def assert_accepted(
        self,
        event: Any,
        pull_request: Any,
        runs: Any,
        expected_run_id: int,
    ) -> None:
        result = self.run_resolver(event, pull_request, runs)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, f"{expected_run_id}\n")

    def test_accepts_single_success_and_ignores_pull_requests(self) -> None:
        for pull_requests in ([], [{"number": 999, "malformed": True}]):
            with self.subTest(pull_requests=pull_requests):
                self.assert_accepted(
                    event_fixture(),
                    pull_request_fixture(),
                    runs_fixture(pull_requests),
                    987654,
                )

    def test_ignores_top_level_run_ref(self) -> None:
        for top_level_ref in (None, "refs/heads/not-the-pull-request-ref"):
            with self.subTest(top_level_ref=top_level_ref):
                runs = runs_fixture()
                runs["workflow_runs"][0]["ref"] = top_level_ref
                self.assert_accepted(
                    event_fixture(), pull_request_fixture(), runs, 987654
                )

    def test_accepts_latest_success_for_two_runs_in_input_order(self) -> None:
        older = workflow_run_fixture(
            run_id=987653,
            run_number=83,
            created_at=RUN_83_CREATED_AT,
            updated_at=RUN_83_UPDATED_AT,
        )
        latest = workflow_run_fixture(
            run_id=987654,
            run_number=84,
            created_at=RUN_84_CREATED_AT,
            updated_at=RUN_84_UPDATED_AT,
        )
        for name, ordered_runs in (
            ("older before latest", [older, latest]),
            ("latest before older", [latest, older]),
        ):
            with self.subTest(name=name):
                self.assert_accepted(
                    event_fixture(),
                    pull_request_fixture(),
                    runs_response(ordered_runs),
                    987654,
                )

    def test_accepts_latest_success_across_paginated_responses(self) -> None:
        older = workflow_run_fixture(
            run_id=987653,
            run_number=83,
            created_at=RUN_83_CREATED_AT,
            updated_at=RUN_83_UPDATED_AT,
        )
        latest = workflow_run_fixture(
            run_id=987654,
            run_number=84,
            created_at=RUN_84_CREATED_AT,
            updated_at=RUN_84_UPDATED_AT,
        )
        pages = [
            runs_response([older], total_count=2),
            runs_response([latest], total_count=2),
        ]
        self.assert_accepted(
            event_fixture(), pull_request_fixture(), pages, 987654
        )

    def test_rejects_zero_exact_matches(self) -> None:
        self.assert_rejected(
            event_fixture(), pull_request_fixture(), runs_response([])
        )

    def test_ignores_different_pull_request_success_in_either_order(self) -> None:
        exact = runs_fixture()["workflow_runs"][0]
        different = different_pull_request_run_fixture()
        for name, ordered_runs in (
            ("different before exact", [different, exact]),
            ("exact before different", [exact, different]),
        ):
            with self.subTest(name=name):
                result = self.run_resolver(
                    event_fixture(),
                    pull_request_fixture(),
                    runs_response(ordered_runs),
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "987654\n")

    def test_ignores_unrelated_success_with_invalid_id(self) -> None:
        unrelated = different_pull_request_run_fixture()
        unrelated["id"] = 0
        exact = runs_fixture()["workflow_runs"][0]
        for ordered_runs in ([unrelated, exact], [exact, unrelated]):
            with self.subTest(ordered_runs=ordered_runs):
                result = self.run_resolver(
                    event_fixture(),
                    pull_request_fixture(),
                    runs_response(ordered_runs),
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "987654\n")

    def test_rejects_zero_exact_matches_with_only_unrelated_run(self) -> None:
        unrelated = different_pull_request_run_fixture()
        self.assert_rejected(
            event_fixture(), pull_request_fixture(), runs_response([unrelated])
        )

    def test_rejects_malformed_successful_run_with_exact_match(self) -> None:
        malformed = runs_fixture()["workflow_runs"][0]
        malformed["head_commit"].pop("id")
        self.assert_rejected(
            event_fixture(),
            pull_request_fixture(),
            runs_response([malformed]),
        )

    def test_rejects_latest_non_success_even_with_older_success(self) -> None:
        older = workflow_run_fixture(
            run_id=987653,
            run_number=83,
            created_at=RUN_83_CREATED_AT,
            updated_at=RUN_83_UPDATED_AT,
        )
        for status, conclusion in (
            ("completed", "failure"),
            ("completed", "cancelled"),
            ("queued", None),
            ("in_progress", None),
        ):
            latest = workflow_run_fixture(
                run_id=987654,
                run_number=84,
                status=status,
                conclusion=conclusion,
                created_at=RUN_84_CREATED_AT,
                updated_at=RUN_84_UPDATED_AT,
            )
            for name, ordered_runs in (
                ("older before latest", [older, latest]),
                ("latest before older", [latest, older]),
            ):
                with self.subTest(status=status, name=name):
                    self.assert_rejected(
                        event_fixture(),
                        pull_request_fixture(),
                        runs_response(ordered_runs),
                    )

    def test_rejects_maximum_run_number_tie(self) -> None:
        first = workflow_run_fixture(
            run_id=987654, run_number=84, run_attempt=1
        )
        second = workflow_run_fixture(
            run_id=987655, run_number=84, run_attempt=2
        )
        self.assert_rejected(
            event_fixture(),
            pull_request_fixture(),
            runs_response([first, second]),
        )

    def test_rejects_postmerge_latest_success_with_older_success(self) -> None:
        older = workflow_run_fixture(
            run_id=987653,
            run_number=83,
            created_at="2026-08-28T23:58:00Z",
            updated_at="2026-08-28T23:59:00Z",
        )
        latest = workflow_run_fixture(
            run_id=987654,
            run_number=84,
            status="completed",
            conclusion="success",
            created_at="2026-08-29T00:06:00Z",
            updated_at="2026-08-29T00:07:00Z",
        )
        self.assert_rejected(
            event_fixture(),
            pull_request_fixture(),
            runs_response([older, latest]),
        )

    def test_rejects_created_after_updated(self) -> None:
        self.assert_rejected(
            event_fixture(),
            pull_request_fixture(),
            runs_fixture(
                created_at=RUN_84_UPDATED_AT,
                updated_at=RUN_84_CREATED_AT,
            ),
        )

    def test_accepts_run_at_pr_merge_boundary(self) -> None:
        self.assert_accepted(
            event_fixture(),
            pull_request_fixture(),
            runs_fixture(
                created_at=MERGED_AT,
                updated_at=MERGED_AT,
            ),
            987654,
        )

    def test_rejects_missing_or_malformed_run_timestamps(self) -> None:
        cases: dict[str, Callable[[dict[str, Any]], None]] = {
            "created_at missing": lambda run: run.pop("created_at"),
            "updated_at missing": lambda run: run.pop("updated_at"),
            "created_at malformed": lambda run: run.update(
                created_at="not-a-timestamp"
            ),
            "updated_at malformed": lambda run: run.update(
                updated_at="not-a-timestamp"
            ),
            "created_at null": lambda run: run.update(created_at=None),
            "updated_at null": lambda run: run.update(updated_at=None),
        }
        for name, change in cases.items():
            with self.subTest(name=name):
                runs = runs_fixture()
                change(runs["workflow_runs"][0])
                self.assert_rejected(
                    event_fixture(), pull_request_fixture(), runs
                )

    def test_rejects_total_count_mismatch(self) -> None:
        self.assert_rejected(
            event_fixture(),
            pull_request_fixture(),
            runs_response([workflow_run_fixture()], total_count=2),
        )
        self.assert_rejected(
            event_fixture(),
            pull_request_fixture(),
            [
                runs_response([workflow_run_fixture(run_id=987653)], total_count=2),
                runs_response([workflow_run_fixture(run_id=987654)], total_count=3),
            ],
        )

    def test_rejects_non_dict_workflow_run(self) -> None:
        self.assert_rejected(
            event_fixture(), pull_request_fixture(), runs_response([None])
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
            "event merged_at mismatch": (
                mutate(
                    event_fixture(),
                    lambda value: value["pull_request"].update(
                        merged_at="2026-08-29T00:00:01Z"
                    ),
                ),
                pull_request_fixture(),
            ),
            "event merged_at missing": (
                mutate(
                    event_fixture(),
                    lambda value: value["pull_request"].pop("merged_at"),
                ),
                pull_request_fixture(),
            ),
            "event merged_at malformed": (
                mutate(
                    event_fixture(),
                    lambda value: value["pull_request"].update(
                        merged_at="not-a-timestamp"
                    ),
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
            "PR merged_at mismatch": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value.update(
                        merged_at="2026-08-29T00:00:01Z"
                    ),
                ),
            ),
            "PR merged_at missing": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value.pop("merged_at"),
                ),
            ),
            "PR merged_at malformed": (
                event_fixture(),
                mutate(
                    pull_request_fixture(),
                    lambda value: value.update(merged_at="not-a-timestamp"),
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
            "referenced workflows one": lambda run: run.update(
                referenced_workflows=[run["referenced_workflows"][0]]
            ),
            "referenced workflows duplicate": lambda run: run.update(
                referenced_workflows=[
                    run["referenced_workflows"][0],
                    copy.deepcopy(run["referenced_workflows"][0]),
                ]
            ),
            "referenced workflows three": lambda run: run["referenced_workflows"].append(
                copy.deepcopy(run["referenced_workflows"][0])
            ),
            "referenced workflow branch": lambda run: run[
                "referenced_workflows"
            ][0].update(ref="refs/heads/other/release"),
            "referenced workflow repository": lambda run: run[
                "referenced_workflows"
            ][0].update(
                path="other/repository/.github/workflows/rust.yml@" + HEAD_SHA
            ),
            "referenced workflow path": lambda run: run[
                "referenced_workflows"
            ][0].update(
                path=BASE_REPOSITORY["full_name"]
                + "/.github/workflows/other.yml@"
                + HEAD_SHA
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
            "CodeQL referenced workflow branch": lambda run: run[
                "referenced_workflows"
            ][1].update(ref="refs/heads/other/release"),
            "CodeQL referenced workflow path": lambda run: run[
                "referenced_workflows"
            ][1].update(
                path=BASE_REPOSITORY["full_name"]
                + "/.github/workflows/other.yml@"
                + HEAD_SHA
            ),
            "CodeQL referenced workflow SHA mismatch": lambda run: run[
                "referenced_workflows"
            ][1].update(
                path=BASE_REPOSITORY["full_name"]
                + "/.github/workflows/codeql.yml@"
                + "d" * 40,
                sha="d" * 40,
            ),
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

    def test_rejects_invalid_or_missing_run_numbers_and_attempts(self) -> None:
        cases: dict[str, Callable[[dict[str, Any]], None]] = {
            "run_number zero": lambda run: run.update(run_number=0),
            "run_number negative": lambda run: run.update(run_number=-1),
            "run_number bool": lambda run: run.update(run_number=True),
            "run_number string": lambda run: run.update(run_number="84"),
            "run_number missing": lambda run: run.pop("run_number"),
            "run_attempt zero": lambda run: run.update(run_attempt=0),
            "run_attempt negative": lambda run: run.update(run_attempt=-1),
            "run_attempt bool": lambda run: run.update(run_attempt=True),
            "run_attempt string": lambda run: run.update(run_attempt="1"),
            "run_attempt missing": lambda run: run.pop("run_attempt"),
        }
        for name, change in cases.items():
            with self.subTest(name=name):
                runs = runs_fixture()
                change(runs["workflow_runs"][0])
                self.assert_rejected(
                    event_fixture(), pull_request_fixture(), runs
                )


if __name__ == "__main__":
    unittest.main()

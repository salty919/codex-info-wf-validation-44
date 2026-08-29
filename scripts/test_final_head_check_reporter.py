#!/usr/bin/env python3
"""Deterministic tests for final-head required-check publication."""

from __future__ import annotations

import copy
import hashlib
import tempfile
import unittest
from pathlib import Path
from typing import Any

import final_head_check_reporter as reporter

REPOSITORY = "salty919/codex_info_v2"
BASE_SHA = "1" * 40
HEAD_SHA = "2" * 40
RUN_ID = 24680


def identity() -> reporter.Identity:
    return reporter.Identity(
        repository=REPOSITORY,
        pr_number=42,
        base_repository=REPOSITORY,
        head_repository=REPOSITORY,
        base_sha=BASE_SHA,
        head_sha=HEAD_SHA,
        head_ref="feat/next",
        run_id=RUN_ID,
        run_url=f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}",
    )


def pull_request(*, head_sha: str = HEAD_SHA) -> dict[str, Any]:
    return {
        "number": 42,
        "state": "open",
        "base": {
            "repo": {"full_name": REPOSITORY},
            "ref": "main",
            "sha": BASE_SHA,
        },
        "head": {
            "repo": {"full_name": REPOSITORY},
            "ref": "feat/next",
            "sha": head_sha,
        },
    }


def check_run(
    name: str,
    check_id: int,
    *,
    status: str = "in_progress",
    conclusion: str | None = None,
    run_id: int = RUN_ID,
) -> dict[str, Any]:
    return {
        "id": check_id,
        "name": name,
        "head_sha": HEAD_SHA,
        "external_id": (f"codex-quality-v1:pr=42:head={HEAD_SHA}:run={run_id}"),
        "status": status,
        "conclusion": conclusion,
        "app": {"id": reporter.GITHUB_ACTIONS_APP_ID},
    }


class FakeApi:
    def __init__(self) -> None:
        self.pull_request = pull_request()
        self.checks: dict[str, list[dict[str, Any]]] = {
            "version-prepared": [],
            "acceptance": [],
        }
        self.active_runs: set[int] = set()
        self.calls: list[tuple[str, str, dict[str, Any] | None]] = []
        self.next_id = 100

    def request(
        self, method: str, path: str, payload: dict[str, Any] | None = None
    ) -> tuple[int, Any]:
        self.calls.append((method, path, copy.deepcopy(payload)))
        if method == "GET" and path == f"/repos/{REPOSITORY}/pulls/42":
            return 200, copy.deepcopy(self.pull_request)
        if method == "GET" and "/commits/" in path and "/check-runs?" in path:
            name = path.split("check_name=", 1)[1].split("&", 1)[0]
            checks = copy.deepcopy(self.checks[name])
            return 200, {"total_count": len(checks), "check_runs": checks}
        if method == "GET" and "/actions/runs/" in path:
            run_id = int(path.rsplit("/", 1)[1])
            return 200, {
                "repository": {"full_name": REPOSITORY},
                "status": "in_progress" if run_id in self.active_runs else "completed",
            }
        if method == "POST" and path == f"/repos/{REPOSITORY}/check-runs":
            assert payload is not None
            created = self._response(payload, self.next_id)
            self.next_id += 1
            self.checks[created["name"]] = [created]
            return 201, copy.deepcopy(created)
        if method == "PATCH" and "/check-runs/" in path:
            assert payload is not None
            check_id = int(path.rsplit("/", 1)[1])
            updated = self._response(payload, check_id)
            self.checks[updated["name"]] = [updated]
            return 200, copy.deepcopy(updated)
        raise AssertionError(f"unexpected API request: {method} {path}")

    @staticmethod
    def _response(payload: dict[str, Any], check_id: int) -> dict[str, Any]:
        return {
            "id": check_id,
            "name": payload["name"],
            "head_sha": payload["head_sha"],
            "external_id": payload["external_id"],
            "status": payload["status"],
            "conclusion": payload.get("conclusion"),
            "app": {"id": reporter.GITHUB_ACTIONS_APP_ID},
        }


def write_evidence(directory: Path, fields: dict[str, str]) -> None:
    directory.mkdir(parents=True)
    evidence = directory / "acceptance.txt"
    evidence.write_text(
        "".join(f"{key}: {value}\n" for key, value in fields.items()),
        encoding="utf-8",
    )
    digest = hashlib.sha256(evidence.read_bytes()).hexdigest()
    (directory / "SHA256SUMS").write_text(
        f"{digest}  acceptance.txt\n", encoding="utf-8"
    )


class FinalHeadCheckReporterTests(unittest.TestCase):
    def test_register_creates_exact_required_checks_on_final_head(self) -> None:
        api = FakeApi()
        result = reporter.register_checks(api, identity())

        self.assertTrue(result.quality_required)
        self.assertEqual(set(api.checks), set(reporter.CHECK_NAMES))
        self.assertEqual(api.checks["version-prepared"][0]["conclusion"], "success")
        self.assertEqual(api.checks["acceptance"][0]["status"], "in_progress")
        for name in reporter.CHECK_NAMES:
            self.assertEqual(api.checks[name][0]["head_sha"], HEAD_SHA)
            self.assertEqual(api.checks[name][0]["external_id"], identity().external_id)

    def test_register_reuses_successful_acceptance_without_quality_rerun(self) -> None:
        api = FakeApi()
        api.checks["version-prepared"] = [
            check_run(
                "version-prepared",
                10,
                status="completed",
                conclusion="success",
                run_id=13579,
            )
        ]
        api.checks["acceptance"] = [
            check_run(
                "acceptance",
                11,
                status="completed",
                conclusion="success",
                run_id=13579,
            )
        ]

        result = reporter.register_checks(api, identity())

        self.assertFalse(result.quality_required)
        self.assertEqual(
            api.checks["acceptance"][0]["external_id"].rsplit("=", 1)[1], "13579"
        )
        acceptance_mutations = [
            call
            for call in api.calls
            if call[0] in {"POST", "PATCH"}
            and call[2] is not None
            and call[2].get("name") == "acceptance"
        ]
        self.assertEqual(acceptance_mutations, [])

    def test_register_rejects_parallel_owner_and_duplicate_check(self) -> None:
        for duplicate in (False, True):
            with self.subTest(duplicate=duplicate):
                api = FakeApi()
                prior = check_run("acceptance", 11, run_id=13579)
                api.checks["acceptance"] = (
                    [prior, copy.deepcopy(prior)] if duplicate else [prior]
                )
                api.active_runs.add(13579)
                with self.assertRaises(reporter.ReporterError):
                    reporter.register_checks(api, identity())

    def test_register_rejects_active_version_owner_without_acceptance(self) -> None:
        api = FakeApi()
        api.checks["version-prepared"] = [
            check_run("version-prepared", 10, run_id=13579)
        ]
        api.active_runs.add(13579)
        with self.assertRaisesRegex(reporter.ReporterError, "another active run"):
            reporter.register_checks(api, identity())

    def test_register_rejects_foreign_app_check_with_required_name(self) -> None:
        api = FakeApi()
        foreign = check_run("acceptance", 11)
        foreign["app"] = {"id": 999}
        api.checks["acceptance"] = [foreign]
        with self.assertRaisesRegex(reporter.ReporterError, "non-Actions App"):
            reporter.register_checks(api, identity())

    def test_register_rejects_successful_pair_with_different_owners(self) -> None:
        api = FakeApi()
        api.checks["version-prepared"] = [
            check_run(
                "version-prepared",
                10,
                status="completed",
                conclusion="success",
                run_id=13579,
            )
        ]
        api.checks["acceptance"] = [
            check_run(
                "acceptance",
                11,
                status="completed",
                conclusion="success",
                run_id=13580,
            )
        ]
        with self.assertRaisesRegex(reporter.ReporterError, "share one owner"):
            reporter.register_checks(api, identity())

    def test_register_rejects_moved_pull_request(self) -> None:
        api = FakeApi()
        api.pull_request = pull_request(head_sha="3" * 40)
        with self.assertRaisesRegex(
            reporter.ReporterError, "live pull-request identity"
        ):
            reporter.register_checks(api, identity())

    def test_finalize_accepts_bound_artifacts_and_completes_acceptance(self) -> None:
        api = FakeApi()
        api.checks["version-prepared"] = [
            check_run("version-prepared", 10, status="completed", conclusion="success")
        ]
        api.checks["acceptance"] = [check_run("acceptance", 11)]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            verdict = root / "verdict"
            candidate = root / "candidate"
            write_evidence(
                verdict,
                {
                    "schema": "codex-info-final-head-v1",
                    "pr-number": "42",
                    "source-sha": HEAD_SHA,
                    "binary-impact": "true",
                    "version": "1.2.3",
                    "acceptance": "PASS",
                },
            )
            write_evidence(
                candidate,
                {
                    "schema": "codex-info-quality-v1",
                    "pr-number": "42",
                    "source-sha": HEAD_SHA,
                    "tree-sha": "3" * 40,
                    "version": "1.2.3",
                    "acceptance": "PASS",
                },
            )
            reporter.finalize_acceptance(
                api,
                identity(),
                quality_result="success",
                binary_impact="true",
                version="1.2.3",
                verdict_directory=verdict,
                candidate_directory=candidate,
                verdict_artifact_id="500",
                verdict_artifact_digest="a" * 64,
                candidate_artifact_id="501",
                candidate_artifact_digest="b" * 64,
            )

        self.assertEqual(api.checks["acceptance"][0]["conclusion"], "success")

    def test_finalize_records_failure_for_malformed_artifact_digest(self) -> None:
        api = FakeApi()
        api.checks["version-prepared"] = [
            check_run("version-prepared", 10, status="completed", conclusion="success")
        ]
        api.checks["acceptance"] = [check_run("acceptance", 11)]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            verdict = root / "verdict"
            candidate = root / "candidate"
            write_evidence(
                verdict,
                {
                    "schema": "codex-info-final-head-v1",
                    "pr-number": "42",
                    "source-sha": HEAD_SHA,
                    "binary-impact": "false",
                    "version": "",
                    "acceptance": "PASS",
                },
            )
            with self.assertRaisesRegex(reporter.ReporterError, "artifact identity"):
                reporter.finalize_acceptance(
                    api,
                    identity(),
                    quality_result="success",
                    binary_impact="false",
                    version="",
                    verdict_directory=verdict,
                    candidate_directory=candidate,
                    verdict_artifact_id="500",
                    verdict_artifact_digest="a" * 40,
                    candidate_artifact_id="",
                    candidate_artifact_digest="",
                )

        self.assertEqual(api.checks["acceptance"][0]["conclusion"], "failure")

    def test_finalize_never_overwrites_another_run_owner(self) -> None:
        api = FakeApi()
        api.checks["version-prepared"] = [
            check_run(
                "version-prepared",
                10,
                status="completed",
                conclusion="success",
                run_id=13579,
            )
        ]
        api.checks["acceptance"] = [check_run("acceptance", 11, run_id=13579)]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(reporter.ReporterError, "another workflow run"):
                reporter.finalize_acceptance(
                    api,
                    identity(),
                    quality_result="failure",
                    binary_impact="true",
                    version="1.2.3",
                    verdict_directory=root / "verdict",
                    candidate_directory=root / "candidate",
                    verdict_artifact_id="",
                    verdict_artifact_digest="",
                    candidate_artifact_id="",
                    candidate_artifact_digest="",
                )
        self.assertFalse(any(method == "PATCH" for method, _path, _body in api.calls))

    def test_mutation_readback_requires_exact_terminal_state(self) -> None:
        api = FakeApi()
        mutated = check_run(
            "version-prepared", 10, status="completed", conclusion="success"
        )
        stale = copy.deepcopy(mutated)
        stale["conclusion"] = "failure"
        api.checks["version-prepared"] = [stale]
        with self.assertRaisesRegex(reporter.ReporterError, "state is not exact"):
            reporter._assert_unique_mutation(
                api,
                identity(),
                "version-prepared",
                mutated,
                expected_status="completed",
                expected_conclusion="success",
            )


if __name__ == "__main__":
    unittest.main()

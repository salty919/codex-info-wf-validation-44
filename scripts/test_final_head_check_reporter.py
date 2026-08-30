#!/usr/bin/env python3
"""Deterministic tests for final-head required-check publication."""

from __future__ import annotations

import copy
import hashlib
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from typing import Any

import final_head_check_reporter as reporter

REPOSITORY = "salty919/codex_info_v2"
BASE_SHA = "1" * 40
HEAD_SHA = "2" * 40
RUN_ID = 24680
HEAD_REF = "user/main-change"


def identity() -> reporter.Identity:
    return reporter.Identity(
        repository=REPOSITORY,
        pr_number=42,
        base_repository=REPOSITORY,
        head_repository=REPOSITORY,
        base_sha=BASE_SHA,
        head_sha=HEAD_SHA,
        head_ref=HEAD_REF,
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
            "ref": HEAD_REF,
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

    def test_same_repository_main_pr_accepts_arbitrary_nonempty_head_refs(self) -> None:
        for head_ref in ("feat/next", "fix/windows-only", "release_2026.08"):
            with self.subTest(head_ref=head_ref):
                value = replace(identity(), head_ref=head_ref)
                api = FakeApi()
                api.pull_request["head"]["ref"] = head_ref
                result = reporter.register_checks(api, value)
                self.assertTrue(result.quality_required)
        for head_ref in ("", "bad\x00ref"):
            with self.subTest(head_ref=head_ref), self.assertRaises(
                reporter.ReporterError
            ):
                reporter.validate_identity(replace(identity(), head_ref=head_ref))

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
                    "windows-impact": "true",
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
                windows_impact="true",
                version="1.2.3",
                verdict_directory=verdict,
                candidate_directory=candidate,
                verdict_artifact_id="500",
                verdict_artifact_digest="a" * 64,
                candidate_artifact_id="501",
                candidate_artifact_digest="b" * 64,
            )

        self.assertEqual(api.checks["acceptance"][0]["conclusion"], "success")

    def test_artifact_contract_covers_all_valid_impact_combinations(self) -> None:
        cases = (
            ("false", "false", "", False, True),
            ("true", "false", "1.2.3", False, True),
            ("true", "true", "1.2.3", True, True),
            ("false", "true", "", False, False),
        )
        for binary_impact, windows_impact, version, with_candidate, valid in cases:
            with self.subTest(
                binary_impact=binary_impact, windows_impact=windows_impact
            ), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                verdict = root / "verdict"
                candidate = root / "candidate"
                write_evidence(
                    verdict,
                    {
                        "schema": "codex-info-final-head-v1",
                        "pr-number": "42",
                        "source-sha": HEAD_SHA,
                        "binary-impact": binary_impact,
                        "windows-impact": windows_impact,
                        "version": version,
                        "acceptance": "PASS",
                    },
                )
                if with_candidate:
                    write_evidence(
                        candidate,
                        {
                            "schema": "codex-info-quality-v1",
                            "pr-number": "42",
                            "source-sha": HEAD_SHA,
                            "tree-sha": "3" * 40,
                            "version": version,
                            "acceptance": "PASS",
                        },
                    )
                if valid:
                    digest = reporter._verify_artifacts(
                        identity(),
                        binary_impact=binary_impact,
                        windows_impact=windows_impact,
                        version=version,
                        verdict_directory=verdict,
                        candidate_directory=candidate,
                    )
                    self.assertRegex(digest, r"^[0-9a-f]{64}$")
                else:
                    with self.assertRaises(reporter.ReporterError):
                        reporter._verify_artifacts(
                            identity(),
                            binary_impact=binary_impact,
                            windows_impact=windows_impact,
                            version=version,
                            verdict_directory=verdict,
                            candidate_directory=candidate,
                        )

    def test_verify_verdict_cli_writes_release_decision_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            verdict = root / "verdict"
            output = root / "github-output"
            write_evidence(
                verdict,
                {
                    "schema": "codex-info-final-head-v1",
                    "pr-number": "42",
                    "source-sha": HEAD_SHA,
                    "binary-impact": "true",
                    "windows-impact": "false",
                    "version": "1.2.3",
                    "acceptance": "PASS",
                },
            )
            self.assertEqual(
                reporter.main(
                    [
                        "verify-verdict",
                        "--pr-number",
                        "42",
                        "--head-sha",
                        HEAD_SHA,
                        "--verdict-directory",
                        str(verdict),
                        "--github-output",
                        str(output),
                    ]
                ),
                0,
            )
            self.assertEqual(
                output.read_text(encoding="utf-8").splitlines(),
                [
                    "binary_impact=true",
                    "windows_impact=false",
                    "version=1.2.3",
                ],
            )

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
                    "windows-impact": "false",
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
                    windows_impact="false",
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
                    windows_impact="true",
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
                attempts=1,
                sleep=lambda _seconds: None,
            )

    def test_mutation_readback_retries_a_stale_single_result(self) -> None:
        exact = check_run(
            "version-prepared", 10, status="completed", conclusion="success"
        )
        stale = copy.deepcopy(exact)
        stale["conclusion"] = "failure"

        class DelayedReadBackApi(FakeApi):
            def __init__(self) -> None:
                super().__init__()
                self.responses = [[stale], [stale], [exact]]

            def request(
                self,
                method: str,
                path: str,
                payload: dict[str, Any] | None = None,
            ) -> tuple[int, Any]:
                if method == "GET" and "/check-runs?" in path:
                    checks = self.responses.pop(0)
                    return 200, {
                        "total_count": len(checks),
                        "check_runs": copy.deepcopy(checks),
                    }
                return super().request(method, path, payload)

        sleeps: list[float] = []
        reporter._assert_unique_mutation(
            DelayedReadBackApi(),
            identity(),
            "version-prepared",
            exact,
            expected_status="completed",
            expected_conclusion="success",
            attempts=3,
            sleep=sleeps.append,
        )
        self.assertEqual(sleeps, [reporter.READBACK_DELAY_SECONDS] * 2)


TRANSITION_NEW_SHA = "3" * 40


def transition_identity() -> reporter.VersionTransition:
    return reporter.VersionTransition(
        repository=REPOSITORY,
        pr_number=42,
        base_repository=REPOSITORY,
        base_sha=BASE_SHA,
        head_repository=REPOSITORY,
        head_ref=HEAD_REF,
        old_head_sha=HEAD_SHA,
        new_head_sha=TRANSITION_NEW_SHA,
        run_id=RUN_ID,
        run_url=f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}",
    )


class TransitionFakeApi:
    def __init__(
        self,
        *,
        updates: list[str] | None = None,
        check_delay: int = 0,
        projection_delay: int = 0,
    ) -> None:
        self.ref_sha = HEAD_SHA
        self.updates = list(updates or ["success"])
        self.check_delay = check_delay
        self.projection_delay = projection_delay
        self.created_check: dict[str, Any] | None = None
        self.duplicate_check = False
        self.malformed_check_page = False
        self.pull_override: dict[str, Any] | None = None
        self.calls: list[tuple[str, str, dict[str, Any] | None]] = []

    def request(
        self, method: str, path: str, payload: dict[str, Any] | None = None
    ) -> tuple[int, Any]:
        self.calls.append((method, path, copy.deepcopy(payload)))
        if method == "GET" and "/git/ref/heads/" in path:
            return 200, {"object": {"sha": self.ref_sha}}
        if method == "PATCH" and "/git/refs/heads/" in path:
            outcome = self.updates.pop(0) if self.updates else "success"
            if outcome == "success":
                self.ref_sha = TRANSITION_NEW_SHA
                return 200, {"object": {"sha": TRANSITION_NEW_SHA}}
            if outcome == "wrong-success":
                return 200, {"object": {"sha": "4" * 40}}
            if outcome == "rule":
                return 422, {
                    "message": (
                        "Repository rule violations found\n\n"
                        f"{reporter.TRANSITION_RULE_ERROR}"
                    )
                }
            return 422, {"message": "Update is not a fast forward"}
        if method == "POST" and path == f"/repos/{REPOSITORY}/check-runs":
            assert payload is not None
            self.created_check = {
                "id": 700,
                "name": payload["name"],
                "head_sha": payload["head_sha"],
                "external_id": payload["external_id"],
                "status": payload["status"],
                "conclusion": payload["conclusion"],
                "app": {"id": reporter.GITHUB_ACTIONS_APP_ID},
            }
            return 201, copy.deepcopy(self.created_check)
        if method == "GET" and "/check-runs?" in path:
            if self.malformed_check_page:
                return 200, {"total_count": 2, "check_runs": []}
            if self.check_delay > 0:
                self.check_delay -= 1
                checks: list[dict[str, Any]] = []
            elif self.created_check is None:
                checks = []
            else:
                checks = [copy.deepcopy(self.created_check)]
                if self.duplicate_check:
                    duplicate = copy.deepcopy(self.created_check)
                    duplicate["id"] = 701
                    checks.append(duplicate)
            return 200, {"total_count": len(checks), "check_runs": checks}
        if method == "GET" and path == f"/repos/{REPOSITORY}/pulls/42":
            if self.pull_override is not None:
                return 200, copy.deepcopy(self.pull_override)
            if self.projection_delay > 0:
                self.projection_delay -= 1
                head_sha = HEAD_SHA
            else:
                head_sha = TRANSITION_NEW_SHA
            return 200, pull_request(head_sha=head_sha)
        raise AssertionError(f"unexpected API request: {method} {path}")


def run_local_transition_tests() -> unittest.result.TestResult:
    """Run the finite migration matrix only when explicitly requested locally."""

    class LocalTransitionTests(unittest.TestCase):
        def publish(self, api: TransitionFakeApi, *, attempts: int = 4) -> list[float]:
            sleeps: list[float] = []
            reporter.publish_version_transition(
                api, transition_identity(), attempts=attempts, sleep=sleeps.append
            )
            return sleeps

        def test_unprotected_ref_updates_without_transition_check(self) -> None:
            api = TransitionFakeApi()
            self.assertEqual(self.publish(api), [])
            self.assertEqual(api.ref_sha, TRANSITION_NEW_SHA)
            self.assertIsNone(api.created_check)
            self.assertEqual(
                sum(method == "PATCH" for method, _path, _body in api.calls), 1
            )

        def test_rule_rejection_waits_for_check_and_projection(self) -> None:
            api = TransitionFakeApi(
                updates=["rule", "rule", "success"],
                check_delay=1,
                projection_delay=2,
            )
            self.assertGreaterEqual(len(self.publish(api)), 4)
            self.assertEqual(
                sum(method == "POST" for method, _path, _body in api.calls), 1
            )
            self.assertEqual(
                sum(method == "PATCH" for method, _path, _body in api.calls), 3
            )

        def test_existing_new_ref_waits_for_projection(self) -> None:
            api = TransitionFakeApi(projection_delay=1)
            api.ref_sha = TRANSITION_NEW_SHA
            self.assertEqual(self.publish(api), [1])
            self.assertFalse(
                any(method == "PATCH" for method, _path, _body in api.calls)
            )

        def test_unexpected_rejection_and_wrong_success_fail(self) -> None:
            api = TransitionFakeApi(updates=["unexpected"])
            with self.assertRaisesRegex(reporter.ReporterError, "unexpected HTTP 422"):
                self.publish(api)
            self.assertIsNone(api.created_check)
            with self.assertRaisesRegex(reporter.ReporterError, "unexpected SHA"):
                self.publish(TransitionFakeApi(updates=["wrong-success"]))

        def test_visibility_timeout_does_not_retry_update(self) -> None:
            api = TransitionFakeApi(updates=["rule"], check_delay=20)
            with self.assertRaisesRegex(
                reporter.ReporterError, "did not become visible"
            ):
                self.publish(api, attempts=3)
            self.assertEqual(
                sum(method == "PATCH" for method, _path, _body in api.calls), 1
            )

        def test_duplicate_and_malformed_checks_fail(self) -> None:
            for malformed in (False, True):
                with self.subTest(malformed=malformed):
                    api = TransitionFakeApi(updates=["rule"])
                    api.duplicate_check = not malformed
                    api.malformed_check_page = malformed
                    with self.assertRaises(reporter.ReporterError):
                        self.publish(api)

        def test_ref_movement_before_and_after_update_fails(self) -> None:
            api = TransitionFakeApi()
            api.ref_sha = "4" * 40
            with self.assertRaisesRegex(reporter.ReporterError, "moved before"):
                self.publish(api)

            api = TransitionFakeApi(projection_delay=1)
            original_request = api.request

            def move_after_update(
                method: str, path: str, payload: dict[str, Any] | None = None
            ) -> tuple[int, Any]:
                result = original_request(method, path, payload)
                if method == "PATCH" and result[0] == 200:
                    api.ref_sha = "4" * 40
                return result

            api.request = move_after_update  # type: ignore[method-assign]
            with self.assertRaisesRegex(reporter.ReporterError, "moved after"):
                self.publish(api)

        def test_pull_identity_head_and_timeout_fail(self) -> None:
            changed_base = pull_request(head_sha=TRANSITION_NEW_SHA)
            changed_base["base"]["sha"] = "4" * 40
            changed_head = pull_request(head_sha="4" * 40)
            for payload, message in (
                (changed_base, "identity changed"),
                (changed_head, "unexpected SHA"),
            ):
                with self.subTest(message=message):
                    api = TransitionFakeApi()
                    api.pull_override = payload
                    with self.assertRaisesRegex(reporter.ReporterError, message):
                        self.publish(api)
            with self.assertRaisesRegex(
                reporter.ReporterError, "did not become visible"
            ):
                self.publish(TransitionFakeApi(projection_delay=20), attempts=3)

        def test_identity_accepts_arbitrary_same_repository_ref_only(self) -> None:
            for head_ref in ("feat/next", "feature/windows-fix", "release_2026.08"):
                reporter.validate_version_transition(
                    replace(transition_identity(), head_ref=head_ref)
                )
            for candidate in (
                replace(transition_identity(), head_ref=""),
                replace(transition_identity(), head_ref="bad\x00ref"),
                replace(transition_identity(), head_repository="fork/project"),
                replace(transition_identity(), old_head_sha=TRANSITION_NEW_SHA),
            ):
                with self.assertRaises(reporter.ReporterError):
                    reporter.validate_version_transition(candidate)

        def test_attempt_count_must_be_positive(self) -> None:
            with self.assertRaisesRegex(reporter.ReporterError, "attempts"):
                reporter.publish_version_transition(
                    TransitionFakeApi(),
                    transition_identity(),
                    attempts=0,
                    sleep=lambda _seconds: None,
                )

    suite = unittest.defaultTestLoader.loadTestsFromTestCase(LocalTransitionTests)
    return unittest.TextTestRunner(verbosity=1).run(suite)


if __name__ == "__main__":
    if sys.argv[1:] == ["--transition-self-test"]:
        result = run_local_transition_tests()
        print(
            f"version-transition-local: "
            f"{'PASS' if result.wasSuccessful() else 'FAIL'} cases={result.testsRun}"
        )
        raise SystemExit(0 if result.wasSuccessful() else 1)
    unittest.main()

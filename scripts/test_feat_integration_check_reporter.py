#!/usr/bin/env python3
"""Finite trust and race tests for feat integration check reporting."""

from __future__ import annotations

import copy
import unittest

from feat_integration_check_reporter import (
    APP_ID,
    CHECK_NAME,
    ReporterError,
    external_id,
    finalize,
    register,
)


REPOSITORY = "example/project"
PR = 51
BASE = "1" * 40
HEAD = "2" * 40
HEAD_REF = "codex/change"
URL = "https://github.com/example/project/actions/runs/123"


class FakeClient:
    def __init__(self) -> None:
        self.pr = {
            "number": PR,
            "state": "open",
            "base": {"ref": "feat/next", "sha": BASE, "repo": {"full_name": REPOSITORY}},
            "head": {"ref": "codex/change", "sha": HEAD, "repo": {"full_name": REPOSITORY}},
        }
        self.checks: list[dict] = []
        self.next_id = 900

    def get(self, endpoint: str):
        if endpoint == f"repos/{REPOSITORY}/pulls/{PR}":
            return copy.deepcopy(self.pr)
        if endpoint.startswith(f"repos/{REPOSITORY}/commits/{HEAD}/check-runs?"):
            return {"total_count": len(self.checks), "check_runs": copy.deepcopy(self.checks)}
        raise AssertionError(endpoint)

    def post(self, endpoint: str, payload: dict):
        self.assert_endpoint = endpoint
        check = dict(payload)
        check.update({"id": self.next_id, "app": {"id": APP_ID}})
        self.next_id += 1
        self.checks.append(check)
        return copy.deepcopy(check)

    def patch(self, endpoint: str, payload: dict):
        check_id = int(endpoint.rsplit("/", 1)[1])
        for check in self.checks:
            if check["id"] == check_id:
                check.update(payload)
                return copy.deepcopy(check)
        raise ReporterError("unknown check")


def do_register(client: FakeClient) -> int:
    return register(
        client,
        repository=REPOSITORY,
        pr_number=PR,
        base_sha=BASE,
        head_ref=HEAD_REF,
        head_sha=HEAD,
        run_url=URL,
    )


def do_finalize(client: FakeClient, check_id: int, result: str = "success") -> None:
    finalize(
        client,
        repository=REPOSITORY,
        pr_number=PR,
        base_sha=BASE,
        head_ref=HEAD_REF,
        head_sha=HEAD,
        run_url=URL,
        check_id=check_id,
        quality_result=result,
    )


class ReporterTests(unittest.TestCase):
    def test_register_then_finalize_exact_check(self) -> None:
        client = FakeClient()
        check_id = do_register(client)
        self.assertEqual(client.checks[0]["name"], CHECK_NAME)
        self.assertEqual(client.checks[0]["external_id"], external_id(PR, HEAD))
        do_finalize(client, check_id)
        self.assertEqual(client.checks[0]["status"], "completed")
        self.assertEqual(client.checks[0]["conclusion"], "success")

    def test_same_head_rerun_reuses_one_check(self) -> None:
        client = FakeClient()
        first = do_register(client)
        second = do_register(client)
        self.assertEqual(first, second)
        self.assertEqual(len(client.checks), 1)

    def test_foreign_app_duplicate_and_malformed_reserved_identity_fail(self) -> None:
        candidates = (
            {"id": 1, "name": CHECK_NAME, "app": {"id": 999}, "external_id": external_id(PR, HEAD)},
            {"id": 1, "name": CHECK_NAME, "app": {"id": APP_ID}, "external_id": "malformed"},
        )
        for candidate in candidates:
            client = FakeClient()
            client.checks = [candidate]
            with self.subTest(candidate=candidate), self.assertRaises(ReporterError):
                do_register(client)

        client = FakeClient()
        check_id = do_register(client)
        client.checks.append(copy.deepcopy(client.checks[0]) | {"id": check_id + 1})
        with self.assertRaises(ReporterError):
            do_register(client)

    def test_moved_head_closed_pr_and_changed_base_fail_registration(self) -> None:
        mutations = (
            ("state", "closed"),
            ("head", {"ref": "codex/change", "sha": "3" * 40, "repo": {"full_name": REPOSITORY}}),
            ("head", {"ref": "codex/moved", "sha": HEAD, "repo": {"full_name": REPOSITORY}}),
            ("base", {"ref": "feat/next", "sha": "4" * 40, "repo": {"full_name": REPOSITORY}}),
        )
        for key, value in mutations:
            client = FakeClient()
            client.pr[key] = value
            with self.subTest(key=key), self.assertRaises(ReporterError):
                do_register(client)

    def test_quality_failure_cancel_skip_missing_and_head_move_finalize_failure(self) -> None:
        for result in ("failure", "cancelled", "skipped", ""):
            client = FakeClient()
            check_id = do_register(client)
            with self.subTest(result=result), self.assertRaises(ReporterError):
                do_finalize(client, check_id, result)
            self.assertEqual(client.checks[0]["conclusion"], "failure")
        client = FakeClient()
        check_id = do_register(client)
        client.pr["head"]["sha"] = "3" * 40
        with self.assertRaises(ReporterError):
            do_finalize(client, check_id)
        self.assertEqual(client.checks[0]["conclusion"], "failure")

    def test_wrong_check_id_or_parallel_owner_fails(self) -> None:
        client = FakeClient()
        check_id = do_register(client)
        with self.assertRaises(ReporterError):
            do_finalize(client, check_id + 1)
        client = FakeClient()
        check_id = do_register(client)
        client.checks.append(copy.deepcopy(client.checks[0]) | {"id": check_id + 1})
        with self.assertRaises(ReporterError):
            do_finalize(client, check_id)


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Regression tests for release resolution through final-head acceptance checks."""

from __future__ import annotations

import copy
import json
import os
import stat
import subprocess
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from typing import Any

import release_quality_run_resolver as resolver

REPOSITORY = "salty919/codex_info_v2"
PR_NUMBER = 42
H0_SHA = "2" * 40
H1_SHA = "3" * 40
BASE_SHA = "1" * 40
UNRELATED_SHA = "4" * 40
MERGE_SHA = "5" * 40
RUN_ID = 987654
CHECK_ID = 555
HEAD_REF = "fix/windows-release"
REPO_ROOT = Path(__file__).resolve().parents[1]


def pull_request() -> dict[str, Any]:
    return {
        "number": PR_NUMBER,
        "merged": True,
        "merged_at": "2026-08-29T12:30:00Z",
        "merge_commit_sha": MERGE_SHA,
        "head": {
            "sha": H1_SHA,
            "ref": HEAD_REF,
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
        "id": CHECK_ID,
        "name": "acceptance",
        "head_sha": H1_SHA,
        "status": "completed",
        "conclusion": "success",
        "external_id": (f"codex-quality-v1:pr={PR_NUMBER}:head={H1_SHA}:run={RUN_ID}"),
        # GitHub's observed custom check URL is keyed by check ID, not run ID.
        "details_url": f"https://github.com/{REPOSITORY}/runs/{CHECK_ID}",
        "app": {"id": resolver.GITHUB_ACTIONS_APP_ID},
    }


def native_acceptance_check(check_id: int, run_id: int) -> dict[str, Any]:
    """Model same-name native Actions job checks observed on failed PRs."""

    return {
        "id": check_id,
        "name": "acceptance",
        "head_sha": H1_SHA,
        "status": "completed",
        "conclusion": "skipped",
        "external_id": f"00000000-0000-0000-0000-{check_id:012d}",
        "details_url": (
            f"https://github.com/{REPOSITORY}/actions/runs/{run_id}/job/{check_id}"
        ),
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
        # The accepted run is from the PR source ref, not from main.
        "head_branch": HEAD_REF,
        "head_sha": H0_SHA,
        "repository": {"full_name": REPOSITORY},
        "status": "completed",
        "conclusion": "success",
        "html_url": f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}",
        "created_at": "2026-08-29T11:00:00Z",
        "updated_at": "2026-08-29T12:00:00Z",
    }


def head_commit(sha: str = H1_SHA, parent: str = H0_SHA) -> dict[str, Any]:
    return {
        "sha": sha,
        "parents": [{"sha": parent}],
    }


def _write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value), encoding="utf-8")


def _release_quality_run_shell_body() -> str:
    """Extract the actual run block from the release workflow, without retyping it."""

    workflow_path = REPO_ROOT / ".github/workflows/release.yml"
    lines = workflow_path.read_text(encoding="utf-8").splitlines()
    try:
        step_start = lines.index("      - name: Resolve accepted PR quality run")
    except ValueError as error:
        raise AssertionError("release quality-run step is missing") from error

    try:
        run_start = lines.index("        run: |", step_start + 1) + 1
    except ValueError as error:
        raise AssertionError("release quality-run shell body is missing") from error

    body: list[str] = []
    for line in lines[run_start:]:
        if line.startswith("      - name:"):
            break
        if not line:
            body.append("")
            continue
        if not line.startswith("          "):
            raise AssertionError(f"unexpected release shell indentation: {line!r}")
        body.append(line[10:])
    if not body:
        raise AssertionError("release quality-run shell body is empty")
    return "\n".join(body).rstrip() + "\n"


def _write_fake_gh(bin_dir: Path) -> None:
    fake_gh = bin_dir / "gh"
    fake_gh.write_text(
        """#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

fixture_dir = Path(os.environ["FAKE_GH_FIXTURE_DIR"])
endpoint = next((arg for arg in reversed(sys.argv[1:]) if "repos/" in arg), "")
if "/pulls/" in endpoint:
    name = "pull-request.json"
elif "/check-runs" in endpoint:
    name = "check-runs.json"
elif "/actions/runs/" in endpoint:
    name = "workflow-run.json"
elif "/commits/" in endpoint:
    name = "head-commit.json"
else:
    print(f"unexpected fake gh endpoint: {endpoint}", file=sys.stderr)
    raise SystemExit(2)
print(json.dumps(json.loads((fixture_dir / name).read_text(encoding="utf-8"))))
""",
        encoding="utf-8",
    )
    fake_gh.chmod(fake_gh.stat().st_mode | stat.S_IXUSR)


def _run_release_quality_run_step(
    *, head_commit_value: dict[str, Any] | None = None
) -> tuple[subprocess.CompletedProcess[str], str]:
    body = _release_quality_run_shell_body()
    with tempfile.TemporaryDirectory() as temporary:
        temporary_path = Path(temporary)
        fixture_dir = temporary_path / "fixtures"
        fixture_dir.mkdir()
        bin_dir = temporary_path / "bin"
        bin_dir.mkdir()
        _write_fake_gh(bin_dir)
        _write_json(fixture_dir / "event.json", event())
        _write_json(fixture_dir / "pull-request.json", pull_request())
        _write_json(fixture_dir / "check-runs.json", check_runs())
        _write_json(fixture_dir / "workflow-run.json", workflow_run())
        _write_json(
            fixture_dir / "head-commit.json",
            head_commit_value if head_commit_value is not None else head_commit(),
        )
        output_path = temporary_path / "github-output"
        output_path.touch()
        environment = os.environ.copy()
        environment.update(
            {
                "FAKE_GH_FIXTURE_DIR": str(fixture_dir),
                "GITHUB_EVENT_PATH": str(fixture_dir / "event.json"),
                "GITHUB_OUTPUT": str(output_path),
                "GH_TOKEN": "test-token",
                "PR_HEAD_SHA": H1_SHA,
                "PR_NUMBER": str(PR_NUMBER),
                "REPOSITORY": REPOSITORY,
                "RUNNER_TEMP": str(temporary_path),
                "PATH": f"{bin_dir}{os.pathsep}{environment['PATH']}",
            }
        )
        result = subprocess.run(
            ["bash", "-e", "-u", "-o", "pipefail", "-c", body],
            cwd=REPO_ROOT,
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )
        return result, output_path.read_text(encoding="utf-8")


class ReleaseQualityRunResolverTests(unittest.TestCase):
    def test_accepts_github_observed_authority_shape_h0_to_h1(self) -> None:
        self.assertEqual(
            resolver.resolve_quality_run_id(event(), pull_request(), check_runs()),
            RUN_ID,
        )
        self.assertEqual(
            resolver.verify_quality_run(
                event(), pull_request(), check_runs(), workflow_run(), head_commit()
            ),
            RUN_ID,
        )

    def test_accepts_final_head_when_h0_equals_h1(self) -> None:
        observed_run = workflow_run()
        observed_run["head_sha"] = H1_SHA
        self.assertEqual(
            resolver.verify_quality_run(
                event(),
                pull_request(),
                check_runs(),
                observed_run,
                head_commit(sha=H1_SHA, parent=BASE_SHA),
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

    def test_accepts_any_nonempty_same_repository_head_ref(self) -> None:
        for head_ref in ("feat/next", "fix/windows-only", "release_2026.08"):
            with self.subTest(head_ref=head_ref):
                changed_event = event()
                changed_pr = pull_request()
                changed_event["pull_request"]["head"]["ref"] = head_ref
                changed_pr["head"]["ref"] = head_ref
                changed_run = workflow_run()
                changed_run["head_branch"] = head_ref
                self.assertEqual(
                    resolver.verify_quality_run(
                        changed_event,
                        changed_pr,
                        check_runs(),
                        changed_run,
                        head_commit(),
                    ),
                    RUN_ID,
                )
        for head_ref in ("", "bad\x00ref"):
            with self.subTest(head_ref=head_ref):
                changed_event = event()
                changed_pr = pull_request()
                changed_event["pull_request"]["head"]["ref"] = head_ref
                changed_pr["head"]["ref"] = head_ref
                with self.assertRaisesRegex(resolver.ResolutionError, "invalid"):
                    resolver.resolve_quality_run_id(
                        changed_event, changed_pr, check_runs()
                    )

    def test_ignores_native_same_name_job_checks_from_premerge_and_closed_runs(
        self,
    ) -> None:
        observed_premerge = native_acceptance_check(99103608680, 33253681960)
        observed_closed = native_acceptance_check(99103726774, 33253725121)
        checks = check_runs(observed_closed, observed_premerge, acceptance_check())

        self.assertEqual(
            resolver.resolve_quality_run_id(event(), pull_request(), checks), RUN_ID
        )
        self.assertEqual(
            resolver.verify_quality_run(
                event(), pull_request(), checks, workflow_run(), head_commit()
            ),
            RUN_ID,
        )

    def test_native_same_name_job_checks_are_not_release_authority(self) -> None:
        checks = check_runs(
            native_acceptance_check(99103726774, 33253725121),
            native_acceptance_check(99103608680, 33253681960),
        )
        with self.assertRaisesRegex(
            resolver.ResolutionError, "final-head acceptance check, found 0"
        ):
            resolver.resolve_quality_run_id(event(), pull_request(), checks)

    def test_ignores_malformed_or_mismatched_reserved_non_authority(self) -> None:
        missing_app = acceptance_check()
        missing_app.pop("app")
        malformed = acceptance_check()
        malformed["external_id"] = "codex-quality-v1:malformed"
        mismatch = acceptance_check()
        mismatch["external_id"] = f"codex-quality-v1:pr=99:head={H1_SHA}:run={RUN_ID}"
        self.assertEqual(
            resolver.resolve_quality_run_id(
                event(),
                pull_request(),
                check_runs(missing_app, malformed, mismatch, acceptance_check()),
            ),
            RUN_ID,
        )

    def test_rejects_duplicate_or_missing_authority_acceptance(self) -> None:
        for runs in (
            check_runs(acceptance_check(), acceptance_check()),
            check_runs(native_acceptance_check(99103726774, 33253725121)),
        ):
            with (
                self.subTest(count=len(runs["check_runs"])),
                self.assertRaises(resolver.ResolutionError),
            ):
                resolver.resolve_quality_run_id(event(), pull_request(), runs)

    def test_rejects_non_successful_or_wrong_candidate_check(self) -> None:
        mutations: tuple[Callable[[dict[str, Any]], None], ...] = (
            lambda value: value.update(conclusion="failure"),
            lambda value: value.update(status="in_progress", conclusion=None),
            lambda value: value.update(head_sha=UNRELATED_SHA),
            lambda value: value.update(
                external_id=(f"codex-quality-v1:pr=99:head={H1_SHA}:run={RUN_ID}")
            ),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                accepted = acceptance_check()
                mutate(accepted)
                with self.assertRaises(resolver.ResolutionError):
                    resolver.resolve_quality_run_id(
                        event(), pull_request(), check_runs(accepted)
                    )

    def test_display_urls_and_unused_run_attempt_are_not_authority(self) -> None:
        for replacement in (None, "https://example.invalid/not-the-run"):
            with self.subTest(field="details_url", replacement=replacement):
                accepted = acceptance_check()
                if replacement is None:
                    accepted.pop("details_url")
                else:
                    accepted["details_url"] = replacement
                self.assertEqual(
                    resolver.resolve_quality_run_id(
                        event(), pull_request(), check_runs(accepted)
                    ),
                    RUN_ID,
                )

        for field in ("html_url", "run_attempt"):
            for replacement in (None, "https://example.invalid/display"):
                with self.subTest(field=field, replacement=replacement):
                    run = workflow_run()
                    if replacement is None:
                        run.pop(field)
                    else:
                        run[field] = replacement
                    self.assertEqual(
                        resolver.verify_quality_run(
                            event(), pull_request(), check_runs(), run, head_commit()
                        ),
                        RUN_ID,
                    )

    def test_rejects_incomplete_check_pagination(self) -> None:
        incomplete = check_runs()
        incomplete["total_count"] = 2
        with self.assertRaisesRegex(resolver.ResolutionError, "incomplete"):
            resolver.resolve_quality_run_id(event(), pull_request(), incomplete)

    def test_rejects_event_and_live_pull_request_disagreement(self) -> None:
        live = pull_request()
        live["head"]["sha"] = UNRELATED_SHA
        with self.assertRaisesRegex(resolver.ResolutionError, "does not match"):
            resolver.resolve_quality_run_id(event(), live, check_runs())

    def test_rejects_unrelated_or_malformed_head_commit(self) -> None:
        unrelated = head_commit(parent=UNRELATED_SHA)
        with self.assertRaises(resolver.ResolutionError):
            resolver.verify_quality_run(
                event(), pull_request(), check_runs(), workflow_run(), unrelated
            )

        malformed_cases = [
            {"sha": "not-a-sha", "parents": [{"sha": H0_SHA}]},
            {"sha": H1_SHA, "parents": [{"sha": H0_SHA}, {"sha": BASE_SHA}]},
            {"sha": H1_SHA, "parents": "not-a-list"},
        ]
        for malformed in malformed_cases:
            with (
                self.subTest(malformed=malformed),
                self.assertRaises(resolver.ResolutionError),
            ):
                resolver.verify_quality_run(
                    event(),
                    pull_request(),
                    check_runs(),
                    workflow_run(),
                    malformed,
                )

    def test_rejects_untrusted_or_unsuccessful_workflow_run(self) -> None:
        mutations: tuple[Callable[[dict[str, Any]], None], ...] = (
            lambda value: value.update(id=RUN_ID + 1),
            lambda value: value.update(path=".github/workflows/windows-client.yml"),
            lambda value: value.update(event="workflow_dispatch"),
            lambda value: value.update(head_sha=UNRELATED_SHA),
            lambda value: value.update(head_branch="main"),
            lambda value: value.update(repository={"full_name": "other/repository"}),
            lambda value: value.update(status="queued"),
            lambda value: value.update(conclusion="failure"),
            lambda value: value.update(updated_at="2026-08-29T13:00:00Z"),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                run = workflow_run()
                mutate(run)
                with self.assertRaises(resolver.ResolutionError):
                    resolver.verify_quality_run(
                        event(), pull_request(), check_runs(), run, head_commit()
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

    def test_cli_requires_head_commit_with_workflow_run(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            _write_json(path / "event.json", event())
            _write_json(path / "pull-request.json", pull_request())
            _write_json(path / "check-runs.json", check_runs())
            _write_json(path / "workflow-run.json", workflow_run())
            arguments = [
                "--event",
                str(path / "event.json"),
                "--pull-request",
                str(path / "pull-request.json"),
                "--check-runs",
                str(path / "check-runs.json"),
                "--workflow-run",
                str(path / "workflow-run.json"),
            ]
            self.assertEqual(resolver.main(arguments), 1)

    def test_release_workflow_shell_succeeds_and_rejects_unrelated_h0(self) -> None:
        success, success_output = _run_release_quality_run_step()
        self.assertEqual(success.returncode, 0, success.stderr)
        self.assertEqual(success_output, f"run-id={RUN_ID}\n")

        failure, _ = _run_release_quality_run_step(
            head_commit_value=head_commit(parent=UNRELATED_SHA)
        )
        self.assertNotEqual(failure.returncode, 0)

    def test_normal_actions_do_not_invoke_this_resolver_test(self) -> None:
        test_name = "scripts/test_release_quality_run_resolver.py"
        invoked_by = [
            path
            for path in sorted((REPO_ROOT / ".github/workflows").glob("*.yml"))
            if test_name in path.read_text(encoding="utf-8")
        ]
        self.assertEqual(invoked_by, [])

    def test_fixture_mutations_do_not_leak(self) -> None:
        first = workflow_run()
        second = copy.deepcopy(first)
        second["status"] = "queued"
        self.assertEqual(first["status"], "completed")


if __name__ == "__main__":
    unittest.main()

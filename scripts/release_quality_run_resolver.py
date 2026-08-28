#!/usr/bin/env python3
"""Resolve exactly one accepted pull-request quality workflow run.

The release workflow obtains the three JSON documents from GitHub and calls
this module as the sole identity resolver.  In particular, pull_requests on a
workflow run is deliberately not consulted: GitHub can omit it after a pull
request is merged.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


SHA_PATTERN = re.compile(r"^[0-9a-f]{40}$")
REPOSITORY_PATTERN = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
MISSING = object()


class ResolutionError(ValueError):
    """Raised when release-run evidence is missing or inconsistent."""


def _required(value: Any, key: str, path: str) -> Any:
    if not isinstance(value, dict) or key not in value:
        raise ResolutionError(f"missing {path}.{key}")
    return value[key]


def _string(value: Any, path: str) -> str:
    if not isinstance(value, str) or not value:
        raise ResolutionError(f"invalid {path}")
    return value


def _sha(value: Any, path: str) -> str:
    if not isinstance(value, str) or SHA_PATTERN.fullmatch(value) is None:
        raise ResolutionError(f"invalid {path}")
    return value


def _positive_integer(value: Any, path: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise ResolutionError(f"invalid {path}")
    return value


def _repository(value: Any, path: str) -> tuple[int, str]:
    if not isinstance(value, dict):
        raise ResolutionError(f"invalid {path}")
    repository_id = _positive_integer(_required(value, "id", path), f"{path}.id")
    full_name = _string(_required(value, "full_name", path), f"{path}.full_name")
    if REPOSITORY_PATTERN.fullmatch(full_name) is None:
        raise ResolutionError(f"invalid {path}.full_name")
    return repository_id, full_name


def _pull_request_identity(
    value: Any, path: str
) -> tuple[int, str, str, tuple[int, str], str, str, tuple[int, str], str]:
    if not isinstance(value, dict):
        raise ResolutionError(f"invalid {path}")
    number = _positive_integer(_required(value, "number", path), f"{path}.number")
    head = _required(value, "head", path)
    base = _required(value, "base", path)
    if not isinstance(head, dict) or not isinstance(base, dict):
        raise ResolutionError(f"invalid {path}.head/base")
    head_sha = _sha(_required(head, "sha", f"{path}.head"), f"{path}.head.sha")
    head_ref = _string(_required(head, "ref", f"{path}.head"), f"{path}.head.ref")
    head_repository = _repository(
        _required(head, "repo", f"{path}.head"), f"{path}.head.repo"
    )
    base_ref = _string(_required(base, "ref", f"{path}.base"), f"{path}.base.ref")
    base_sha = _sha(_required(base, "sha", f"{path}.base"), f"{path}.base.sha")
    base_repository = _repository(
        _required(base, "repo", f"{path}.base"), f"{path}.base.repo"
    )
    merge_sha = _sha(
        _required(value, "merge_commit_sha", path), f"{path}.merge_commit_sha"
    )
    return (
        number,
        head_sha,
        head_ref,
        head_repository,
        base_ref,
        base_sha,
        base_repository,
        merge_sha,
    )


def _event_identity(
    event: Any,
) -> tuple[int, str, str, tuple[int, str], str, str, tuple[int, str], str]:
    if not isinstance(event, dict):
        raise ResolutionError("invalid event")
    if _required(event, "action", "event") != "closed":
        raise ResolutionError("event action is not closed")
    event_pr = _required(event, "pull_request", "event")
    if _required(event_pr, "merged", "event.pull_request") is not True:
        raise ResolutionError("event pull request is not merged")
    identity = _pull_request_identity(event_pr, "event.pull_request")
    event_repository = _repository(
        _required(event, "repository", "event"), "event.repository"
    )
    if event_repository != identity[6]:
        raise ResolutionError("event repository does not match pull-request base")
    return identity


def _api_pull_request_identity(
    pull_request: Any,
) -> tuple[int, str, str, tuple[int, str], str, str, tuple[int, str], str]:
    if not isinstance(pull_request, dict):
        raise ResolutionError("invalid pull-request response")
    if pull_request.get("merged") is not True:
        raise ResolutionError("pull-request response is not merged")
    return _pull_request_identity(pull_request, "pull_request")


def _workflow_runs(value: Any) -> list[Any]:
    if isinstance(value, dict):
        runs = value.get("workflow_runs", MISSING)
        if not isinstance(runs, list):
            raise ResolutionError("invalid workflow-runs response")
        return runs
    if isinstance(value, list):
        runs: list[Any] = []
        for index, page in enumerate(value):
            if not isinstance(page, dict) or not isinstance(
                page.get("workflow_runs"), list
            ):
                raise ResolutionError(f"invalid workflow-runs page {index}")
            runs.extend(page["workflow_runs"])
        return runs
    raise ResolutionError("invalid workflow-runs response")


def _run_id(
    run: Any,
    expected: tuple[int, str, str, tuple[int, str], str, str, tuple[int, str], str],
) -> int:
    if not isinstance(run, dict):
        raise ResolutionError("invalid workflow run")
    (
        number,
        head_sha,
        head_ref,
        head_repository,
        _base_ref,
        _base_sha,
        base_repository,
        _merge_sha,
    ) = expected
    if run.get("path") != ".github/workflows/windows-client.yml":
        raise ResolutionError("workflow run path mismatch")
    if run.get("event") != "pull_request":
        raise ResolutionError("workflow run event mismatch")
    if run.get("status") != "completed":
        raise ResolutionError("workflow run status mismatch")
    if run.get("conclusion") != "success":
        raise ResolutionError("workflow run conclusion mismatch")
    if run.get("head_sha") != head_sha:
        raise ResolutionError("workflow run head SHA mismatch")
    head_commit = run.get("head_commit")
    if not isinstance(head_commit, dict) or head_commit.get("id") != head_sha:
        raise ResolutionError("workflow run head commit mismatch")
    if run.get("head_branch") != head_ref:
        raise ResolutionError("workflow run head branch mismatch")
    if (
        _repository(run.get("head_repository"), "workflow_run.head_repository")
        != head_repository
    ):
        raise ResolutionError("workflow run head repository mismatch")
    if (
        _repository(run.get("repository"), "workflow_run.repository")
        != base_repository
    ):
        raise ResolutionError("workflow run repository mismatch")
    referenced_workflows = run.get("referenced_workflows", MISSING)
    if not isinstance(referenced_workflows, list) or len(referenced_workflows) != 1:
        raise ResolutionError("workflow run referenced workflow cardinality mismatch")
    referenced_workflow = referenced_workflows[0]
    if not isinstance(referenced_workflow, dict):
        raise ResolutionError("invalid referenced workflow")
    referenced_sha = _sha(
        referenced_workflow.get("sha"), "referenced_workflow.sha"
    )
    if referenced_workflow.get("ref") != f"refs/pull/{number}/merge":
        raise ResolutionError("referenced workflow ref mismatch")
    expected_path = (
        f"{base_repository[1]}/.github/workflows/rust.yml@{referenced_sha}"
    )
    if referenced_workflow.get("path") != expected_path:
        raise ResolutionError("referenced workflow path mismatch")
    return _positive_integer(run.get("id"), "workflow_run.id")


def resolve_quality_run(event: Any, pull_request: Any, workflow_runs: Any) -> int:
    """Return the sole matching run ID, or raise ResolutionError."""

    event_identity = _event_identity(event)
    api_identity = _api_pull_request_identity(pull_request)
    if api_identity != event_identity:
        raise ResolutionError("pull-request identity does not match event")

    runs = _workflow_runs(workflow_runs)
    if any(not isinstance(run, dict) for run in runs):
        raise ResolutionError("invalid workflow run")
    successful_runs = [
        run
        for run in runs
        if run.get("status") == "completed" and run.get("conclusion") == "success"
    ]
    resolved = [_run_id(run, event_identity) for run in successful_runs]
    if len(resolved) != 1:
        raise ResolutionError(
            f"expected exactly one successful workflow run, found {len(resolved)}"
        )
    return resolved[0]


def _read_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ResolutionError(f"invalid {label} JSON") from exc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--event", type=Path, required=True)
    parser.add_argument("--pull-request", type=Path, required=True)
    parser.add_argument("--workflow-runs", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        resolved = resolve_quality_run(
            _read_json(args.event, "event"),
            _read_json(args.pull_request, "pull-request"),
            _read_json(args.workflow_runs, "workflow-runs"),
        )
    except ResolutionError as exc:
        print(f"release quality run resolution failed: {exc}", file=sys.stderr)
        return 1
    print(resolved)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

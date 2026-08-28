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


def _workflow_run_identity(
    run: Any,
) -> tuple[
    str,
    str,
    str,
    str,
    str,
    str,
    str,
    tuple[int, str],
    tuple[int, str],
    str,
    str,
    str,
]:
    if not isinstance(run, dict):
        raise ResolutionError("invalid workflow run")
    path = _string(_required(run, "path", "workflow_run"), "workflow_run.path")
    event = _string(_required(run, "event", "workflow_run"), "workflow_run.event")
    status = _string(
        _required(run, "status", "workflow_run"), "workflow_run.status"
    )
    conclusion = _string(
        _required(run, "conclusion", "workflow_run"), "workflow_run.conclusion"
    )
    head_sha = _sha(
        _required(run, "head_sha", "workflow_run"), "workflow_run.head_sha"
    )
    head_commit = run.get("head_commit")
    if not isinstance(head_commit, dict):
        raise ResolutionError("invalid workflow_run.head_commit")
    head_commit_id = _sha(
        _required(head_commit, "id", "workflow_run.head_commit"),
        "workflow_run.head_commit.id",
    )
    head_branch = _string(
        _required(run, "head_branch", "workflow_run"), "workflow_run.head_branch"
    )
    head_repository = _repository(
        _required(run, "head_repository", "workflow_run"),
        "workflow_run.head_repository",
    )
    base_repository = _repository(
        _required(run, "repository", "workflow_run"), "workflow_run.repository"
    )
    referenced_workflows = run.get("referenced_workflows", MISSING)
    if not isinstance(referenced_workflows, list) or len(referenced_workflows) != 1:
        raise ResolutionError("workflow run referenced workflow cardinality mismatch")
    referenced_workflow = referenced_workflows[0]
    if not isinstance(referenced_workflow, dict):
        raise ResolutionError("invalid referenced workflow")
    referenced_sha = _sha(
        _required(referenced_workflow, "sha", "referenced_workflow"),
        "referenced_workflow.sha",
    )
    referenced_ref = _string(
        _required(referenced_workflow, "ref", "referenced_workflow"),
        "referenced_workflow.ref",
    )
    referenced_path = _string(
        _required(referenced_workflow, "path", "referenced_workflow"),
        "referenced_workflow.path",
    )
    return (
        path,
        event,
        status,
        conclusion,
        head_sha,
        head_commit_id,
        head_branch,
        head_repository,
        base_repository,
        referenced_sha,
        referenced_ref,
        referenced_path,
    )


def _workflow_run_matches(
    identity: tuple[
        str,
        str,
        str,
        str,
        str,
        str,
        str,
        tuple[int, str],
        tuple[int, str],
        str,
        str,
        str,
    ],
    expected: tuple[int, str, str, tuple[int, str], str, str, tuple[int, str], str],
) -> bool:
    (
        path,
        event,
        status,
        conclusion,
        head_sha,
        head_commit_id,
        head_branch,
        head_repository,
        base_repository,
        referenced_sha,
        referenced_ref,
        referenced_path,
    ) = identity
    (
        number,
        expected_head_sha,
        expected_head_ref,
        expected_head_repository,
        _base_ref,
        _base_sha,
        expected_base_repository,
        _merge_sha,
    ) = expected
    return (
        path == ".github/workflows/windows-client.yml"
        and event == "pull_request"
        and status == "completed"
        and conclusion == "success"
        and head_sha == expected_head_sha
        and head_commit_id == expected_head_sha
        and head_branch == expected_head_ref
        and head_repository == expected_head_repository
        and base_repository == expected_base_repository
        and referenced_ref == f"refs/pull/{number}/merge"
        and referenced_path
        == f"{expected_base_repository[1]}/.github/workflows/rust.yml@{referenced_sha}"
    )


def resolve_quality_run(event: Any, pull_request: Any, workflow_runs: Any) -> int:
    """Return the sole matching run ID, or raise ResolutionError."""

    event_identity = _event_identity(event)
    api_identity = _api_pull_request_identity(pull_request)
    if api_identity != event_identity:
        raise ResolutionError("pull-request identity does not match event")

    runs = _workflow_runs(workflow_runs)
    if any(not isinstance(run, dict) for run in runs):
        raise ResolutionError("invalid workflow run")
    matching_runs: list[Any] = []
    for run in runs:
        if run.get("status") == "completed" and run.get("conclusion") == "success":
            identity = _workflow_run_identity(run)
            if _workflow_run_matches(identity, event_identity):
                matching_runs.append(run)
    if len(matching_runs) != 1:
        raise ResolutionError(
            f"expected exactly one matching successful workflow run, found {len(matching_runs)}"
        )
    return _positive_integer(matching_runs[0].get("id"), "workflow_run.id")


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

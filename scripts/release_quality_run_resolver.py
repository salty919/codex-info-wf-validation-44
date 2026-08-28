#!/usr/bin/env python3
"""Resolve the latest unique accepted pull-request quality workflow run.

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
from datetime import datetime
from pathlib import Path
from typing import Any


SHA_PATTERN = re.compile(r"^[0-9a-f]{40}$")
REPOSITORY_PATTERN = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
RFC3339_PATTERN = re.compile(
    r"^(?P<date>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})"
    r"(?P<fraction>\.\d+)?(?P<timezone>Z|[+-]\d{2}:\d{2})$"
)
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


def _nonnegative_integer(value: Any, path: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ResolutionError(f"invalid {path}")
    return value


def _rfc3339(value: Any, path: str) -> datetime:
    if not isinstance(value, str):
        raise ResolutionError(f"invalid {path}")
    match = RFC3339_PATTERN.fullmatch(value)
    if match is None or match.group("timezone") == "-00:00":
        raise ResolutionError(f"invalid {path}")
    normalized = value[:-1] + "+00:00" if value.endswith("Z") else value
    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError as exc:
        raise ResolutionError(f"invalid {path}") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise ResolutionError(f"invalid {path}")
    return parsed


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
) -> tuple[
    int,
    str,
    str,
    tuple[int, str],
    str,
    str,
    tuple[int, str],
    str,
    datetime,
]:
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
    merged_at = _rfc3339(_required(value, "merged_at", path), f"{path}.merged_at")
    return (
        number,
        head_sha,
        head_ref,
        head_repository,
        base_ref,
        base_sha,
        base_repository,
        merge_sha,
        merged_at,
    )


def _event_identity(
    event: Any,
) -> tuple[
    int,
    str,
    str,
    tuple[int, str],
    str,
    str,
    tuple[int, str],
    str,
    datetime,
]:
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
) -> tuple[
    int,
    str,
    str,
    tuple[int, str],
    str,
    str,
    tuple[int, str],
    str,
    datetime,
]:
    if not isinstance(pull_request, dict):
        raise ResolutionError("invalid pull-request response")
    if pull_request.get("merged") is not True:
        raise ResolutionError("pull-request response is not merged")
    return _pull_request_identity(pull_request, "pull_request")


def _workflow_runs(value: Any) -> list[Any]:
    if isinstance(value, dict):
        pages = [value]
    elif isinstance(value, list):
        pages = value
    else:
        raise ResolutionError("invalid workflow-runs response")
    if not pages:
        raise ResolutionError("invalid workflow-runs response")

    runs: list[Any] = []
    total_count: int | None = None
    for index, page in enumerate(pages):
        page_path = "workflow-runs" if len(pages) == 1 else f"workflow-runs page {index}"
        if not isinstance(page, dict):
            raise ResolutionError(f"invalid {page_path}")
        page_runs = _required(page, "workflow_runs", page_path)
        if not isinstance(page_runs, list):
            raise ResolutionError(f"invalid {page_path}.workflow_runs")
        page_total_count = _nonnegative_integer(
            _required(page, "total_count", page_path), f"{page_path}.total_count"
        )
        if total_count is None:
            total_count = page_total_count
        elif page_total_count != total_count:
            raise ResolutionError("workflow-runs total_count mismatch across pages")
        runs.extend(page_runs)

    assert total_count is not None
    if total_count != len(runs):
        raise ResolutionError(
            "workflow-runs total_count does not match flattened run count"
        )
    return runs


def _workflow_run_identity(
    run: Any,
) -> tuple[
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
        tuple[int, str],
        tuple[int, str],
        str,
        str,
        str,
    ],
    expected: tuple[
        int,
        str,
        str,
        tuple[int, str],
        str,
        str,
        tuple[int, str],
        str,
        datetime,
    ],
) -> bool:
    (
        path,
        event,
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
        _merged_at,
    ) = expected
    return (
        path == ".github/workflows/windows-client.yml"
        and event == "pull_request"
        and head_sha == expected_head_sha
        and head_commit_id == expected_head_sha
        and head_branch == expected_head_ref
        and head_repository == expected_head_repository
        and base_repository == expected_base_repository
        and referenced_ref == f"refs/pull/{number}/merge"
        and referenced_path
        == f"{expected_base_repository[1]}/.github/workflows/rust.yml@{referenced_sha}"
    )


def _workflow_run_metadata(
    run: Any,
) -> tuple[int, int, int, datetime, datetime]:
    if not isinstance(run, dict):
        raise ResolutionError("invalid workflow run")
    run_id = _positive_integer(_required(run, "id", "workflow_run"), "workflow_run.id")
    run_number = _positive_integer(
        _required(run, "run_number", "workflow_run"), "workflow_run.run_number"
    )
    run_attempt = _positive_integer(
        _required(run, "run_attempt", "workflow_run"), "workflow_run.run_attempt"
    )
    created_at = _rfc3339(
        _required(run, "created_at", "workflow_run"), "workflow_run.created_at"
    )
    updated_at = _rfc3339(
        _required(run, "updated_at", "workflow_run"), "workflow_run.updated_at"
    )
    return run_id, run_number, run_attempt, created_at, updated_at


def resolve_quality_run(event: Any, pull_request: Any, workflow_runs: Any) -> int:
    """Return the latest exact pre-merge successful run ID."""

    event_identity = _event_identity(event)
    api_identity = _api_pull_request_identity(pull_request)
    if api_identity != event_identity:
        raise ResolutionError("pull-request identity does not match event")

    runs = _workflow_runs(workflow_runs)
    if any(not isinstance(run, dict) for run in runs):
        raise ResolutionError("invalid workflow run")
    matching_runs: list[tuple[Any, tuple[int, int, int, datetime, datetime]]] = []
    for run in runs:
        if not isinstance(run, dict):
            raise ResolutionError("invalid workflow run")
        identity = _workflow_run_identity(run)
        if not _workflow_run_matches(identity, event_identity):
            continue
        metadata = _workflow_run_metadata(run)
        _, _, _, created_at, updated_at = metadata
        merged_at = event_identity[-1]
        if not (created_at <= updated_at <= merged_at):
            raise ResolutionError(
                "exact workflow run timestamps are not ordered before pull-request merge"
            )
        matching_runs.append((run, metadata))

    if not matching_runs:
        raise ResolutionError("expected at least one exact workflow run, found 0")

    max_run_number = max(metadata[1] for _, metadata in matching_runs)
    latest_runs = [
        item for item in matching_runs if item[1][1] == max_run_number
    ]
    if len(latest_runs) != 1:
        raise ResolutionError(
            "expected exactly one exact workflow run at maximum run_number, "
            f"found {len(latest_runs)}"
        )

    latest_run, metadata = latest_runs[0]
    if latest_run.get("status") != "completed":
        raise ResolutionError("latest exact workflow run is not completed")
    if latest_run.get("conclusion") != "success":
        raise ResolutionError("latest exact workflow run is not successful")
    return metadata[0]


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

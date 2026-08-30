#!/usr/bin/env python3
"""Resolve and verify the trusted workflow run bound to final-head acceptance."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

GITHUB_ACTIONS_APP_ID = 15368
SHA_PATTERN = re.compile(r"^[0-9a-f]{40}$")
REPOSITORY_PATTERN = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
EXTERNAL_ID_PATTERN = re.compile(
    r"^codex-quality-v1:pr=(?P<pr>[1-9][0-9]*):"
    r"head=(?P<head>[0-9a-f]{40}):run=(?P<run>[1-9][0-9]*)$"
)


class ResolutionError(ValueError):
    """Raised when accepted-run evidence is missing or inconsistent."""


@dataclass(frozen=True)
class PullRequestIdentity:
    """Named merged-PR identity used by every release authority check."""

    number: int
    head_sha: str
    head_ref: str
    head_repository: str
    base_sha: str
    base_ref: str
    base_repository: str
    merge_commit_sha: str
    merged_at: datetime


def _required(value: Any, key: str, path: str) -> Any:
    if not isinstance(value, dict) or key not in value:
        raise ResolutionError(f"missing {path}.{key}")
    return value[key]


def _positive_integer(value: Any, path: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise ResolutionError(f"invalid {path}")
    return value


def _sha(value: Any, path: str) -> str:
    if not isinstance(value, str) or SHA_PATTERN.fullmatch(value) is None:
        raise ResolutionError(f"invalid {path}")
    return value


def _repository(value: Any, path: str) -> str:
    if not isinstance(value, str) or REPOSITORY_PATTERN.fullmatch(value) is None:
        raise ResolutionError(f"invalid {path}")
    return value


def _ref(value: Any, path: str) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise ResolutionError(f"invalid {path}")
    return value


def _timestamp(value: Any, path: str) -> datetime:
    if not isinstance(value, str) or not value:
        raise ResolutionError(f"invalid {path}")
    normalized = value[:-1] + "+00:00" if value.endswith("Z") else value
    try:
        result = datetime.fromisoformat(normalized)
    except ValueError as error:
        raise ResolutionError(f"invalid {path}") from error
    if result.tzinfo is None or result.utcoffset() is None:
        raise ResolutionError(f"invalid {path}")
    return result


def _pull_request_identity(value: Any, path: str) -> PullRequestIdentity:
    number = _positive_integer(_required(value, "number", path), f"{path}.number")
    head = _required(value, "head", path)
    base = _required(value, "base", path)
    head_repo = _required(head, "repo", f"{path}.head")
    base_repo = _required(base, "repo", f"{path}.base")
    return PullRequestIdentity(
        number=number,
        head_sha=_sha(_required(head, "sha", f"{path}.head"), f"{path}.head.sha"),
        head_ref=_ref(_required(head, "ref", f"{path}.head"), f"{path}.head.ref"),
        head_repository=_repository(
            _required(head_repo, "full_name", f"{path}.head.repo"),
            f"{path}.head.repo.full_name",
        ),
        base_sha=_sha(_required(base, "sha", f"{path}.base"), f"{path}.base.sha"),
        base_ref=_ref(_required(base, "ref", f"{path}.base"), f"{path}.base.ref"),
        base_repository=_repository(
            _required(base_repo, "full_name", f"{path}.base.repo"),
            f"{path}.base.repo.full_name",
        ),
        merge_commit_sha=_sha(
            _required(value, "merge_commit_sha", path),
            f"{path}.merge_commit_sha",
        ),
        merged_at=_timestamp(_required(value, "merged_at", path), f"{path}.merged_at"),
    )


def validate_merged_identity(event: Any, pull_request: Any) -> PullRequestIdentity:
    if _required(event, "action", "event") != "closed":
        raise ResolutionError("event action is not closed")
    event_pr = _required(event, "pull_request", "event")
    if _required(event_pr, "merged", "event.pull_request") is not True:
        raise ResolutionError("event pull request is not merged")
    if _required(pull_request, "merged", "pull_request") is not True:
        raise ResolutionError("pull-request response is not merged")
    event_identity = _pull_request_identity(event_pr, "event.pull_request")
    api_identity = _pull_request_identity(pull_request, "pull_request")
    if event_identity != api_identity:
        raise ResolutionError("pull-request identity does not match event")
    event_repository = _required(event, "repository", "event")
    repository = _repository(
        _required(event_repository, "full_name", "event.repository"),
        "event.repository.full_name",
    )
    if (
        repository != event_identity.base_repository
        or event_identity.head_repository != repository
        or event_identity.base_ref != "main"
    ):
        raise ResolutionError("merged identity is outside same-repository PRs to main")
    return event_identity


def _head_commit_parents(value: Any, expected_sha: str) -> tuple[str, ...]:
    commit_sha = _sha(_required(value, "sha", "head_commit"), "head_commit.sha")
    if commit_sha != expected_sha:
        raise ResolutionError("head commit does not match final PR head")
    raw_parents = _required(value, "parents", "head_commit")
    if not isinstance(raw_parents, list):
        raise ResolutionError("invalid head_commit.parents")
    return tuple(
        _sha(
            _required(parent, "sha", f"head_commit.parents[{index}]"),
            f"head_commit.parents[{index}].sha",
        )
        for index, parent in enumerate(raw_parents)
    )


def _flatten_check_runs(value: Any) -> list[Any]:
    pages = value if isinstance(value, list) else [value]
    if not pages:
        raise ResolutionError("check-runs response is empty")
    result: list[Any] = []
    total_count: int | None = None
    for index, page in enumerate(pages):
        path = f"check-runs[{index}]"
        runs = _required(page, "check_runs", path)
        if not isinstance(runs, list):
            raise ResolutionError(f"invalid {path}.check_runs")
        count = _required(page, "total_count", path)
        if isinstance(count, bool) or not isinstance(count, int) or count < 0:
            raise ResolutionError(f"invalid {path}.total_count")
        if total_count is None:
            total_count = count
        elif total_count != count:
            raise ResolutionError("check-run total_count differs across pages")
        result.extend(runs)
    if total_count != len(result):
        raise ResolutionError("check-run response is incomplete")
    return result


def _resolve_quality_run_id(identity: PullRequestIdentity, check_runs: Any) -> int:
    exact: list[tuple[Any, re.Match[str]]] = []
    for run in _flatten_check_runs(check_runs):
        if not isinstance(run, dict):
            raise ResolutionError("invalid check run")
        if not (
            run.get("name") == "acceptance" and run.get("head_sha") == identity.head_sha
        ):
            continue
        app = run.get("app")
        if not isinstance(app, dict) or app.get("id") != GITHUB_ACTIONS_APP_ID:
            continue
        external_id = run.get("external_id")
        match = (
            EXTERNAL_ID_PATTERN.fullmatch(external_id)
            if isinstance(external_id, str)
            else None
        )
        if match is None:
            continue
        if (int(match.group("pr")), match.group("head")) != (
            identity.number,
            identity.head_sha,
        ):
            continue
        exact.append((run, match))
    if len(exact) != 1:
        raise ResolutionError(
            f"expected exactly one final-head acceptance check, found {len(exact)}"
        )
    accepted, match = exact[0]
    if accepted.get("status") != "completed" or accepted.get("conclusion") != "success":
        raise ResolutionError("final-head acceptance check is not successful")
    return int(match.group("run"))


def resolve_quality_run_id(event: Any, pull_request: Any, check_runs: Any) -> int:
    """Return the unique run ID named by successful H1 acceptance."""

    return _resolve_quality_run_id(
        validate_merged_identity(event, pull_request), check_runs
    )


def verify_quality_run(
    event: Any,
    pull_request: Any,
    check_runs: Any,
    workflow_run: Any,
    head_commit: Any,
) -> int:
    """Verify that the selected run is the successful trusted main authority."""

    identity = validate_merged_identity(event, pull_request)
    run_id = _resolve_quality_run_id(identity, check_runs)
    workflow_repository = _required(workflow_run, "repository", "workflow_run")
    workflow_head_sha = _sha(
        _required(workflow_run, "head_sha", "workflow_run"),
        "workflow_run.head_sha",
    )
    final_head_parents = _head_commit_parents(head_commit, identity.head_sha)
    if (
        _positive_integer(
            _required(workflow_run, "id", "workflow_run"), "workflow_run.id"
        )
        != run_id
    ):
        raise ResolutionError("workflow run ID does not match acceptance")
    if (
        workflow_run.get("path") != ".github/workflows/version-prepare.yml"
        or workflow_run.get("event") != "pull_request_target"
        or workflow_run.get("head_branch") != identity.head_ref
        or _required(workflow_repository, "full_name", "workflow_run.repository")
        != identity.base_repository
        or workflow_run.get("status") != "completed"
        or workflow_run.get("conclusion") != "success"
    ):
        raise ResolutionError(
            "workflow run is not the successful trusted main authority"
        )
    if workflow_head_sha != identity.head_sha and final_head_parents != (
        workflow_head_sha,
    ):
        raise ResolutionError(
            "workflow run head is neither the final head nor its sole parent"
        )
    created_at = _timestamp(
        _required(workflow_run, "created_at", "workflow_run"),
        "workflow_run.created_at",
    )
    updated_at = _timestamp(
        _required(workflow_run, "updated_at", "workflow_run"),
        "workflow_run.updated_at",
    )
    if not created_at <= updated_at <= identity.merged_at:
        raise ResolutionError("workflow run timestamps are not ordered before merge")
    return run_id


def _read_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ResolutionError(f"invalid {label} JSON") from error


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--event", type=Path, required=True)
    parser.add_argument("--pull-request", type=Path, required=True)
    parser.add_argument("--check-runs", type=Path, required=True)
    parser.add_argument("--workflow-run", type=Path)
    parser.add_argument("--head-commit", type=Path)
    args = parser.parse_args(argv)
    try:
        event = _read_json(args.event, "event")
        pull_request = _read_json(args.pull_request, "pull-request")
        check_runs = _read_json(args.check_runs, "check-runs")
        if args.workflow_run is None:
            if args.head_commit is not None:
                raise ResolutionError("head-commit requires workflow-run")
            resolved = resolve_quality_run_id(event, pull_request, check_runs)
        else:
            if args.head_commit is None:
                raise ResolutionError("workflow-run verification requires head-commit")
            resolved = verify_quality_run(
                event,
                pull_request,
                check_runs,
                _read_json(args.workflow_run, "workflow-run"),
                _read_json(args.head_commit, "head-commit"),
            )
    except ResolutionError as error:
        print(f"release quality run resolution failed: {error}", file=sys.stderr)
        return 1
    print(resolved)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

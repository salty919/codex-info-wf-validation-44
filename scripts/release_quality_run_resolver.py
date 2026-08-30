#!/usr/bin/env python3
"""Resolve and verify the trusted workflow run bound to final-head acceptance."""

from __future__ import annotations

import argparse
import json
import re
import sys
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


def _pull_request_identity(value: Any, path: str) -> tuple[Any, ...]:
    number = _positive_integer(_required(value, "number", path), f"{path}.number")
    head = _required(value, "head", path)
    base = _required(value, "base", path)
    head_repo = _required(head, "repo", f"{path}.head")
    base_repo = _required(base, "repo", f"{path}.base")
    return (
        number,
        _sha(_required(head, "sha", f"{path}.head"), f"{path}.head.sha"),
        _ref(_required(head, "ref", f"{path}.head"), f"{path}.head.ref"),
        _repository(
            _required(head_repo, "full_name", f"{path}.head.repo"),
            f"{path}.head.repo.full_name",
        ),
        _sha(_required(base, "sha", f"{path}.base"), f"{path}.base.sha"),
        _ref(_required(base, "ref", f"{path}.base"), f"{path}.base.ref"),
        _repository(
            _required(base_repo, "full_name", f"{path}.base.repo"),
            f"{path}.base.repo.full_name",
        ),
        _sha(_required(value, "merge_commit_sha", path), f"{path}.merge_commit_sha"),
        _timestamp(_required(value, "merged_at", path), f"{path}.merged_at"),
    )


def validate_merged_identity(event: Any, pull_request: Any) -> tuple[Any, ...]:
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
        repository != event_identity[6]
        or event_identity[3] != repository
        or event_identity[5] != "main"
    ):
        raise ResolutionError("merged identity is outside same-repository PRs to main")
    return event_identity


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


def resolve_quality_run_id(event: Any, pull_request: Any, check_runs: Any) -> int:
    """Return the unique run ID named by successful H1 acceptance."""

    identity = validate_merged_identity(event, pull_request)
    pr_number, head_sha = identity[0], identity[1]
    exact: list[tuple[Any, re.Match[str]]] = []
    for run in _flatten_check_runs(check_runs):
        if not isinstance(run, dict):
            raise ResolutionError("invalid check run")
        app = _required(run, "app", "check_run")
        if not (
            run.get("name") == "acceptance"
            and run.get("head_sha") == head_sha
            and _required(app, "id", "check_run.app") == GITHUB_ACTIONS_APP_ID
        ):
            continue
        external_id = run.get("external_id")
        match = (
            EXTERNAL_ID_PATTERN.fullmatch(external_id)
            if isinstance(external_id, str)
            else None
        )
        if match is None:
            # Native Actions job checks use an opaque UUID external ID and may
            # share the required context name. They are not release authority.
            # Anything claiming our reserved namespace must still fail closed.
            if isinstance(external_id, str) and external_id.startswith(
                "codex-quality-v1:"
            ):
                raise ResolutionError(
                    "final-head acceptance external_id is malformed"
                )
            continue
        if (int(match.group("pr")), match.group("head")) != (pr_number, head_sha):
            raise ResolutionError("final-head acceptance belongs to another candidate")
        exact.append((run, match))
    if len(exact) != 1:
        raise ResolutionError(
            f"expected exactly one final-head acceptance check, found {len(exact)}"
        )
    accepted, match = exact[0]
    if accepted.get("status") != "completed" or accepted.get("conclusion") != "success":
        raise ResolutionError("final-head acceptance check is not successful")
    run_id = int(match.group("run"))
    expected_url = f"https://github.com/{identity[6]}/actions/runs/{run_id}"
    if accepted.get("details_url") != expected_url:
        raise ResolutionError(
            "final-head acceptance details URL does not match its run"
        )
    return run_id


def verify_quality_run(
    event: Any, pull_request: Any, check_runs: Any, workflow_run: Any
) -> int:
    """Verify that the selected run is the successful trusted main authority."""

    identity = validate_merged_identity(event, pull_request)
    run_id = resolve_quality_run_id(event, pull_request, check_runs)
    repository = identity[6]
    workflow_repository = _required(workflow_run, "repository", "workflow_run")
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
        or workflow_run.get("head_branch") != "main"
        or workflow_run.get("head_sha") != identity[4]
        or _required(workflow_repository, "full_name", "workflow_run.repository")
        != repository
        or workflow_run.get("status") != "completed"
        or workflow_run.get("conclusion") != "success"
        or workflow_run.get("html_url")
        != f"https://github.com/{repository}/actions/runs/{run_id}"
    ):
        raise ResolutionError(
            "workflow run is not the successful trusted main authority"
        )
    _positive_integer(
        _required(workflow_run, "run_attempt", "workflow_run"),
        "workflow_run.run_attempt",
    )
    created_at = _timestamp(
        _required(workflow_run, "created_at", "workflow_run"),
        "workflow_run.created_at",
    )
    updated_at = _timestamp(
        _required(workflow_run, "updated_at", "workflow_run"),
        "workflow_run.updated_at",
    )
    if not created_at <= updated_at <= identity[8]:
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
    args = parser.parse_args(argv)
    try:
        event = _read_json(args.event, "event")
        pull_request = _read_json(args.pull_request, "pull-request")
        check_runs = _read_json(args.check_runs, "check-runs")
        if args.workflow_run is None:
            resolved = resolve_quality_run_id(event, pull_request, check_runs)
        else:
            resolved = verify_quality_run(
                event,
                pull_request,
                check_runs,
                _read_json(args.workflow_run, "workflow-run"),
            )
    except ResolutionError as error:
        print(f"release quality run resolution failed: {error}", file=sys.stderr)
        return 1
    print(resolved)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

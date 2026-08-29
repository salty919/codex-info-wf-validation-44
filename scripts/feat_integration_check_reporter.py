#!/usr/bin/env python3
"""Register and finalize the single trusted feat/next integration check."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from typing import Any, Protocol, Sequence


CHECK_NAME = "feat-acceptance"
APP_ID = 15368
FULL_SHA = re.compile(r"^[0-9a-f]{40}$")
REPOSITORY = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")


class ReporterError(ValueError):
    pass


class Client(Protocol):
    def get(self, endpoint: str) -> Any: ...
    def post(self, endpoint: str, payload: dict[str, Any]) -> Any: ...
    def patch(self, endpoint: str, payload: dict[str, Any]) -> Any: ...


class GhClient:
    def _call(self, method: str, endpoint: str, payload: dict[str, Any] | None = None) -> Any:
        command = [
            "gh", "api", "--method", method,
            "-H", "Accept: application/vnd.github+json",
            "-H", "X-GitHub-Api-Version: 2022-11-28",
            endpoint,
        ]
        if payload is not None:
            command.extend(("--input", "-"))
        result = subprocess.run(
            command,
            input=None if payload is None else json.dumps(payload),
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise ReporterError("GitHub API request failed")
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError as exc:
            raise ReporterError("GitHub API response is malformed") from exc

    def get(self, endpoint: str) -> Any:
        return self._call("GET", endpoint)

    def post(self, endpoint: str, payload: dict[str, Any]) -> Any:
        return self._call("POST", endpoint, payload)

    def patch(self, endpoint: str, payload: dict[str, Any]) -> Any:
        return self._call("PATCH", endpoint, payload)


def external_id(pr_number: int, head_sha: str) -> str:
    return f"codex-feat-v1:pr={pr_number}:head={head_sha}"


def _validate_inputs(
    repository: str,
    pr_number: int,
    base_sha: str,
    head_ref: str,
    head_sha: str,
    run_url: str,
) -> None:
    if not REPOSITORY.fullmatch(repository):
        raise ReporterError("repository is malformed")
    if pr_number <= 0:
        raise ReporterError("pull request number is malformed")
    if not FULL_SHA.fullmatch(base_sha) or not FULL_SHA.fullmatch(head_sha):
        raise ReporterError("pull request SHA is malformed")
    if not head_ref or "\x00" in head_ref:
        raise ReporterError("pull request head ref is malformed")
    if not run_url.startswith("https://github.com/"):
        raise ReporterError("run URL is malformed")


def _validate_live_pr(
    client: Client,
    repository: str,
    pr_number: int,
    base_sha: str,
    head_ref: str,
    head_sha: str,
) -> None:
    pr = client.get(f"repos/{repository}/pulls/{pr_number}")
    try:
        valid = (
            pr["number"] == pr_number
            and pr["state"] == "open"
            and pr["base"]["repo"]["full_name"] == repository
            and pr["base"]["ref"] == "feat/next"
            and pr["base"]["sha"] == base_sha
            and pr["head"]["repo"]["full_name"] == repository
            and pr["head"]["ref"] == head_ref
            and pr["head"]["sha"] == head_sha
        )
    except (KeyError, TypeError):
        valid = False
    if not valid:
        raise ReporterError("live pull request identity moved or is malformed")


def _check_runs(client: Client, repository: str, head_sha: str) -> list[dict[str, Any]]:
    response = client.get(
        f"repos/{repository}/commits/{head_sha}/check-runs?check_name={CHECK_NAME}&filter=all&per_page=100"
    )
    if not isinstance(response, dict) or not isinstance(response.get("check_runs"), list):
        raise ReporterError("check-run response is malformed")
    if response.get("total_count") != len(response["check_runs"]):
        raise ReporterError("check-run pagination is incomplete")
    return response["check_runs"]


def _owned_check(check: dict[str, Any], expected_external_id: str) -> bool:
    try:
        return (
            check["name"] == CHECK_NAME
            and check["app"]["id"] == APP_ID
            and check["external_id"] == expected_external_id
        )
    except (KeyError, TypeError):
        return False


def register(
    client: Client,
    *,
    repository: str,
    pr_number: int,
    base_sha: str,
    head_ref: str,
    head_sha: str,
    run_url: str,
) -> int:
    _validate_inputs(repository, pr_number, base_sha, head_ref, head_sha, run_url)
    _validate_live_pr(client, repository, pr_number, base_sha, head_ref, head_sha)
    identity = external_id(pr_number, head_sha)
    checks = _check_runs(client, repository, head_sha)
    if any(not _owned_check(check, identity) for check in checks):
        raise ReporterError("foreign, malformed, or duplicate-name feat check exists")
    if len(checks) > 1:
        raise ReporterError("multiple feat integration checks exist for the current head")
    payload = {
        "name": CHECK_NAME,
        "status": "in_progress",
        "external_id": identity,
        "details_url": run_url,
        "output": {
            "title": "Selective integration quality is running",
            "summary": "Only owners selected from the complete pull-request path set are evaluated.",
        },
    }
    if checks:
        check_id = checks[0].get("id")
        if type(check_id) is not int or check_id <= 0:
            raise ReporterError("existing check id is malformed")
        response = client.patch(f"repos/{repository}/check-runs/{check_id}", payload)
    else:
        response = client.post(
            f"repos/{repository}/check-runs", {"head_sha": head_sha, **payload}
        )
    check_id = response.get("id") if isinstance(response, dict) else None
    if type(check_id) is not int or check_id <= 0:
        raise ReporterError("check registration response is malformed")
    return check_id


def finalize(
    client: Client,
    *,
    repository: str,
    pr_number: int,
    base_sha: str,
    head_ref: str,
    head_sha: str,
    run_url: str,
    check_id: int,
    quality_result: str,
) -> None:
    _validate_inputs(repository, pr_number, base_sha, head_ref, head_sha, run_url)
    if check_id <= 0:
        raise ReporterError("check id is malformed")
    conclusion = "success"
    summary = "Every selected quality owner succeeded and every unselected owner was skipped."
    try:
        _validate_live_pr(client, repository, pr_number, base_sha, head_ref, head_sha)
        checks = _check_runs(client, repository, head_sha)
        identity = external_id(pr_number, head_sha)
        if len(checks) != 1 or checks[0].get("id") != check_id or not _owned_check(checks[0], identity):
            raise ReporterError("the exact registered feat check could not be proven")
        if quality_result != "success":
            raise ReporterError(f"selective quality did not succeed: {quality_result}")
    except ReporterError as exc:
        conclusion = "failure"
        summary = str(exc)
    response = client.patch(
        f"repos/{repository}/check-runs/{check_id}",
        {
            "name": CHECK_NAME,
            "status": "completed",
            "conclusion": conclusion,
            "details_url": run_url,
            "output": {"title": f"Selective integration quality: {conclusion}", "summary": summary},
        },
    )
    if not isinstance(response, dict) or response.get("id") != check_id:
        raise ReporterError("check finalization response is malformed")
    if conclusion != "success":
        raise ReporterError(summary)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("register", "finalize"))
    parser.add_argument("--repository", required=True)
    parser.add_argument("--pr-number", type=int, required=True)
    parser.add_argument("--base-sha", required=True)
    parser.add_argument("--head-ref", required=True)
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--github-output")
    parser.add_argument("--check-id", type=int)
    parser.add_argument("--quality-result")
    args = parser.parse_args(argv)
    client = GhClient()
    try:
        if args.command == "register":
            check_id = register(
                client,
                repository=args.repository,
                pr_number=args.pr_number,
                base_sha=args.base_sha,
                head_ref=args.head_ref,
                head_sha=args.head_sha,
                run_url=args.run_url,
            )
            if not args.github_output:
                raise ReporterError("GitHub output path is required for registration")
            with open(args.github_output, "a", encoding="utf-8") as output:
                output.write(f"check_id={check_id}\n")
        else:
            if args.check_id is None or args.quality_result is None:
                raise ReporterError("finalization inputs are incomplete")
            finalize(
                client,
                repository=args.repository,
                pr_number=args.pr_number,
                base_sha=args.base_sha,
                head_ref=args.head_ref,
                head_sha=args.head_sha,
                run_url=args.run_url,
                check_id=args.check_id,
                quality_result=args.quality_result,
            )
    except ReporterError as exc:
        print(f"feat-integration-check-reporter: FAIL {exc}", file=sys.stderr)
        return 1
    print(f"feat-integration-check-reporter: PASS {args.command}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

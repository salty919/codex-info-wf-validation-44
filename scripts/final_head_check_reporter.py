#!/usr/bin/env python3
"""Publish the two required checks on the immutable final PR head.

The caller is a trusted ``pull_request_target`` job.  This module never reads
or executes PR code.  It binds every mutation to the live pull-request
identity and to the GitHub Actions App before creating or updating checks.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Protocol
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen

API_VERSION = "2022-11-28"
GITHUB_ACTIONS_APP_ID = 15368
CHECK_NAMES = ("version-prepared", "acceptance")
SHA_PATTERN = re.compile(r"^[0-9a-f]{40}$")
DIGEST_PATTERN = re.compile(r"^[0-9a-f]{64}$")
REPOSITORY_PATTERN = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
EXTERNAL_ID_PATTERN = re.compile(
    r"^codex-quality-v1:pr=(?P<pr>[1-9][0-9]*):"
    r"head=(?P<head>[0-9a-f]{40}):run=(?P<run>[1-9][0-9]*)$"
)


class ReporterError(ValueError):
    """Raised when final-head evidence is missing or ambiguous."""


class Api(Protocol):
    def request(
        self, method: str, path: str, payload: dict[str, Any] | None = None
    ) -> tuple[int, Any]: ...


@dataclass(frozen=True)
class Identity:
    repository: str
    pr_number: int
    base_repository: str
    head_repository: str
    base_sha: str
    head_sha: str
    head_ref: str
    run_id: int
    run_url: str

    @property
    def external_id(self) -> str:
        return (
            f"codex-quality-v1:pr={self.pr_number}:"
            f"head={self.head_sha}:run={self.run_id}"
        )


@dataclass(frozen=True)
class RegisterResult:
    quality_required: bool
    version_check_id: int
    acceptance_check_id: int


class GitHubApi:
    def __init__(self, token: str, base_url: str = "https://api.github.com") -> None:
        if not token:
            raise ReporterError("GH_TOKEN is missing")
        self._token = token
        self._base_url = base_url.rstrip("/")

    def request(
        self, method: str, path: str, payload: dict[str, Any] | None = None
    ) -> tuple[int, Any]:
        body = None
        if payload is not None:
            body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        request = Request(
            self._base_url + path,
            data=body,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self._token}",
                "X-GitHub-Api-Version": API_VERSION,
                "User-Agent": "codex-info-final-head-reporter",
            },
        )
        try:
            with urlopen(request, timeout=30) as response:
                raw = response.read()
                return response.status, json.loads(raw) if raw else {}
        except HTTPError as error:
            raw = error.read()
            detail = raw.decode("utf-8", errors="replace")
            raise ReporterError(
                f"GitHub API {method} {path} failed with HTTP {error.code}: {detail}"
            ) from error


def _positive_integer(value: Any, label: str) -> int:
    if isinstance(value, bool):
        raise ReporterError(f"invalid {label}")
    try:
        result = int(value)
    except (TypeError, ValueError) as error:
        raise ReporterError(f"invalid {label}") from error
    if result <= 0 or str(result) != str(value):
        raise ReporterError(f"invalid {label}")
    return result


def _sha(value: Any, label: str) -> str:
    if not isinstance(value, str) or SHA_PATTERN.fullmatch(value) is None:
        raise ReporterError(f"invalid {label}")
    return value


def _repository(value: Any, label: str) -> str:
    if not isinstance(value, str) or REPOSITORY_PATTERN.fullmatch(value) is None:
        raise ReporterError(f"invalid {label}")
    return value


def _required(mapping: Any, key: str, label: str) -> Any:
    if not isinstance(mapping, dict) or key not in mapping:
        raise ReporterError(f"missing {label}.{key}")
    return mapping[key]


def validate_identity(identity: Identity) -> None:
    _repository(identity.repository, "repository")
    _repository(identity.base_repository, "base repository")
    _repository(identity.head_repository, "head repository")
    _positive_integer(identity.pr_number, "PR number")
    _positive_integer(identity.run_id, "run ID")
    _sha(identity.base_sha, "base SHA")
    _sha(identity.head_sha, "head SHA")
    if (
        identity.repository != identity.base_repository
        or identity.repository != identity.head_repository
        or identity.head_ref != "feat/next"
        or identity.run_url
        != f"https://github.com/{identity.repository}/actions/runs/{identity.run_id}"
    ):
        raise ReporterError(
            "identity is outside the trusted feat/next to main boundary"
        )


def _validate_pull_request(api: Api, identity: Identity, *, require_open: bool) -> None:
    status, pull_request = api.request(
        "GET", f"/repos/{identity.repository}/pulls/{identity.pr_number}"
    )
    if status != 200:
        raise ReporterError(f"pull-request read returned HTTP {status}")
    head = _required(pull_request, "head", "pull_request")
    base = _required(pull_request, "base", "pull_request")
    expected_state = "open" if require_open else "closed"
    actual = (
        _positive_integer(
            _required(pull_request, "number", "pull_request"), "PR number"
        ),
        _required(pull_request, "state", "pull_request"),
        _required(
            _required(base, "repo", "pull_request.base"),
            "full_name",
            "pull_request.base.repo",
        ),
        _required(base, "ref", "pull_request.base"),
        _required(base, "sha", "pull_request.base"),
        _required(
            _required(head, "repo", "pull_request.head"),
            "full_name",
            "pull_request.head.repo",
        ),
        _required(head, "ref", "pull_request.head"),
        _required(head, "sha", "pull_request.head"),
    )
    expected = (
        identity.pr_number,
        expected_state,
        identity.base_repository,
        "main",
        identity.base_sha,
        identity.head_repository,
        identity.head_ref,
        identity.head_sha,
    )
    if actual != expected:
        raise ReporterError("live pull-request identity does not match the final head")


def _check_runs(api: Api, identity: Identity, name: str) -> list[dict[str, Any]]:
    encoded_name = quote(name, safe="")
    status, response = api.request(
        "GET",
        f"/repos/{identity.repository}/commits/{identity.head_sha}/check-runs"
        f"?check_name={encoded_name}&filter=all&per_page=100",
    )
    if status != 200 or not isinstance(response, dict):
        raise ReporterError(f"check-run list for {name} returned HTTP {status}")
    total_count = (
        _positive_integer(response.get("total_count", 0), "check total")
        if response.get("total_count")
        else 0
    )
    runs = response.get("check_runs")
    if not isinstance(runs, list) or total_count != len(runs) or total_count >= 100:
        raise ReporterError(f"check-run list for {name} is incomplete or malformed")
    matching: list[dict[str, Any]] = []
    for run in runs:
        app = _required(run, "app", "check_run")
        if _required(run, "name", "check_run") != name:
            raise ReporterError(f"check-run query returned a different name for {name}")
        if _required(run, "head_sha", "check_run") != identity.head_sha:
            raise ReporterError(f"check-run query returned a different head for {name}")
        if _required(app, "id", "check_run.app") != GITHUB_ACTIONS_APP_ID:
            raise ReporterError(
                f"non-Actions App {name} check exists on the final head"
            )
        matching.append(run)
    if len(matching) > 1:
        raise ReporterError(
            f"multiple GitHub Actions {name} checks exist on the final head"
        )
    return matching


def _parse_external_id(value: Any) -> tuple[int, str, int]:
    if not isinstance(value, str):
        raise ReporterError("check external_id is missing")
    match = EXTERNAL_ID_PATTERN.fullmatch(value)
    if match is None:
        raise ReporterError("check external_id is malformed")
    return int(match.group("pr")), match.group("head"), int(match.group("run"))


def _verify_check_response(
    status: int, response: Any, identity: Identity, name: str, expected_status: int
) -> dict[str, Any]:
    if status != expected_status or not isinstance(response, dict):
        raise ReporterError(f"{name} check mutation returned HTTP {status}")
    app = _required(response, "app", "check_run")
    if (
        _required(response, "name", "check_run") != name
        or _required(response, "head_sha", "check_run") != identity.head_sha
        or _required(response, "external_id", "check_run") != identity.external_id
        or _required(app, "id", "check_run.app") != GITHUB_ACTIONS_APP_ID
    ):
        raise ReporterError(f"{name} check mutation response has the wrong identity")
    _positive_integer(_required(response, "id", "check_run"), "check ID")
    return response


def _check_payload(
    identity: Identity,
    name: str,
    *,
    status: str,
    conclusion: str | None,
    title: str,
    summary: str,
) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "name": name,
        "head_sha": identity.head_sha,
        "external_id": identity.external_id,
        "details_url": identity.run_url,
        "status": status,
        "output": {"title": title, "summary": summary},
    }
    if conclusion is not None:
        payload["conclusion"] = conclusion
    return payload


def _mutate_check(
    api: Api,
    identity: Identity,
    name: str,
    existing: dict[str, Any] | None,
    payload: dict[str, Any],
) -> dict[str, Any]:
    if existing is None:
        status, response = api.request(
            "POST", f"/repos/{identity.repository}/check-runs", payload
        )
        return _verify_check_response(status, response, identity, name, 201)
    check_id = _positive_integer(_required(existing, "id", "check_run"), "check ID")
    status, response = api.request(
        "PATCH", f"/repos/{identity.repository}/check-runs/{check_id}", payload
    )
    return _verify_check_response(status, response, identity, name, 200)


def _originating_run_is_active(api: Api, identity: Identity, run_id: int) -> bool:
    status, run = api.request(
        "GET", f"/repos/{identity.repository}/actions/runs/{run_id}"
    )
    if status != 200 or not isinstance(run, dict):
        raise ReporterError(f"originating workflow run {run_id} could not be verified")
    if (
        _required(run, "repository", "workflow_run").get("full_name")
        != identity.repository
    ):
        raise ReporterError("originating workflow run repository mismatch")
    return _required(run, "status", "workflow_run") != "completed"


def _check_owner(identity: Identity, check: dict[str, Any]) -> int:
    pr_number, head_sha, run_id = _parse_external_id(
        _required(check, "external_id", "check_run")
    )
    if (pr_number, head_sha) != (identity.pr_number, identity.head_sha):
        raise ReporterError("existing required check belongs to another candidate")
    return run_id


def _assert_unique_mutation(
    api: Api,
    identity: Identity,
    name: str,
    mutated: dict[str, Any],
    *,
    expected_status: str,
    expected_conclusion: str | None,
) -> None:
    read_back = _check_runs(api, identity, name)
    if (
        len(read_back) != 1
        or _required(read_back[0], "id", "check_run")
        != _required(mutated, "id", "check_run")
        or _required(read_back[0], "external_id", "check_run") != identity.external_id
        or _required(read_back[0], "status", "check_run") != expected_status
        or _required(read_back[0], "conclusion", "check_run") != expected_conclusion
    ):
        raise ReporterError(f"{name} check state is not exact after mutation")


def register_checks(api: Api, identity: Identity) -> RegisterResult:
    validate_identity(identity)
    _validate_pull_request(api, identity, require_open=True)
    existing = {
        name: (_check_runs(api, identity, name) or [None])[0] for name in CHECK_NAMES
    }

    version_existing = existing["version-prepared"]
    acceptance = existing["acceptance"]
    owners = {
        name: _check_owner(identity, check)
        for name, check in existing.items()
        if check is not None
    }
    acceptance_successful = acceptance is not None and (
        _required(acceptance, "status", "check_run") == "completed"
        and _required(acceptance, "conclusion", "check_run") == "success"
    )
    if acceptance_successful:
        if (
            version_existing is None
            or _required(version_existing, "status", "check_run") != "completed"
            or _required(version_existing, "conclusion", "check_run") != "success"
            or owners["version-prepared"] != owners["acceptance"]
        ):
            raise ReporterError("successful required checks do not share one owner")
        return RegisterResult(
            quality_required=False,
            version_check_id=_positive_integer(
                version_existing["id"], "version check ID"
            ),
            acceptance_check_id=_positive_integer(
                acceptance["id"], "acceptance check ID"
            ),
        )

    for prior_run_id in set(owners.values()):
        if prior_run_id != identity.run_id and _originating_run_is_active(
            api, identity, prior_run_id
        ):
            raise ReporterError("another active run owns a final-head required check")

    version_payload = _check_payload(
        identity,
        "version-prepared",
        status="completed",
        conclusion="success",
        title="Final versioned head prepared",
        summary=f"PR #{identity.pr_number} final head is {identity.head_sha}.",
    )
    version = _mutate_check(
        api, identity, "version-prepared", version_existing, version_payload
    )
    _assert_unique_mutation(
        api,
        identity,
        "version-prepared",
        version,
        expected_status="completed",
        expected_conclusion="success",
    )

    acceptance_payload = _check_payload(
        identity,
        "acceptance",
        status="in_progress",
        conclusion=None,
        title="Final-head quality is running",
        summary=f"Run {identity.run_id} owns PR #{identity.pr_number} at {identity.head_sha}.",
    )
    acceptance = _mutate_check(
        api, identity, "acceptance", acceptance, acceptance_payload
    )
    _assert_unique_mutation(
        api,
        identity,
        "acceptance",
        acceptance,
        expected_status="in_progress",
        expected_conclusion=None,
    )
    _assert_unique_mutation(
        api,
        identity,
        "version-prepared",
        version,
        expected_status="completed",
        expected_conclusion="success",
    )
    assert acceptance is not None
    return RegisterResult(
        quality_required=True,
        version_check_id=_positive_integer(version["id"], "version check ID"),
        acceptance_check_id=_positive_integer(acceptance["id"], "acceptance check ID"),
    )


def _parse_evidence(path: Path) -> dict[str, str]:
    if not path.is_file():
        raise ReporterError(f"evidence file is missing: {path}")
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if ": " not in line:
            raise ReporterError(f"malformed evidence line: {line!r}")
        key, value = line.split(": ", 1)
        if not key or key in result:
            raise ReporterError(f"duplicate or empty evidence field: {key!r}")
        result[key] = value
    return result


def _verify_sha256s(directory: Path) -> None:
    manifest = directory / "SHA256SUMS"
    if not manifest.is_file():
        raise ReporterError(f"SHA256SUMS is missing in {directory}")
    expected_files: set[str] = set()
    for line in manifest.read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (\./)?(.+)", line)
        if match is None:
            raise ReporterError("SHA256SUMS contains a malformed line")
        relative = match.group(3)
        if relative == "SHA256SUMS" or relative in expected_files:
            raise ReporterError("SHA256SUMS contains a duplicate or recursive entry")
        target = directory / relative
        if not target.is_file() or target.is_symlink():
            raise ReporterError(f"artifact file is missing or unsafe: {relative}")
        actual = hashlib.sha256(target.read_bytes()).hexdigest()
        if actual != match.group(1):
            raise ReporterError(f"artifact digest mismatch: {relative}")
        expected_files.add(relative)
    actual_files = {
        str(path.relative_to(directory))
        for path in directory.rglob("*")
        if path.is_file() and path.name != "SHA256SUMS"
    }
    if expected_files != actual_files:
        raise ReporterError("SHA256SUMS does not cover the exact artifact file set")


def _verify_artifacts(
    identity: Identity,
    *,
    binary_impact: str,
    version: str,
    verdict_directory: Path,
    candidate_directory: Path,
) -> str:
    if binary_impact not in {"true", "false"}:
        raise ReporterError("binary impact output is missing or malformed")
    if binary_impact == "false" and version:
        raise ReporterError("no-binary acceptance unexpectedly contains a version")
    if (
        binary_impact == "true"
        and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is None
    ):
        raise ReporterError("binary acceptance version is missing or malformed")
    _verify_sha256s(verdict_directory)
    verdict = _parse_evidence(verdict_directory / "acceptance.txt")
    expected = {
        "schema": "codex-info-final-head-v1",
        "pr-number": str(identity.pr_number),
        "source-sha": identity.head_sha,
        "binary-impact": binary_impact,
        "version": version,
        "acceptance": "PASS",
    }
    if verdict != expected:
        raise ReporterError("acceptance verdict identity does not match the final head")
    if binary_impact == "true":
        _verify_sha256s(candidate_directory)
        candidate = _parse_evidence(candidate_directory / "acceptance.txt")
        if set(candidate) != {
            "schema",
            "pr-number",
            "source-sha",
            "tree-sha",
            "version",
            "acceptance",
        }:
            raise ReporterError("release candidate evidence fields are not exact")
        for key, value in (
            ("schema", "codex-info-quality-v1"),
            ("pr-number", str(identity.pr_number)),
            ("source-sha", identity.head_sha),
            ("version", version),
            ("acceptance", "PASS"),
        ):
            if candidate.get(key) != value:
                raise ReporterError(f"release candidate {key} mismatch")
        _sha(candidate.get("tree-sha"), "release candidate tree SHA")
    elif candidate_directory.exists() and any(candidate_directory.iterdir()):
        raise ReporterError(
            "no-binary acceptance unexpectedly contains a release candidate"
        )
    return hashlib.sha256((verdict_directory / "SHA256SUMS").read_bytes()).hexdigest()


def finalize_acceptance(
    api: Api,
    identity: Identity,
    *,
    quality_result: str,
    binary_impact: str,
    version: str,
    verdict_directory: Path,
    candidate_directory: Path,
    verdict_artifact_id: str,
    verdict_artifact_digest: str,
    candidate_artifact_id: str,
    candidate_artifact_digest: str,
) -> None:
    validate_identity(identity)
    acceptance_checks = _check_runs(api, identity, "acceptance")
    version_checks = _check_runs(api, identity, "version-prepared")
    if len(acceptance_checks) != 1:
        raise ReporterError("exactly one final-head acceptance check is required")
    if len(version_checks) != 1:
        raise ReporterError("exactly one final-head version-prepared check is required")
    acceptance = acceptance_checks[0]
    version_check = version_checks[0]
    if _required(acceptance, "external_id", "check_run") != identity.external_id:
        raise ReporterError("acceptance check is owned by another workflow run")
    if _required(version_check, "external_id", "check_run") != identity.external_id:
        raise ReporterError("version-prepared check is owned by another workflow run")
    conclusion = "failure"
    detail = "Final-head quality evidence is incomplete or invalid."
    error: Exception | None = None
    try:
        if (
            _required(version_check, "status", "check_run") != "completed"
            or _required(version_check, "conclusion", "check_run") != "success"
        ):
            raise ReporterError("version-prepared check is not successful for this run")
        _validate_pull_request(api, identity, require_open=True)
        if quality_result != "success":
            raise ReporterError(f"quality workflow result is {quality_result!r}")
        evidence_digest = _verify_artifacts(
            identity,
            binary_impact=binary_impact,
            version=version,
            verdict_directory=verdict_directory,
            candidate_directory=candidate_directory,
        )
        _positive_integer(verdict_artifact_id, "acceptance-verdict artifact ID")
        if not DIGEST_PATTERN.fullmatch(verdict_artifact_digest):
            raise ReporterError("acceptance-verdict artifact identity is malformed")
        if binary_impact == "true":
            _positive_integer(candidate_artifact_id, "release-candidate artifact ID")
            if not DIGEST_PATTERN.fullmatch(candidate_artifact_digest):
                raise ReporterError("release-candidate artifact identity is malformed")
        if binary_impact == "false" and (
            candidate_artifact_id or candidate_artifact_digest
        ):
            raise ReporterError("no-binary acceptance has release-candidate metadata")
        conclusion = "success"
        detail = (
            f"run={identity.run_id}; verdict-artifact={verdict_artifact_id}; "
            f"verdict-digest={verdict_artifact_digest}; evidence={evidence_digest}; "
            f"candidate-artifact={candidate_artifact_id or 'none'}; "
            f"candidate-digest={candidate_artifact_digest or 'none'}"
        )
    # Any verification defect must reach a terminal required-check failure;
    # otherwise an unexpected parser or filesystem exception could leave the
    # candidate pending forever.
    except Exception as caught:  # noqa: BLE001
        error = caught
        detail = str(caught)

    payload = _check_payload(
        identity,
        "acceptance",
        status="completed",
        conclusion=conclusion,
        title="Final-head quality accepted"
        if conclusion == "success"
        else "Final-head quality rejected",
        summary=detail,
    )
    mutated = _mutate_check(api, identity, "acceptance", acceptance, payload)
    _assert_unique_mutation(
        api,
        identity,
        "acceptance",
        mutated,
        expected_status="completed",
        expected_conclusion=conclusion,
    )
    _assert_unique_mutation(
        api,
        identity,
        "version-prepared",
        version_check,
        expected_status="completed",
        expected_conclusion="success",
    )
    if error is not None:
        raise ReporterError(detail) from error


def _identity_from_args(args: argparse.Namespace) -> Identity:
    return Identity(
        repository=args.repository,
        pr_number=_positive_integer(args.pr_number, "PR number"),
        base_repository=args.base_repository,
        head_repository=args.head_repository,
        base_sha=args.base_sha,
        head_sha=args.head_sha,
        head_ref=args.head_ref,
        run_id=_positive_integer(args.run_id, "run ID"),
        run_url=args.run_url,
    )


def _add_identity_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--repository", required=True)
    parser.add_argument("--pr-number", required=True)
    parser.add_argument("--base-repository", required=True)
    parser.add_argument("--head-repository", required=True)
    parser.add_argument("--base-sha", required=True)
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--head-ref", required=True)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--run-url", required=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    register_parser = subparsers.add_parser("register")
    _add_identity_arguments(register_parser)
    register_parser.add_argument("--github-output", type=Path, required=True)
    finalize_parser = subparsers.add_parser("finalize")
    _add_identity_arguments(finalize_parser)
    finalize_parser.add_argument("--quality-result", required=True)
    finalize_parser.add_argument("--binary-impact", required=True)
    finalize_parser.add_argument("--version", default="")
    finalize_parser.add_argument("--verdict-directory", type=Path, required=True)
    finalize_parser.add_argument("--candidate-directory", type=Path, required=True)
    finalize_parser.add_argument("--verdict-artifact-id", required=True)
    finalize_parser.add_argument("--verdict-artifact-digest", required=True)
    finalize_parser.add_argument("--candidate-artifact-id", default="")
    finalize_parser.add_argument("--candidate-artifact-digest", default="")
    args = parser.parse_args(argv)
    identity = _identity_from_args(args)
    api = GitHubApi(os.environ.get("GH_TOKEN", ""))
    if args.command == "register":
        result = register_checks(api, identity)
        with args.github_output.open("a", encoding="utf-8") as output:
            output.write(
                f"quality_required={'true' if result.quality_required else 'false'}\n"
            )
            output.write(f"version_check_id={result.version_check_id}\n")
            output.write(f"acceptance_check_id={result.acceptance_check_id}\n")
    else:
        finalize_acceptance(
            api,
            identity,
            quality_result=args.quality_result,
            binary_impact=args.binary_impact,
            version=args.version,
            verdict_directory=args.verdict_directory,
            candidate_directory=args.candidate_directory,
            verdict_artifact_id=args.verdict_artifact_id,
            verdict_artifact_digest=args.verdict_artifact_digest,
            candidate_artifact_id=args.candidate_artifact_id,
            candidate_artifact_digest=args.candidate_artifact_digest,
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ReporterError as error:
        print(f"final-head-check-reporter: FAIL: {error}", file=sys.stderr)
        raise SystemExit(1) from error

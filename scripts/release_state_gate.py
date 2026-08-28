#!/usr/bin/env python3
"""Small fail-closed oracle for the GitHub Release state fixtures.

The command reads one GitHub-shaped JSON object from stdin.  It does not call
the GitHub API: callers own transport and pass the observed object here for a
deterministic state check.  Local Setup/manifest sizes are read from the two
paths supplied to ``draft`` and ``published``.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile
from typing import Any, Callable


SHA_LENGTH = 40


class GateError(ValueError):
    """An observed release state is incomplete or does not match its phase."""


def _require_object(value: Any, label: str = "JSON input") -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GateError(f"{label} must be a JSON object")
    return value


def _require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise GateError(f"{label} must be a non-empty string")
    return value


def _require_bool(value: Any, label: str, expected: bool) -> None:
    # bool is a subclass of int in Python; checking the type explicitly keeps
    # 0/1 and JSON strings from being accepted as GitHub state flags.
    if type(value) is not bool or value is not expected:
        expected_text = "true" if expected else "false"
        raise GateError(f"{label} must be {expected_text}")


def _require_sha(value: Any, label: str) -> str:
    if not isinstance(value, str) or len(value) != SHA_LENGTH:
        raise GateError(f"{label} must be a {SHA_LENGTH}-character hexadecimal SHA")
    if any(character not in "0123456789abcdef" for character in value):
        raise GateError(f"{label} must be lowercase hexadecimal")
    return value


def _require_tag(release: dict[str, Any], expected_tag: str | None) -> None:
    tag = _require_string(release.get("tag_name"), "tag_name")
    if expected_tag is not None and tag != expected_tag:
        raise GateError(f"tag_name does not match expected tag: {tag!r}")


def validate_created(value: Any, expected_tag: str | None = None) -> None:
    release = _require_object(value)
    release_id = release.get("id")
    if type(release_id) is not int or release_id <= 0:
        raise GateError("created release id must be a positive integer")
    _require_tag(release, expected_tag)
    _require_bool(release.get("draft"), "draft", True)
    _require_bool(release.get("prerelease"), "prerelease", False)


def _local_asset_sizes(setup_path: str, manifest_path: str) -> dict[str, int]:
    paths = (Path(setup_path), Path(manifest_path))
    sizes: dict[str, int] = {}
    for path in paths:
        if not path.is_file():
            raise GateError(f"local release asset is missing: {path}")
        name = path.name
        if not name or name in sizes:
            raise GateError("local release asset names must be unique")
        try:
            size = path.stat().st_size
        except OSError as error:
            raise GateError(f"could not stat local release asset: {path}") from error
        if size <= 0:
            raise GateError(f"local release asset must be non-empty: {path}")
        sizes[name] = size
    return sizes


def _require_assets(release: dict[str, Any], expected_sizes: dict[str, int]) -> None:
    assets = release.get("assets")
    if not isinstance(assets, list) or len(assets) != 2:
        raise GateError("release must contain exactly two assets")

    seen: set[str] = set()
    for index, raw_asset in enumerate(assets):
        asset = _require_object(raw_asset, f"asset[{index}]")
        name = _require_string(asset.get("name"), f"asset[{index}].name")
        if name in seen:
            raise GateError(f"duplicate release asset name: {name}")
        seen.add(name)
        if name not in expected_sizes:
            raise GateError(f"unexpected release asset name: {name}")

        size = asset.get("size")
        if type(size) is not int or size != expected_sizes[name]:
            raise GateError(f"release asset size does not match local file: {name}")
        if asset.get("state") != "uploaded":
            raise GateError(f"release asset is not uploaded: {name}")

    if seen != set(expected_sizes):
        raise GateError("release assets are not an exact name set")


def validate_phase(
    value: Any,
    phase: str,
    setup_path: str,
    manifest_path: str,
    expected_tag: str | None = None,
) -> None:
    if phase not in {"draft", "published"}:
        raise GateError(f"unsupported release phase: {phase}")
    release = _require_object(value)
    _require_tag(release, expected_tag)
    _require_bool(release.get("prerelease"), "prerelease", False)
    if phase == "draft":
        _require_bool(release.get("draft"), "draft", True)
    else:
        _require_bool(release.get("draft"), "draft", False)
        _require_string(release.get("published_at"), "published_at")
    _require_assets(release, _local_asset_sizes(setup_path, manifest_path))


def validate_tag(value: Any, expected_sha: str) -> None:
    expected = _require_sha(expected_sha, "expected commit SHA")
    tag = _require_object(value)
    obj = _require_object(tag.get("object"), "tag.object")
    if obj.get("type") != "commit":
        raise GateError("tag object must identify a commit")
    observed = _require_sha(obj.get("sha"), "tag object SHA")
    if observed != expected:
        raise GateError(
            f"tag commit SHA does not match expected SHA: observed={observed} expected={expected}"
        )


def _read_json() -> Any:
    try:
        return json.load(sys.stdin)
    except (json.JSONDecodeError, OSError) as error:
        raise GateError(f"invalid JSON input: {error}") from error


def _run_self_test() -> int:
    cases = 0

    def accepted(name: str, callback: Callable[[], None]) -> None:
        nonlocal cases
        try:
            callback()
        except GateError as error:
            raise AssertionError(f"baseline case was rejected: {name}: {error}") from error
        cases += 1

    def rejected(name: str, callback: Callable[[], None]) -> None:
        nonlocal cases
        try:
            callback()
        except GateError:
            cases += 1
            return
        raise AssertionError(f"invalid case was accepted: {name}")

    commit_sha = "a" * SHA_LENGTH
    other_sha = "b" * SHA_LENGTH
    tag_name = "windows-v1.0.9"

    with tempfile.TemporaryDirectory(prefix="codex-info-release-state-") as directory:
        root = Path(directory)
        setup = root / "CodexInfo.WindowsClient.Setup.exe"
        manifest = root / "CodexInfo.WindowsClient.update.json"
        setup.write_bytes(b"setup")
        manifest.write_bytes(b"manifest")
        assets = [
            {"name": setup.name, "size": setup.stat().st_size, "state": "uploaded"},
            {"name": manifest.name, "size": manifest.stat().st_size, "state": "uploaded"},
        ]
        created = {
            "id": 17,
            "tag_name": tag_name,
            "draft": True,
            "prerelease": False,
        }
        draft = {
            "tag_name": tag_name,
            "draft": True,
            "prerelease": False,
            "assets": assets,
        }
        published = {
            "tag_name": tag_name,
            "draft": False,
            "prerelease": False,
            "published_at": "2026-08-28T00:00:00Z",
            "assets": assets,
        }
        tag = {"object": {"type": "commit", "sha": commit_sha}}

        accepted("created", lambda: validate_created(created, tag_name))
        accepted(
            "draft",
            lambda: validate_phase(draft, "draft", str(setup), str(manifest), tag_name),
        )
        accepted(
            "published",
            lambda: validate_phase(
                published, "published", str(setup), str(manifest), tag_name
            ),
        )
        accepted("tag", lambda: validate_tag(tag, commit_sha))

        missing_assets = dict(draft)
        missing_assets.pop("assets")
        rejected(
            "missing assets",
            lambda: validate_phase(
                missing_assets, "draft", str(setup), str(manifest), tag_name
            ),
        )

        duplicate_assets = dict(draft)
        duplicate_assets["assets"] = [assets[0], dict(assets[0])]
        rejected(
            "duplicate asset",
            lambda: validate_phase(
                duplicate_assets, "draft", str(setup), str(manifest), tag_name
            ),
        )

        wrong_name = dict(draft)
        wrong_name["assets"] = [dict(assets[0]), dict(assets[1])]
        wrong_name["assets"][0]["name"] = "unexpected.exe"
        rejected(
            "wrong asset name",
            lambda: validate_phase(
                wrong_name, "draft", str(setup), str(manifest), tag_name
            ),
        )

        wrong_size = dict(draft)
        wrong_size["assets"] = [dict(assets[0]), dict(assets[1])]
        wrong_size["assets"][0]["size"] += 1
        rejected(
            "wrong asset size",
            lambda: validate_phase(
                wrong_size, "draft", str(setup), str(manifest), tag_name
            ),
        )

        setup.write_bytes(b"")
        rejected(
            "zero-byte Setup asset",
            lambda: validate_phase(
                draft, "draft", str(setup), str(manifest), tag_name
            ),
        )
        setup.write_bytes(b"setup")

        manifest.write_bytes(b"")
        rejected(
            "zero-byte manifest asset",
            lambda: validate_phase(
                draft, "draft", str(setup), str(manifest), tag_name
            ),
        )
        manifest.write_bytes(b"manifest")

        wrong_state = dict(draft)
        wrong_state["assets"] = [dict(assets[0]), dict(assets[1])]
        wrong_state["assets"][0]["state"] = "created"
        rejected(
            "wrong asset state",
            lambda: validate_phase(
                wrong_state, "draft", str(setup), str(manifest), tag_name
            ),
        )

        missing_published_time = dict(published)
        missing_published_time.pop("published_at")
        rejected(
            "missing published time",
            lambda: validate_phase(
                missing_published_time,
                "published",
                str(setup),
                str(manifest),
                tag_name,
            ),
        )

        empty_published_time = dict(published)
        empty_published_time["published_at"] = ""
        rejected(
            "empty published time",
            lambda: validate_phase(
                empty_published_time,
                "published",
                str(setup),
                str(manifest),
                tag_name,
            ),
        )

        wrong_published_phase = dict(published)
        wrong_published_phase["draft"] = True
        rejected(
            "published release remains draft",
            lambda: validate_phase(
                wrong_published_phase,
                "published",
                str(setup),
                str(manifest),
                tag_name,
            ),
        )

        missing_created_id = dict(created)
        missing_created_id.pop("id")
        rejected("missing created id", lambda: validate_created(missing_created_id, tag_name))

        wrong_tag_name = dict(created)
        wrong_tag_name["tag_name"] = "windows-v1.0.8"
        rejected("wrong release tag", lambda: validate_created(wrong_tag_name, tag_name))

        wrong_tag_type = {"object": {"type": "tag", "sha": commit_sha}}
        rejected("wrong tag object type", lambda: validate_tag(wrong_tag_type, commit_sha))

        wrong_tag_sha = {"object": {"type": "commit", "sha": other_sha}}
        rejected("wrong tag commit", lambda: validate_tag(wrong_tag_sha, commit_sha))

        missing_tag_object = {}
        rejected("missing tag object", lambda: validate_tag(missing_tag_object, commit_sha))

    if cases <= 0:
        raise AssertionError("fixture case count is zero")
    print(f"release-state-gate: PASS cases={cases}")
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run finite baseline and rejection fixtures",
    )
    subparsers = parser.add_subparsers(dest="command")

    created = subparsers.add_parser("created", help="validate a newly created draft release")
    created.add_argument("--tag", "--expected-tag", dest="expected_tag")

    for phase in ("draft", "published"):
        phase_parser = subparsers.add_parser(phase, help=f"validate a {phase} release")
        phase_parser.add_argument("--setup", required=True, help="local Setup asset")
        phase_parser.add_argument("--manifest", required=True, help="local update manifest")
        phase_parser.add_argument("--tag", "--expected-tag", dest="expected_tag")

    tag = subparsers.add_parser("tag", help="validate a tag ref's commit target")
    tag.add_argument(
        "expected_sha_positional",
        nargs="?",
        help="expected commit SHA (may instead be supplied with --commit-sha)",
    )
    tag.add_argument(
        "--commit-sha",
        "--expected-sha",
        "--sha",
        dest="expected_sha",
        help="expected commit SHA",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _parser()
    arguments = parser.parse_args(argv)
    if arguments.self_test:
        try:
            return _run_self_test()
        except (AssertionError, OSError) as error:
            print(f"release-state-gate: FAIL: {error}", file=sys.stderr)
            return 1
    if arguments.command is None:
        parser.error("a subcommand or --self-test is required")

    try:
        value = _read_json()
        if arguments.command == "created":
            validate_created(value, arguments.expected_tag)
        elif arguments.command in {"draft", "published"}:
            validate_phase(
                value,
                arguments.command,
                arguments.setup,
                arguments.manifest,
                arguments.expected_tag,
            )
        else:
            expected_sha = arguments.expected_sha or arguments.expected_sha_positional
            if expected_sha is None:
                raise GateError("tag requires an expected commit SHA")
            validate_tag(value, expected_sha)
    except (GateError, OSError) as error:
        print(f"release-state-gate: HOLD: {error}", file=sys.stderr)
        return 2

    print(f"release-state-gate: PASS command={arguments.command}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

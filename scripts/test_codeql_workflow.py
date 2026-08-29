#!/usr/bin/env python3
"""Finite mutation contract for immutable-source CodeQL attribution."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CODEQL = ROOT / ".github" / "workflows" / "codeql.yml"
WINDOWS = ROOT / ".github" / "workflows" / "windows-client.yml"
EXPECTED_LANGUAGES = ("actions", "csharp", "python", "rust")


def validate(codeql: str, windows: str) -> list[str]:
    errors: list[str] = []

    def exact(source: str, marker: str, count: int = 1) -> None:
        actual = source.count(marker)
        if actual != count:
            errors.append(f"count {marker!r}: expected {count}, found {actual}")

    for trigger in ("  workflow_call:\n", "  schedule:\n"):
        exact(codeql, trigger)
    for forbidden in ("  workflow_dispatch:\n", "  pull_request:\n", "  push:\n"):
        exact(codeql, forbidden, 0)
    exact(codeql, "      source_sha:\n")
    exact(codeql, "      head_ref:\n")
    exact(codeql, "          ref: ${{ inputs.source_sha || github.sha }}\n")
    exact(
        codeql,
        "          ref: ${{ inputs.source_sha && format('refs/heads/{0}', inputs.head_ref) || github.ref }}\n",
    )
    exact(codeql, "          sha: ${{ inputs.source_sha || github.sha }}\n")
    exact(codeql, "  security-events: write\n")
    exact(codeql, "github/codeql-action/init@v4")
    exact(codeql, "github/codeql-action/analyze@v4")
    exact(codeql, "github/codeql-action/autobuild@", 0)
    for language in EXPECTED_LANGUAGES:
        exact(codeql, f"          - language: {language}\n")
    exact(codeql, "            build-mode: none\n", len(EXPECTED_LANGUAGES))
    exact(windows, "    uses: ./.github/workflows/codeql.yml\n")
    exact(windows, "      source_sha: ${{ inputs.head_sha }}\n", 2)
    exact(windows, "      head_ref: ${{ inputs.head_ref }}\n")
    exact(
        windows,
        "needs: [version-prepared, native-quality, codeql-analysis, windows-quality, ui-quality]",
    )
    exact(windows, "          CODEQL_RESULT: ${{ needs.codeql-analysis.result }}\n")
    return errors


def main() -> int:
    codeql = CODEQL.read_text(encoding="utf-8")
    windows = WINDOWS.read_text(encoding="utf-8")
    baseline = validate(codeql, windows)
    if baseline:
        raise AssertionError(
            "production CodeQL contract failed: " + "; ".join(baseline)
        )
    mutations = (
        ("codeql", "  workflow_call:\n", ""),
        ("codeql", "          - language: rust\n", ""),
        (
            "codeql",
            "            build-mode: none\n",
            "            build-mode: autobuild\n",
        ),
        ("codeql", "          sha: ${{ inputs.source_sha || github.sha }}\n", ""),
        ("codeql", "  security-events: write\n", ""),
        ("windows", "      head_ref: ${{ inputs.head_ref }}\n", ""),
        (
            "windows",
            "needs: [version-prepared, native-quality, codeql-analysis, windows-quality, ui-quality]",
            "needs: [version-prepared, native-quality, windows-quality, ui-quality]",
        ),
    )
    cases = 1
    for target, needle, replacement in mutations:
        candidate_codeql = codeql
        candidate_windows = windows
        if target == "codeql":
            candidate_codeql = candidate_codeql.replace(needle, replacement, 1)
        else:
            candidate_windows = candidate_windows.replace(needle, replacement, 1)
        if not validate(candidate_codeql, candidate_windows):
            raise AssertionError(f"CodeQL workflow mutation was accepted: {needle!r}")
        cases += 1
    print(f"codeql-workflow-test: PASS cases={cases}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

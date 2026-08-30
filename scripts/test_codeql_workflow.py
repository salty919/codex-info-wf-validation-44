#!/usr/bin/env python3
"""Finite mutation contract for selective immutable-source CodeQL attribution."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CODEQL = ROOT / ".github" / "workflows" / "codeql.yml"
WINDOWS = ROOT / ".github" / "workflows" / "windows-client.yml"
SELECTIVE = ROOT / ".github" / "workflows" / "selective-quality.yml"


def validate(codeql: str, windows: str, selective: str) -> list[str]:
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
    exact(codeql, "      languages_json:\n")
    exact(codeql, "        default: '[\"actions\",\"csharp\",\"python\",\"rust\"]'\n")
    exact(
        codeql,
        "        language: ${{ fromJSON(inputs.languages_json || '[\"actions\",\"csharp\",\"python\",\"rust\"]') }}\n",
    )
    exact(codeql, "          ref: ${{ inputs.source_sha || github.sha }}\n")
    exact(codeql, "          sha: ${{ inputs.source_sha || github.sha }}\n")
    exact(codeql, "          build-mode: none\n")
    exact(codeql, "  security-events: write\n")
    exact(codeql, "github/codeql-action/init@v4")
    exact(codeql, "github/codeql-action/analyze@v4")
    exact(codeql, "github/codeql-action/autobuild@", 0)

    exact(windows, "    uses: ./.github/workflows/codeql.yml\n", 0)
    exact(selective, "  codeql-quality:\n")
    exact(selective, "    if: inputs.codeql_languages_json != '[]'\n")
    exact(selective, "    uses: ./.github/workflows/codeql.yml\n")
    exact(selective, "      languages_json: ${{ inputs.codeql_languages_json }}\n")
    exact(selective, "      security-events: write\n")
    exact(selective, "      - codeql-quality\n")
    return errors


def main() -> int:
    sources = [
        CODEQL.read_text(encoding="utf-8"),
        WINDOWS.read_text(encoding="utf-8"),
        SELECTIVE.read_text(encoding="utf-8"),
    ]
    baseline = validate(*sources)
    if baseline:
        raise AssertionError("production CodeQL contract failed: " + "; ".join(baseline))
    mutations = (
        (0, "  workflow_call:\n", ""),
        (0, "      languages_json:\n", "      languages:\n"),
        (0, "          build-mode: none\n", "          build-mode: autobuild\n"),
        (0, "          sha: ${{ inputs.source_sha || github.sha }}\n", ""),
        (0, "  security-events: write\n", ""),
        (
            1,
            "  ui-quality:\n",
            "  duplicate-codeql:\n    uses: ./.github/workflows/codeql.yml\n\n  ui-quality:\n",
        ),
        (2, "    if: inputs.codeql_languages_json != '[]'\n", ""),
        (2, "      languages_json: ${{ inputs.codeql_languages_json }}\n", ""),
        (2, "      - codeql-quality\n", ""),
    )
    cases = 1
    for index, needle, replacement in mutations:
        candidate = sources.copy()
        if candidate[index].count(needle) < 1:
            raise AssertionError(f"missing mutation target: {needle!r}")
        candidate[index] = candidate[index].replace(needle, replacement, 1)
        if not validate(*candidate):
            raise AssertionError(f"CodeQL workflow mutation was accepted: {needle!r}")
        cases += 1
    print(f"codeql-workflow-test: PASS cases={cases}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env bash
set -euo pipefail

# Evidence-only join for the native release/CLI/recorder owners, the merge
# policy audit, and the UI owner.  This script deliberately does not invoke
# cargo, dotnet, a UI runner, or any other test command.  The producing jobs
# own those operations; this boundary only proves that their immutable result
# bundles describe the same committed source tree.

hold() {
    echo "quality-artifact-gate: HOLD: $*" >&2
    exit 2
}

native_dir="${1:-}"
windows_dir="${2:-}"
ui_dir="${3:-}"
expected_sha="${4:-${EXPECTED_E2E_SOURCE_SHA:-${GITHUB_SHA:-}}}"
expected_tree="${5:-${EXPECTED_SOURCE_TREE_SHA:-}}"

[[ -n "$native_dir" && -n "$windows_dir" && -n "$ui_dir" ]] ||
    hold "native, Windows, and UI evidence directories are required"
[[ "$expected_sha" =~ ^[0-9a-f]{40}$ ]] || hold "expected source SHA is required"

if [[ -z "$expected_tree" ]]; then
    expected_tree="$(git rev-parse 'HEAD^{tree}' 2>/dev/null || true)"
fi
[[ "$expected_tree" =~ ^[0-9a-f]{40}$ ]] || hold "expected source tree SHA is required"

current_sha="$(git rev-parse HEAD 2>/dev/null || true)"
current_tree="$(git rev-parse 'HEAD^{tree}' 2>/dev/null || true)"
[[ "$current_sha" == "$expected_sha" ]] ||
    hold "checked out source SHA does not match expected SHA: current=$current_sha expected=$expected_sha"
[[ "$current_tree" == "$expected_tree" ]] ||
    hold "checked out source tree does not match expected tree: current=$current_tree expected=$expected_tree"

verify_bundle() {
    local directory="$1" marker_file="$2" quality_marker="$3"
    [[ -d "$directory" ]] || hold "quality evidence directory is missing: $directory"
    [[ -f "$directory/$marker_file" ]] || hold "quality marker is missing: $directory/$marker_file"
    [[ -f "$directory/SHA256SUMS" ]] || hold "quality hash manifest is missing: $directory/SHA256SUMS"
    grep -Fxq "schema: codex-info-quality-v1" "$directory/$marker_file" ||
        hold "quality evidence schema is invalid: $directory/$marker_file"
    grep -Fxq "source-sha: $expected_sha" "$directory/$marker_file" ||
        hold "quality evidence source SHA does not match: $directory/$marker_file"
    grep -Fxq "tree-sha: $expected_tree" "$directory/$marker_file" ||
        hold "quality evidence tree SHA does not match: $directory/$marker_file"
    grep -Fxq "$quality_marker" "$directory/$marker_file" ||
        hold "quality PASS marker is missing: $directory/$marker_file"

    local listed_count actual_count
    listed_count="$(awk 'NF >= 2 { count++ } END { print count + 0 }' "$directory/SHA256SUMS")"
    actual_count="$(find "$directory" -type f ! -name SHA256SUMS -print | wc -l)"
    [[ "$listed_count" == "$actual_count" ]] ||
        hold "quality hash manifest does not cover the bundle: $directory listed=$listed_count actual=$actual_count"
    (cd "$directory" && sha256sum -c SHA256SUMS >/dev/null) ||
        hold "quality artifact hash verification failed: $directory"
}

verify_bundle "$native_dir" native-quality.txt 'native-quality: PASS'
grep -Fxq 'quality: native' "$native_dir/native-quality.txt" ||
    hold 'native quality marker is missing'
grep -Fxq 'release-build: PASS' "$native_dir/native-quality.txt" ||
    hold 'native release-build PASS marker is missing'
grep -Fxq 'cli-contract-e2e: PASS' "$native_dir/native-quality.txt" ||
    hold 'native CLI contract E2E PASS marker is missing'
grep -Fxq 'recorder-daemon: PASS' "$native_dir/native-quality.txt" ||
    hold 'recorder daemon PASS marker is missing'
for obsolete_marker in 'regression-guard: PASS' 'data-protection: PASS'; do
    if grep -Fxq "$obsolete_marker" "$native_dir/native-quality.txt"; then
        hold "native artifact retains obsolete marker: $obsolete_marker"
    fi
done

verify_bundle "$windows_dir" windows-quality.txt 'merge-policy: PASS'
grep -Fxq 'quality: merge-policy' "$windows_dir/windows-quality.txt" ||
    hold 'merge-policy quality marker is missing'
grep -Fxq 'live-applied-rules: PASS' "$windows_dir/windows-quality.txt" ||
    hold 'live applied-rules PASS marker is missing'
for obsolete_marker in 'quality: windows' 'windows-contract: PASS' 'windows-tests: PASS' 'windows-quality: PASS'; do
    if grep -Fxq "$obsolete_marker" "$windows_dir/windows-quality.txt"; then
        hold "merge-policy artifact retains obsolete marker: $obsolete_marker"
    fi
done

verify_bundle "$ui_dir" ui-quality.txt 'ui-quality: PASS'
grep -Fxq 'windows-ui-e2e: PASS' "$ui_dir/ui-quality.txt" ||
    hold 'Windows UI E2E PASS marker is missing'
grep -Fxq 'window-move-smoke: PASS' "$ui_dir/ui-quality.txt" ||
    hold 'physical window-move PASS marker is missing'

echo "quality-artifact-gate: PASS source-sha=$expected_sha tree-sha=$expected_tree"

#!/usr/bin/env bash
set -euo pipefail

# This is the single PR acceptance boundary.  It is intentionally
# fail-closed, but evidence-only: the selected Windows owner executes and
# uploads its immutable UI bundle; this script verifies that bundle. Other
# selected owners are accepted by the shared selected-quality gate.
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

hold() {
    echo "final-acceptance-gate: HOLD: $*" >&2
    exit 2
}

evidence_dir="${1:-${WINDOWS_E2E_EVIDENCE_DIR:-}}"
[[ -n "$evidence_dir" ]] || hold "Windows UI E2E evidence directory is required"
[[ -d "$evidence_dir" ]] || hold "Windows UI E2E evidence directory is missing: $evidence_dir"

log_file="$evidence_dir/windows-client-e2e.log"
[[ -f "$log_file" ]] || hold "Windows UI E2E log is missing"
move_log="$evidence_dir/window-move-smoke.log"
[[ -f "$move_log" ]] || hold "physical window move smoke log is missing"
expected_sha="${EXPECTED_E2E_SOURCE_SHA:-${GITHUB_SHA:-}}"
[[ "$expected_sha" =~ ^[0-9a-f]{40}$ ]] || hold "expected E2E source SHA is required"
expected_tree="${EXPECTED_SOURCE_TREE_SHA:-$(git rev-parse 'HEAD^{tree}' 2>/dev/null || true)}"
[[ "$expected_tree" =~ ^[0-9a-f]{40}$ ]] || hold "expected source tree SHA is required"
if [[ -n "$(git status --porcelain=v1 --untracked-files=all)" ]]; then
    hold "source tree is dirty; release evidence must match a clean committed revision"
fi
current_sha="$(git rev-parse HEAD 2>/dev/null || true)"
current_tree="$(git rev-parse 'HEAD^{tree}' 2>/dev/null || true)"
[[ "$current_sha" == "$expected_sha" ]] ||
    hold "checked out source SHA does not match expected source SHA: current=$current_sha expected=$expected_sha"
[[ "$current_tree" == "$expected_tree" ]] ||
    hold "checked out source tree does not match expected tree: current=$current_tree expected=$expected_tree"

grep -Fq -- "source-sha: $expected_sha" "$log_file" \
    || hold "Windows UI E2E evidence was not produced from expected source SHA: $expected_sha"
grep -Fq -- "source-sha: $expected_sha" "$move_log" \
    || hold "physical window move smoke evidence was not produced from expected source SHA: $expected_sha"
grep -Fq -- 'window-move-smoke: PASS' "$move_log" \
    || hold "physical window move smoke PASS marker is missing"
if grep -Fq -- 'window-move-smoke: SKIP' "$move_log"; then
    hold "physical window move smoke was skipped"
fi
grep -Fq -- 'fixture: PASS periods=2 threads=3 endpoint=http://127.0.0.1:8787' "$log_file" \
    || hold "Windows fixture PASS marker is missing"
grep -Fq -- 'main-quota-gauge: seven cells, two X-authority surface colors, and half-period boundary PASS' "$log_file" \
    || hold "Windows quota gauge evidence is missing"
grep -Fq -- 'main-product-version: PASS' "$log_file" \
    || hold "Windows main product-version evidence is missing"
grep -Fq -- 'child-product-version: PASS role=Graph count=0' "$log_file" \
    || hold "Windows Graph child product-version evidence is missing"
grep -Fq -- 'child-product-version: PASS role=Threads count=0' "$log_file" \
    || hold "Windows Threads child product-version evidence is missing"
grep -Fq -- 'graph-past-model-data: PASS' "$log_file" \
    || hold "Windows past-period graph model-data evidence is missing"
grep -Fq -- 'graph-past-idle-band: PASS' "$log_file" \
    || hold "Windows past-period idle-band color evidence is missing"
grep -Fq -- 'main-details-status: PASS (matching status/details generation accepted)' "$log_file" \
    || hold "Windows main status/details generation evidence is missing"
grep -Fq -- 'windows-client-e2e: PASS' "$log_file" \
    || hold "Windows UI Automation PASS marker is missing"

# The E2E path is finite by design.  Require every named observation rather
# than accepting an arbitrary count of unrelated PNGs.  This prevents a
# stale/partial run from being made to look complete by copying old images.
required_captures=(
    01-main-ready
    02-graph-current
    03-graph-past
    04-graph-current-again
    05-graph-other-metric
    06-graph-initial-metric
    07-toggle-Remaining-before
    08-toggle-Remaining-off
    09-toggle-Remaining-on
    07-toggle-LUNA-before
    08-toggle-LUNA-off
    09-toggle-LUNA-on
    07-toggle-TERRA-before
    08-toggle-TERRA-off
    09-toggle-TERRA-on
    07-toggle-SOL-before
    08-toggle-SOL-off
    09-toggle-SOL-on
    10-threads-rows
)
for capture_name in "${required_captures[@]}"; do
    capture_path="$evidence_dir/$capture_name.png"
    [[ -s "$capture_path" ]] || hold "Windows UI screenshot is missing or empty: $capture_name.png"
    [[ "$(wc -c < "$capture_path")" -gt 100 ]] || hold "Windows UI screenshot is truncated: $capture_name.png"
    png_signature="$(head -c 8 "$capture_path" | od -An -tx1 | tr -d '[:space:]')"
    [[ "$png_signature" == '89504e470d0a1a0a' ]] ||
        hold "Windows UI screenshot is not a PNG: $capture_name.png"
    capture_line="$(grep -F -- "capture: name=$capture_name " "$log_file" | tail -n 1 || true)"
    [[ -n "$capture_line" ]] || hold "Windows UI capture hash marker is missing: $capture_name.png"
    logged_hash="$(sed -n 's/.* sha256=\([0-9a-fA-F]\{64\}\) .*/\1/p' <<<"$capture_line" | tr '[:upper:]' '[:lower:]')"
    [[ "$logged_hash" =~ ^[0-9a-f]{64}$ ]] || hold "Windows UI capture hash marker is malformed: $capture_name.png"
    actual_hash="$(sha256sum "$capture_path" | cut -d' ' -f1)"
    [[ "$actual_hash" == "$logged_hash" ]] || hold "Windows UI screenshot hash does not match its E2E log: $capture_name.png"
done

echo "final-acceptance-gate: PASS (source-matched Windows UI evidence, physical window-move smoke, and source-matched screenshots)"

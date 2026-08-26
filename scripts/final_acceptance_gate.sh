#!/usr/bin/env bash
set -euo pipefail

# This is the release boundary.  It is intentionally fail-closed: a local
# developer may run the deterministic Rust checks, but release acceptance also
# requires the raw Windows UI Automation evidence produced by the Windows
# runner.  No environment variable can replace that evidence.
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
if [[ -n "$(git status --porcelain=v1 --untracked-files=all)" ]]; then
    hold "source tree is dirty; release evidence must match a clean committed revision"
fi
rg -q --fixed-strings "source-sha: $expected_sha" "$log_file" \
    || hold "Windows UI E2E evidence was not produced from expected source SHA: $expected_sha"
rg -q --fixed-strings "source-sha: $expected_sha" "$move_log" \
    || hold "physical window move smoke evidence was not produced from expected source SHA: $expected_sha"
rg -q --fixed-strings 'window-move-smoke: PASS' "$move_log" \
    || hold "physical window move smoke PASS marker is missing"
if rg -q --fixed-strings 'window-move-smoke: SKIP' "$move_log"; then
    hold "physical window move smoke was skipped"
fi
rg -q --fixed-strings 'fixture: PASS periods=2 threads=3 endpoint=http://127.0.0.1:8787' "$log_file" \
    || hold "Windows fixture PASS marker is missing"
rg -q --fixed-strings 'main-quota-gauge: seven cells, two X-authority surface colors, and half-period boundary PASS' "$log_file" \
    || hold "Windows quota gauge evidence is missing"
rg -q --fixed-strings 'main-product-version: PASS' "$log_file" \
    || hold "Windows main product-version evidence is missing"
rg -q --fixed-strings 'child-product-version: PASS role=Graph count=0' "$log_file" \
    || hold "Windows Graph child product-version evidence is missing"
rg -q --fixed-strings 'child-product-version: PASS role=Threads count=0' "$log_file" \
    || hold "Windows Threads child product-version evidence is missing"
rg -q --fixed-strings 'graph-past-model-data: PASS' "$log_file" \
    || hold "Windows past-period graph model-data evidence is missing"
rg -q --fixed-strings 'graph-past-idle-band: PASS' "$log_file" \
    || hold "Windows past-period idle-band color evidence is missing"
rg -q --fixed-strings 'main-details-status: PASS (matching status/details generation accepted)' "$log_file" \
    || hold "Windows main status/details generation evidence is missing"
rg -q --fixed-strings 'windows-client-e2e: PASS' "$log_file" \
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
    capture_line="$(rg --fixed-strings "capture: name=$capture_name " "$log_file" | tail -n 1 || true)"
    [[ -n "$capture_line" ]] || hold "Windows UI capture hash marker is missing: $capture_name.png"
    logged_hash="$(sed -n 's/.* sha256=\([0-9a-fA-F]\{64\}\) .*/\1/p' <<<"$capture_line" | tr '[:upper:]' '[:lower:]')"
    [[ "$logged_hash" =~ ^[0-9a-f]{64}$ ]] || hold "Windows UI capture hash marker is malformed: $capture_name.png"
    actual_hash="$(sha256sum "$capture_path" | cut -d' ' -f1)"
    [[ "$actual_hash" == "$logged_hash" ]] || hold "Windows UI screenshot hash does not match its E2E log: $capture_name.png"
done

command -v cargo >/dev/null || hold "cargo is unavailable; Rust acceptance was not executed"
# regression_guard is the single native acceptance owner. It runs format,
# commit/tree diff checks, all-target check/test (with a non-zero test count),
# exact mandatory regression tests, and the release build without duplicating
# those expensive operations here.
bash scripts/regression_guard.sh || hold "regression guard failed"

echo "final-acceptance-gate: PASS (Rust checks, mandatory history/graph tests, Windows UI E2E, physical window-move smoke, and source-matched screenshots)"

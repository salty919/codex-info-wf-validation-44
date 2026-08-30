#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
    echo "final-acceptance-gate-test: FAIL: $*" >&2
    exit 1
}

work="$(mktemp -d "${TMPDIR:-/tmp}/codex-info-final-acceptance-test.XXXXXX")" ||
    fail "could not create temporary fixture directory"
case "$work" in
    "${TMPDIR:-/tmp}"/codex-info-final-acceptance-test.*) ;;
    *) fail "temporary fixture path is outside the bounded prefix: $work" ;;
esac
cleanup() {
    rm -rf -- "$work"
}
trap cleanup EXIT HUP INT TERM

clean_tree="$work/clean-tree"
fixture_repo="$work/fixture-repo"
evidence_root="$work/evidence"
tool_dir="$work/tools"
ui_dir="$evidence_root/windows-ui-e2e"
mkdir -p \
    "$clean_tree" \
    "$fixture_repo/scripts" \
    "$tool_dir" \
    "$ui_dir"

# The gate deliberately rejects a dirty source tree.  Use an archive of HEAD
# for its Git view while executing the exact working-copy scripts under test.
git archive HEAD | tar -xf - -C "$clean_tree"
git_dir="$(git rev-parse --absolute-git-dir)"
[[ -z "$(GIT_DIR="$git_dir" GIT_WORK_TREE="$clean_tree" git status --porcelain=v1 --untracked-files=all)" ]] ||
    fail "clean HEAD fixture is unexpectedly dirty"
cp scripts/final_acceptance_gate.sh "$fixture_repo/scripts/"

source_sha="$(git rev-parse HEAD)"
tree_sha="$(git rev-parse 'HEAD^{tree}')"

printf 'schema: codex-info-quality-v1\nquality: ui\nsource-sha: %s\ntree-sha: %s\nwindows-ui-e2e: PASS\nwindow-move-smoke: PASS\nui-quality: PASS\n' \
    "$source_sha" "$tree_sha" > "$ui_dir/ui-quality.txt"
printf 'source-sha: %s\nwindow-move-smoke: PASS\n' "$source_sha" > "$ui_dir/window-move-smoke.log"
printf 'source-sha: %s\n' "$source_sha" > "$ui_dir/windows-client-e2e.log"
printf '%s\n' \
    'fixture: PASS periods=2 threads=3 endpoint=http://127.0.0.1:8787' \
    'main-quota-gauge: seven cells, two X-authority surface colors, and half-period boundary PASS' \
    'main-product-version: PASS' \
    'child-product-version: PASS role=Graph count=0' \
    'child-product-version: PASS role=Threads count=0' \
    'graph-past-model-data: PASS' \
    'graph-past-idle-band: PASS' \
    'main-details-status: PASS (matching status/details generation accepted)' \
    'windows-client-e2e: PASS' >> "$ui_dir/windows-client-e2e.log"

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
    capture_path="$ui_dir/$capture_name.png"
    {
        printf '\x89PNG\r\n\x1a\n'
        head -c 120 /dev/zero
    } > "$capture_path"
    capture_hash="$(sha256sum "$capture_path" | cut -d' ' -f1)"
    printf 'capture: name=%s sha256=%s bytes=128\n' "$capture_name" "$capture_hash" >> \
        "$ui_dir/windows-client-e2e.log"
done

make_manifest() {
    local directory="$1" manifest
    manifest="$(mktemp "$work/manifest.XXXXXX")" || fail "could not create hash manifest"
    (
        cd "$directory"
        find . -type f ! -name SHA256SUMS -print0 | LC_ALL=C sort -z |
            xargs -0 sha256sum > "$manifest"
    ) || fail "could not hash fixture bundle: $directory"
    mv "$manifest" "$directory/SHA256SUMS"
}
make_manifest "$ui_dir"

# Only the commands used by the evidence gate are available at runtime.
# ripgrep is intentionally absent, so any spelling that actually invokes it
# fails without relying on a partial shell parser.
required_tools=(awk bash cut dirname find git grep head od sed sha256sum tail tr wc)
for tool_name in "${required_tools[@]}"; do
    tool_path="$(command -v "$tool_name" || true)"
    [[ -n "$tool_path" ]] || fail "required fixture tool is unavailable: $tool_name"
    ln -s "$tool_path" "$tool_dir/$tool_name"
done
if PATH="$tool_dir" "$tool_dir/bash" -c 'command -v rg >/dev/null 2>&1'; then
    fail "isolated acceptance PATH unexpectedly contains rg"
fi

cases=0
expect_missing_rg() {
    local name="$1" command_text="$2"
    if PATH="$tool_dir" "$tool_dir/bash" -o pipefail -c "$command_text" >/dev/null 2>&1; then
        fail "rg invocation unexpectedly succeeded: $name"
    fi
    cases=$((cases + 1))
}
expect_missing_rg direct 'rg --version'
expect_missing_rg quoted '"rg" --version'
expect_missing_rg escaped 'r\g --version'
expect_missing_rg quoted_hash 'printf "%s\n" "# marker"; rg --version'
expect_missing_rg pipeline 'printf x | rg x'
PATH="$tool_dir" "$tool_dir/bash" -c 'printf "%s\n" "text rg marker" >/dev/null' ||
    fail "non-executing rg text was rejected"
cases=$((cases + 1))

gate_output="$(
    PATH="$tool_dir" \
    GIT_DIR="$git_dir" \
    GIT_WORK_TREE="$clean_tree" \
    EXPECTED_E2E_SOURCE_SHA="$source_sha" \
    EXPECTED_SOURCE_TREE_SHA="$tree_sha" \
        "$tool_dir/bash" "$fixture_repo/scripts/final_acceptance_gate.sh" \
        "$ui_dir" 2>&1
)" || {
    printf '%s\n' "$gate_output" >&2
    fail "final acceptance gate failed without rg in PATH"
}
grep -Fq -- 'final-acceptance-gate: PASS' <<<"$gate_output" ||
    fail "final acceptance terminal PASS marker is missing"
cases=$((cases + 1))

[[ "$cases" -eq 7 ]] || fail "unexpected fixture case count: $cases"
echo "final-acceptance-gate-test: PASS cases=$cases"

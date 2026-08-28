#!/usr/bin/env bash
set -euo pipefail

# Finite, dependency-free fixtures for release_candidate_gate.sh.  They use
# only a temporary directory and never invoke a test, build, or network tool.

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
gate="$script_dir/release_candidate_gate.sh"
root_dir="$(cd -- "$script_dir/.." && pwd -P)"

fail() {
    echo "release-candidate-gate-test: FAIL: $*" >&2
    exit 1
}

work="$(mktemp -d "${TMPDIR:-/tmp}/codex-info-release-candidate-test.XXXXXX")" ||
    fail "could not create temporary fixture directory"
cleanup() {
    rm -rf -- "$work"
}
trap cleanup EXIT HUP INT TERM

current_sha="$(git -C "$root_dir" rev-parse HEAD)"
current_tree="$(git -C "$root_dir" rev-parse 'HEAD^{tree}')"
source_sha=1111111111111111111111111111111111111111

make_baseline() {
    local directory="$1"
    mkdir -p -- "$directory"
    printf 'fixture Setup payload\n' > "$directory/CodexInfo.WindowsClient.Setup.exe"
    cat > "$directory/acceptance.txt" <<EOF
schema: codex-info-quality-v1
source-sha: $source_sha
tree-sha: $current_tree
acceptance: PASS
EOF
    (
        cd -- "$directory"
        find . -type f ! -name SHA256SUMS -print0 | LC_ALL=C sort -z |
            xargs -0 sha256sum > SHA256SUMS
    ) || fail "could not create baseline hash manifest"
}

baseline="$work/baseline"
make_baseline "$baseline"

run_gate() {
    (
        cd -- "$root_dir"
        "$gate" "$1" "$2"
    ) > "$work/gate.out" 2> "$work/gate.err"
}

expect_pass() {
    local label="$1" directory="$2"
    if ! run_gate "$directory" "$current_sha"; then
        cat "$work/gate.err" >&2
        fail "baseline case was rejected: $label"
    fi
}

expect_reject() {
    local label="$1" directory="$2" expected_sha="${3:-$current_sha}"
    if run_gate "$directory" "$expected_sha"; then
        fail "invalid case was accepted: $label"
    fi
}

copy_case() {
    local name="$1"
    cp -a -- "$baseline" "$work/$name"
    printf '%s\n' "$work/$name"
}

cases=0

# Deliberately use a valid source SHA different from the merge SHA.  Source
# evidence is required to be well formed, but this boundary only binds the
# current tree and the expected merge commit.
expect_pass "baseline" "$baseline"
cases=$((cases + 1))

wrong_sha="$(printf '%040d' 0)"
expect_reject "wrong merge SHA" "$baseline" "$wrong_sha"
cases=$((cases + 1))

case_dir="$(copy_case wrong-tree)"
sed -i "s/^tree-sha: .*/tree-sha: $wrong_sha/" "$case_dir/acceptance.txt"
expect_reject "wrong tree SHA" "$case_dir"
cases=$((cases + 1))

case_dir="$(copy_case wrong-source)"
sed -i 's/^source-sha: .*/source-sha: not-a-sha/' "$case_dir/acceptance.txt"
expect_reject "malformed source SHA" "$case_dir"
cases=$((cases + 1))

case_dir="$(copy_case missing-provenance)"
rm -- "$case_dir/acceptance.txt"
expect_reject "missing provenance" "$case_dir"
cases=$((cases + 1))

case_dir="$(copy_case missing-pass)"
sed -i 's/^acceptance: PASS$/acceptance: FAIL/' "$case_dir/acceptance.txt"
expect_reject "missing PASS marker" "$case_dir"
cases=$((cases + 1))

case_dir="$(copy_case missing-setup)"
rm -- "$case_dir/CodexInfo.WindowsClient.Setup.exe"
expect_reject "missing Setup" "$case_dir"
cases=$((cases + 1))

case_dir="$(copy_case missing-manifest)"
rm -- "$case_dir/SHA256SUMS"
expect_reject "missing SHA256SUMS" "$case_dir"
cases=$((cases + 1))

case_dir="$(copy_case tampered)"
printf 'tampered payload\n' >> "$case_dir/CodexInfo.WindowsClient.Setup.exe"
expect_reject "tampered Setup" "$case_dir"
cases=$((cases + 1))

case_dir="$(copy_case unlisted)"
printf 'unlisted payload\n' > "$case_dir/unlisted.bin"
expect_reject "unlisted file" "$case_dir"
cases=$((cases + 1))

[[ "$cases" -gt 0 ]] || fail "fixture case count is zero"
echo "release-candidate-gate-test: PASS cases=$cases"

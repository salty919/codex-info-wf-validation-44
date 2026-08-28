#!/usr/bin/env bash
set -euo pipefail

# Evidence-only release-candidate boundary.  The candidate was produced by the
# quality workflow; this script only checks its immutable provenance and hashes.
# It intentionally does not run a test, build, network request, or packaging
# command.

hold() {
    echo "release-candidate-gate: HOLD: $*" >&2
    exit 2
}

[[ "$#" -eq 2 ]] || hold "usage: $0 CANDIDATE EXPECTED_MERGE_SHA"

candidate="$1"
expected_merge_sha="$2"
[[ -d "$candidate" ]] || hold "candidate directory is missing: $candidate"
[[ "$expected_merge_sha" =~ ^[0-9a-f]{40}$ ]] ||
    hold "expected merge SHA must be a 40-character hexadecimal commit SHA"

candidate="$(cd -- "$candidate" 2>/dev/null && pwd -P)" ||
    hold "candidate directory cannot be opened"

current_sha="$(git rev-parse HEAD 2>/dev/null || true)"
current_tree="$(git rev-parse 'HEAD^{tree}' 2>/dev/null || true)"
[[ "$current_sha" == "$expected_merge_sha" ]] ||
    hold "checked out HEAD does not match expected merge SHA: current=$current_sha expected=$expected_merge_sha"
[[ "$current_sha" =~ ^[0-9a-f]{40}$ && "$current_tree" =~ ^[0-9a-f]{40}$ ]] ||
    hold "checked out revision/tree is not a valid Git object"

provenance="$candidate/acceptance.txt"
[[ -f "$provenance" ]] || hold "acceptance provenance is missing: $provenance"

count_exact() {
    local needle="$1" file="$2"
    grep -Fxc -- "$needle" "$file" 2>/dev/null || true
}

[[ "$(count_exact 'schema: codex-info-quality-v1' "$provenance")" == 1 ]] ||
    hold "acceptance schema is missing or ambiguous"
[[ "$(count_exact 'acceptance: PASS' "$provenance")" == 1 ]] ||
    hold "acceptance PASS marker is missing or ambiguous"

[[ "$(grep -Ec '^source-sha: ' "$provenance" || true)" == 1 ]] ||
    hold "acceptance source SHA is missing or ambiguous"
source_sha="$(sed -n 's/^source-sha: //p' "$provenance")"
[[ "$source_sha" =~ ^[0-9a-f]{40}$ ]] ||
    hold "acceptance source SHA must be a 40-character hexadecimal SHA"

[[ "$(grep -Ec '^tree-sha: ' "$provenance" || true)" == 1 ]] ||
    hold "acceptance tree SHA is missing or ambiguous"
tree_sha="$(sed -n 's/^tree-sha: //p' "$provenance")"
[[ "$tree_sha" == "$current_tree" ]] ||
    hold "accepted tree SHA does not match the checked out tree: accepted=$tree_sha current=$current_tree"

setup="$candidate/CodexInfo.WindowsClient.Setup.exe"
manifest="$candidate/SHA256SUMS"
[[ -s "$setup" ]] || hold "accepted Setup is missing or empty"
[[ -s "$manifest" ]] || hold "accepted SHA256SUMS is missing or empty"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/codex-info-release-candidate.XXXXXX")" ||
    hold "could not create temporary verification directory"
cleanup() {
    rm -rf -- "$scratch"
}
trap cleanup EXIT HUP INT TERM

actual_paths="$scratch/actual"
listed_paths="$scratch/listed"
(
    cd -- "$candidate"
    find . -type f ! -name SHA256SUMS -print | LC_ALL=C sort
) > "$actual_paths" || hold "could not enumerate candidate files"

# Parse the normal sha256sum output format.  A malformed line is rejected
# instead of being silently ignored by sha256sum -c, so the manifest is an
# exact set of paths rather than merely a count of hashes.
awk '
    {
        digest = substr($0, 1, 64)
        if (length($0) < 67 || length(digest) != 64 ||
            digest !~ /^[0-9a-f]+$/ || substr($0, 65, 2) != "  ") {
            invalid = 1
            next
        }
        print substr($0, 67)
    }
    END { if (invalid) exit 1 }
' "$manifest" > "$listed_paths" || hold "SHA256SUMS contains a malformed entry"

LC_ALL=C sort -o "$listed_paths" "$listed_paths"
[[ "$(wc -l < "$actual_paths")" == "$(wc -l < "$listed_paths")" ]] ||
    hold "SHA256SUMS does not cover exactly all candidate files"
[[ -z "$(LC_ALL=C uniq -d "$listed_paths")" ]] ||
    hold "SHA256SUMS contains a duplicate file entry"
cmp -s "$actual_paths" "$listed_paths" ||
    hold "SHA256SUMS file set is not an exact cover of the candidate"

(
    cd -- "$candidate"
    sha256sum -c SHA256SUMS >/dev/null
) || hold "release-candidate hash verification failed"

echo "release-candidate-gate: PASS source-sha=$source_sha tree-sha=$current_tree"

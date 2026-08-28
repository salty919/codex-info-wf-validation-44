#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail() {
    echo "data-protection-gate: FAIL: $*" >&2
    exit 1
}

for required in \
    docs/PRODUCT_REQUIREMENTS.md \
    docs/DATA_PROTECTION_POLICY.md \
    scripts/db_protection_e2e.sh; do
    [[ -f "$required" ]] || fail "missing $required"
done

if rg -n 'rm -rf|DROP TABLE[[:space:]]+usage_history|remove_file\([^)]*usage_history|remove_dir_all\([^)]*history' src; then
    fail "destructive history operation detected"
fi

output="$(bash scripts/db_protection_e2e.sh 2>&1)" || {
    printf '%s\n' "$output" >&2
    fail "DB protection SQLite fixture failed"
}
printf '%s\n' "$output"
rg -q --fixed-strings 'db-protection-e2e: PASS' <<<"$output" ||
    fail "DB protection fixture PASS marker missing"

echo "data-protection-gate: PASS"

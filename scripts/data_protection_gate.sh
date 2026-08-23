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

tests=(
    oversized_tool_records_do_not_hide_following_usage_samples
    recoverable_rollout_parser_keeps_running_state_around_large_tool_output
    concurrent_collectors_merge_one_minute_without_duplicate_rows
    backup_generations_are_sqlite_consistent_and_bounded
    verified_migration_switches_only_after_candidate_validation
    invalid_migration_candidate_leaves_source_untouched
    stale_pid_lock_is_reclaimed_and_live_lock_is_singleton
    opening_an_old_schema_is_rejected_without_migration
    corrupt_database_error_preserves_the_original_file
)

for test_name in "${tests[@]}"; do
    output="$(cargo test --locked "$test_name" -- --nocapture 2>&1)" || {
        printf '%s\n' "$output" >&2
        fail "test failed: $test_name"
    }
    rg -q 'running [1-9][0-9]* tests?' <<<"$output" || fail "test did not run: $test_name"
    rg -q 'test result: ok' <<<"$output" || fail "test not successful: $test_name"
done

output="$(bash scripts/db_protection_e2e.sh 2>&1)" || {
    printf '%s\n' "$output" >&2
    fail "DB protection e2e failed"
}
rg -q 'db-protection-e2e: PASS' <<<"$output" || fail "DB protection PASS marker missing"

echo "data-protection-gate: PASS"

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
    echo "data-protection-gate: $*" >&2
    exit 1
}

require_text() {
    local file="$1"
    local pattern="$2"
    rg -q --fixed-strings -- "$pattern" "$file" || fail "missing required contract: $file: $pattern"
}

require_text docs/DATA_PROTECTION_POLICY.md "## 2. 絶対不変条件"
require_text docs/DATA_PROTECTION_POLICY.md "schema mismatchは拒否する"
require_text docs/DATA_PROTECTION_POLICY.md "backup失敗時はpruneを実行しない"
require_text docs/REQUIREMENTS_LEDGER.md "DP-001"
require_text docs/REQUIREMENTS_LEDGER.md "DP-010"
require_text docs/REQUIREMENTS_LEDGER.md "独立サブエージェント"
require_text docs/REQUIREMENTS_LEDGER.md "evidence/DATA_PROTECTION_RUNTIME.md"
require_text docs/REQUIREMENTS_AUDIT_2026-08-22.md "AUD-007"
require_text docs/COMPLETION_PROTOCOL.md "独立サブエージェント"
require_text docs/B2B_RELEASE_ACCEPTANCE.md "出荷停止条件"
require_text docs/CUSTOMER_OPERATIONS_RUNBOOK.md "DB保護"
require_text docs/TRACEABILITY_MATRIX.md "要求トレーサビリティ"
require_text docs/RELEASE_MANIFEST_2026-08-22.md "RELEASE READY"
if rg -q --fixed-strings -- "NOT RELEASE READY" docs/RELEASE_MANIFEST_2026-08-22.md; then
    fail "release manifest still contains a blocking NOT RELEASE READY decision"
fi
require_text scripts/completion_guard.sh "completion-guard: PASS"
require_text scripts/windows_acceptance_e2e.sh "windows-acceptance-e2e: PASS"
require_text scripts/db_protection_e2e.sh "restore-failure-source-preserved: PASS"
require_text scripts/install_systemd_recorder.sh "systemctl --user enable --now codex-info-recorder.service"
require_text packaging/codex-info-recorder.service "Restart=on-failure"
require_text README.md "docs/DATA_PROTECTION_POLICY.md"
require_text README.md "docs/REQUIREMENTS_LEDGER.md"
require_text README.md "docs/REQUIREMENTS_AUDIT_2026-08-22.md"
require_text .github/workflows/rust.yml "cargo build --release --locked"
require_text .github/workflows/rust.yml "bash scripts/data_protection_gate.sh"
require_text .github/workflows/windows-client.yml "bash scripts/data_protection_gate.sh"

require_text src/usage_store.rs "PRIMARY KEY (reset_at, timestamp)"
require_text src/usage_store.rs "busy_timeout"
require_text src/usage_store.rs "backup_generations"
require_text src/usage_store.rs "migrate_verified"
require_text src/usage_store.rs "database schema mismatch"
require_text src/main.rs "recovery_requested"
require_text src/main.rs "latest_period_hint"
require_text src/main.rs "--record-daemon"
require_text src/daemon.rs "DAEMON_LOCK_FILE_NAME"
require_text src/daemon.rs "persist_reset_hint"
require_text src/thread_contract.rs "parse_rollout_reader_recoverable"

run_required_test() {
    local name="$1"
    local output
    output="$(cargo test --locked "$name" -- --nocapture 2>&1)" || {
        printf '%s\n' "$output" >&2
        fail "required regression test failed: $name"
    }
    printf '%s\n' "$output" | rg -q 'running [1-9][0-9]* tests?' || {
        printf '%s\n' "$output" >&2
        fail "required regression test did not run: $name"
    }
    printf '%s\n' "$output" | rg -q 'test result: ok' || {
        printf '%s\n' "$output" >&2
        fail "required regression test was not successful: $name"
    }
}

if rg -n 'rm -rf|DROP TABLE[[:space:]]+usage_history|remove_file\([^)]*usage_history|remove_dir_all\([^)]*history' src docs; then
    fail "destructive history operation detected"
fi

if rg -n '^\| DP-[0-9]+ .*\| (open|implemented|partial|missing|unverified|inconclusive) \|' docs/REQUIREMENTS_LEDGER.md; then
    fail "unverified data-protection ledger row"
fi
if [[ "$(rg -c '^\| DP-[0-9]+ .*\| verified \|' docs/REQUIREMENTS_LEDGER.md || true)" != "10" ]]; then
    fail "expected exactly ten verified data-protection ledger rows"
fi
if rg -n '^- \[ \]' docs/REQUIREMENTS_LEDGER.md; then
    fail "completion checklist still has unchecked items"
fi

run_required_test oversized_tool_records_do_not_hide_following_usage_samples
run_required_test recoverable_rollout_parser_keeps_running_state_around_large_tool_output
run_required_test persisted_period_backfill_is_admitted_before_auth_without_publishing_usage
run_required_test recovery_backfill_is_one_shot_until_authenticated_quota_returns
run_required_test concurrent_collectors_merge_one_minute_without_duplicate_rows
run_required_test backup_generations_are_sqlite_consistent_and_bounded
run_required_test verified_migration_switches_only_after_candidate_validation
run_required_test invalid_migration_candidate_leaves_source_untouched
run_required_test interval_is_bounded_even_for_invalid_environment_values
run_required_test reset_hint_round_trip_is_atomic_and_private
run_required_test stale_pid_lock_is_reclaimed_and_live_lock_is_singleton
run_required_test daemon_cycle_persists_changed_jsonl_into_history_store
run_required_test opening_an_old_schema_is_rejected_without_migration
run_required_test corrupt_database_error_preserves_the_original_file

db_protection_output="$(bash scripts/db_protection_e2e.sh 2>&1)" || {
    printf '%s\n' "$db_protection_output" >&2
    fail "DB backup/migration/restore runtime evidence failed"
}

completion_output="$(bash scripts/completion_guard.sh 2>&1)" || {
    printf '%s\n' "$completion_output" >&2
    fail "canonical requirements completion guard failed"
}
printf '%s\n' "$completion_output" | rg -q 'completion-guard: PASS' || {
    printf '%s\n' "$completion_output" >&2
    fail "canonical requirements completion PASS marker missing"
}

windows_acceptance_output="$(bash scripts/windows_acceptance_e2e.sh 2>&1)" || {
    printf '%s\n' "$windows_acceptance_output" >&2
    fail "Windows acceptance evidence failed"
}
printf '%s\n' "$windows_acceptance_output" | rg -q 'windows-acceptance-e2e: PASS' || {
    printf '%s\n' "$windows_acceptance_output" >&2
    fail "Windows acceptance PASS marker missing"
}
printf '%s\n' "$db_protection_output" | rg -q 'restore-failure-source-preserved: PASS' || {
    printf '%s\n' "$db_protection_output" >&2
    fail "restore failure preservation evidence missing"
}
printf '%s\n' "$db_protection_output" | rg -q 'db-protection-e2e: PASS' || {
    printf '%s\n' "$db_protection_output" >&2
    fail "DB protection e2e PASS marker missing"
}
printf 'db-protection-runtime-gate: PASS (backup/migration/restore failure boundaries)\n'

echo "data-protection-gate: policy, ledger, non-destructive source constraints: PASS"

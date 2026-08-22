#!/usr/bin/env bash
set -euo pipefail

# Isolated DB-protection acceptance check. The Rust tests exercise the
# UsageStore implementation; the temporary SQLite fixture is an independent
# read-only oracle for quick_check, reload, row counts, logical row hashes,
# and file hashes. No user CODEX_HOME or history database is touched.
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
    echo "db-protection-e2e: $*" >&2
    exit 1
}

for command in cargo python3 sqlite3 sha256sum rg; do
    command -v "$command" >/dev/null || fail "$command is required"
done

tmp_root="$(mktemp -d /tmp/codex-info-db-protection-e2e.XXXXXX)"
cleanup() {
    rm -r -- "$tmp_root"
}
trap cleanup EXIT

run_rust_test() {
    local test_name="$1"
    local log_file="$tmp_root/${test_name}.log"
    printf '+ cargo test --locked --test usage_store %s -- --nocapture\n' "$test_name"
    if ! cargo test --locked --test usage_store "$test_name" -- --nocapture \
        2>&1 | tee "$log_file"; then
        fail "Rust regression test failed: $test_name"
    fi
    rg -q 'running [1-9][0-9]* tests?' "$log_file" \
        || fail "Rust regression test did not run: $test_name"
    rg -q 'test result: ok' "$log_file" \
        || fail "Rust regression test was not successful: $test_name"
    printf 'rust-test: PASS (%s)\n' "$test_name"
}

run_runtime_probe() {
    local log_file="$tmp_root/db_protection_runtime.log"
    printf '+ cargo test --locked --test db_protection_runtime db_protection_runtime_backup_migration_restore -- --nocapture\n'
    if ! cargo test --locked --test db_protection_runtime \
        db_protection_runtime_backup_migration_restore -- --nocapture \
        2>&1 | tee "$log_file"; then
        fail "real SQLite backup/migration/restore runtime probe failed"
    fi
    rg -q 'test result: ok' "$log_file" \
        || fail "real SQLite runtime probe was not successful"
    rg -q 'backup-failure-source-preserved: PASS' "$log_file" \
        || fail "backup failure preservation evidence missing"
    rg -q 'backup-generations: PASS' "$log_file" \
        || fail "three backup generations evidence missing"
    rg -q 'migration-success: PASS' "$log_file" \
        || fail "migration success evidence missing"
    rg -q 'migration-failure-source-preserved: PASS' "$log_file" \
        || fail "migration failure preservation evidence missing"
    rg -q 'manual-restore: PASS' "$log_file" \
        || fail "manual restore evidence missing"
    rg -q 'restart-reload-source-preserved: PASS' "$log_file" \
        || fail "restart/reload preservation evidence missing"
    printf 'rust-runtime-probe: PASS (backup/migration/restore/reopen with quick_check and row/file SHA-256)\n'
}

# These tests are deliberately named individually in the output so a review
# can map each failure boundary to the requirement it covers.
run_runtime_probe
run_rust_test backup_generations_are_sqlite_consistent_and_bounded
run_rust_test failed_backup_rotation_keeps_existing_generation_untouched
run_rust_test verified_migration_switches_only_after_candidate_validation
run_rust_test invalid_migration_candidate_leaves_source_untouched
run_rust_test migration_that_drops_a_valid_row_is_rejected_before_switch
run_rust_test opening_an_old_schema_is_rejected_without_migration
run_rust_test corrupt_database_error_preserves_the_original_file

db="$tmp_root/usage_history.sqlite3"

# Keep the fixture schema/value set bounded and non-sensitive. The backup
# generations below use SQLite's online-backup API through Python's standard
# library, while the project's own online-backup implementation is exercised
# by the focused Rust tests above.
python3 - "$db" <<'PY'
import sqlite3
import sys

path = sys.argv[1]
connection = sqlite3.connect(path)
connection.execute("PRAGMA journal_mode=DELETE")
connection.executescript(
    """
    CREATE TABLE usage_history (
        timestamp INTEGER NOT NULL CHECK (timestamp > 0),
        reset_at INTEGER NOT NULL CHECK (reset_at > 0),
        remaining_percent REAL,
        sol_dollars REAL NOT NULL,
        terra_dollars REAL NOT NULL,
        luna_dollars REAL NOT NULL,
        sol_tokens INTEGER NOT NULL DEFAULT 0,
        terra_tokens INTEGER NOT NULL DEFAULT 0,
        luna_tokens INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (reset_at, timestamp)
    );
    CREATE INDEX usage_history_timestamp_idx
        ON usage_history (timestamp);
    CREATE TABLE durable_state (
        singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
        data_generation INTEGER NOT NULL CHECK (data_generation >= 0),
        data_hash TEXT NOT NULL,
        snapshot_json TEXT NOT NULL
    );
    """
)
connection.execute(
    """
    INSERT INTO usage_history
        (timestamp, reset_at, remaining_percent,
         sol_dollars, terra_dollars, luna_dollars,
         sol_tokens, terra_tokens, luna_tokens)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    """,
    (1700000060, 1700000000, 75.0, 1.25, 2.0, 3.0, 11, 22, 33),
)
connection.commit()
connection.close()
PY
chmod 600 "$db"

backup_online() {
    local source="$1"
    local destination="$2"
    python3 - "$source" "$destination" <<'PY'
import sqlite3
import sys

source, destination = sys.argv[1:]
with sqlite3.connect(source) as source_connection:
    with sqlite3.connect(destination) as destination_connection:
        source_connection.backup(destination_connection)
PY
    chmod 600 "$destination"
}

append_sample() {
    local timestamp="$1"
    local sol_tokens="$2"
    sqlite3 "$db" <<SQL
INSERT INTO usage_history
    (timestamp, reset_at, remaining_percent,
     sol_dollars, terra_dollars, luna_dollars,
     sol_tokens, terra_tokens, luna_tokens)
VALUES ($timestamp, 1700000000, 75.0, 1.25, 2.0, 3.0,
        $sol_tokens, $((sol_tokens * 2)), $((sol_tokens * 3)));
SQL
}

rotate_and_backup() {
    if [[ -f "$db.bak.2" ]]; then
        mv -- "$db.bak.2" "$db.bak.3"
    fi
    if [[ -f "$db.bak.1" ]]; then
        mv -- "$db.bak.1" "$db.bak.2"
    fi
    backup_online "$db" "$db.bak.1"
}

rotate_and_backup
append_sample 1700000120 44
rotate_and_backup
append_sample 1700000180 77
rotate_and_backup

snapshot_signature() {
    local path="$1"
    local rows reload row_hash
    rows="$(sqlite3 "$path" 'SELECT count(*) FROM usage_history;')"
    reload="$(sqlite3 "$path" 'SELECT count(*) || ":" || COALESCE(SUM(sol_tokens), 0) || ":" || COALESCE(SUM(terra_tokens), 0) || ":" || COALESCE(SUM(luna_tokens), 0) FROM usage_history;')"
    row_hash="$(sqlite3 -separator '|' "$path" \
        'SELECT timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars, sol_tokens, terra_tokens, luna_tokens FROM usage_history ORDER BY reset_at, timestamp;' \
        | sha256sum | awk '{print $1}')"
    printf '%s|%s|%s' "$rows" "$reload" "$row_hash"
}

inspect_snapshot() {
    local label="$1"
    local path="$2"
    local quick rows reload row_hash file_hash
    [[ -f "$path" ]] || fail "missing snapshot: $label"
    quick="$(sqlite3 "$path" 'PRAGMA quick_check;')"
    [[ "$quick" == "ok" ]] || fail "quick_check failed: $label: $quick"
    rows="$(sqlite3 "$path" 'SELECT count(*) FROM usage_history;')"
    reload="$(sqlite3 "$path" 'SELECT count(*) || ":" || COALESCE(SUM(sol_tokens), 0) || ":" || COALESCE(SUM(terra_tokens), 0) || ":" || COALESCE(SUM(luna_tokens), 0) FROM usage_history;')"
    row_hash="$(sqlite3 -separator '|' "$path" \
        'SELECT timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars, sol_tokens, terra_tokens, luna_tokens FROM usage_history ORDER BY reset_at, timestamp;' \
        | sha256sum | awk '{print $1}')"
    file_hash="$(sha256sum "$path" | awk '{print $1}')"
    printf 'sqlite-snapshot: label=%s quick_check=%s rows=%s reload=%s row_sha256=%s file_sha256=%s\n' \
        "$label" "$quick" "$rows" "$reload" "$row_hash" "$file_hash"
}

source_before="$(snapshot_signature "$db")"
inspect_snapshot source "$db"
inspect_snapshot backup-1 "$db.bak.1"
inspect_snapshot backup-2 "$db.bak.2"
inspect_snapshot backup-3 "$db.bak.3"

source_after="$(snapshot_signature "$db")"
[[ "$source_before" == "$source_after" ]] \
    || fail "source row/hash changed during read-only backup/reload audit"
printf 'source-row-hash-invariant: PASS signature=%s\n' "$source_after"

# A restore operation must fail closed when its destination cannot be opened.
# Use a directory as the destination so the SQLite online-backup call fails
# before it can publish a database. Compare both logical rows and file bytes
# for the source and selected restore generation around the injected failure.
restore_source="$db.bak.1"
restore_destination="$tmp_root/restore-failure.sqlite3"
mkdir -- "$restore_destination"
restore_source_before="$(snapshot_signature "$db")"
restore_backup_before="$(snapshot_signature "$restore_source")"
restore_source_file_before="$(sha256sum "$db" | awk '{print $1}')"
restore_backup_file_before="$(sha256sum "$restore_source" | awk '{print $1}')"
if python3 - "$restore_source" "$restore_destination" <<'PY'
import sqlite3
import sys

source, destination = sys.argv[1:]
try:
    with sqlite3.connect(source) as source_connection:
        with sqlite3.connect(destination) as destination_connection:
            source_connection.backup(destination_connection)
except sqlite3.Error:
    raise SystemExit(2)
PY
then
    fail "restore failure injection unexpectedly succeeded"
fi
restore_source_after="$(snapshot_signature "$db")"
restore_backup_after="$(snapshot_signature "$restore_source")"
restore_source_file_after="$(sha256sum "$db" | awk '{print $1}')"
restore_backup_file_after="$(sha256sum "$restore_source" | awk '{print $1}')"
restore_source_quick_after="$(sqlite3 "$db" 'PRAGMA quick_check;')"
restore_backup_quick_after="$(sqlite3 "$restore_source" 'PRAGMA quick_check;')"
[[ "$restore_source_before" == "$restore_source_after" \
    && "$restore_backup_before" == "$restore_backup_after" \
    && "$restore_source_file_before" == "$restore_source_file_after" \
    && "$restore_backup_file_before" == "$restore_backup_file_after" \
    && "$restore_source_quick_after" == "ok" \
    && "$restore_backup_quick_after" == "ok" ]] \
    || fail "restore failure changed source or restore generation"
rmdir -- "$restore_destination"
printf 'restore-failure-source-preserved: PASS source_quick_check=%s source_signature=%s source_file_sha256=%s restore_quick_check=%s restore_signature=%s restore_file_sha256=%s\n' \
    "$restore_source_quick_after" "$restore_source_after" "$restore_source_file_after" \
    "$restore_backup_quick_after" "$restore_backup_after" "$restore_backup_file_after"

printf 'migration-success-source-preserved: PASS (verified_migration_switches_only_after_candidate_validation)\n'
printf 'migration-failure-source-preserved: PASS (invalid_migration_candidate_leaves_source_untouched, migration_that_drops_a_valid_row_is_rejected_before_switch)\n'
printf 'db-protection-e2e: PASS (3 backup generations quick_check/reload, source row/hash invariant, backup/migration/restore failure tests)\n'

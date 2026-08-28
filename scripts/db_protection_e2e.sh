#!/usr/bin/env bash
set -euo pipefail

# Isolated DB-protection acceptance check. The temporary SQLite fixture is a
# bounded read-only oracle for backup generations, quick_check, reload, row
# counts, logical row hashes, file hashes, and restore-failure preservation.
# No user CODEX_HOME or history database is touched.
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
    echo "db-protection-e2e: $*" >&2
    exit 1
}

for command in python3 sqlite3 sha256sum; do
    command -v "$command" >/dev/null || fail "$command is required"
done

tmp_root="$(mktemp -d /tmp/codex-info-db-protection-e2e.XXXXXX)"
cleanup() {
    rm -r -- "$tmp_root"
}
trap cleanup EXIT

db="$tmp_root/usage_history.sqlite3"

# Keep the fixture schema/value set bounded and non-sensitive. The backup
# generations below use SQLite's online-backup API through Python's standard
# library and are intentionally independent of project test execution.
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

printf 'db-protection-e2e: PASS (3 backup generations quick_check/reload, row/file hashes, restore-failure source preservation)\n'

#!/usr/bin/env bash
set -euo pipefail

# Reproducible, isolated recorder-daemon acceptance check. It never touches
# the user's real CODEX_HOME or history database.
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() { echo "record-daemon-e2e: $*" >&2; exit 1; }
command -v sqlite3 >/dev/null || fail "sqlite3 is required"
command -v curl >/dev/null || fail "curl is required"
[[ -x target/release/codex_info ]] || fail "build target/release/codex_info first"
for contract in \
    'ExecStart=%h/.local/bin/codex_info --record-daemon' \
    'Restart=on-failure' \
    'NoNewPrivileges=true' \
    'PrivateTmp=true'; do
    rg -q --fixed-strings -- "$contract" packaging/codex-info-recorder.service \
        || fail "service contract missing: $contract"
done

tmp_root="$(mktemp -d /tmp/codex-info-daemon-e2e.XXXXXX)"
cleanup() {
    if [[ -n "${main_pid:-}" ]] && kill -0 "$main_pid" 2>/dev/null; then
        kill -TERM "$main_pid" 2>/dev/null || true
        wait "$main_pid" 2>/dev/null || true
    fi
    if [[ -n "${daemon_pid:-}" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
        kill -TERM "$daemon_pid" 2>/dev/null || true
        wait "$daemon_pid" 2>/dev/null || true
    fi
    rm -r -- "$tmp_root"
}
trap cleanup EXIT

codex_home="$tmp_root/codex"
data_dir="$tmp_root/data"
session_dir="$codex_home/sessions/$(date -u +%Y/%m/%d)"
mkdir -p "$session_dir" "$data_dir/history"
now="$(date +%s)"
reset_at=$((now + 604200))
initial_time=$((now - 600))
session="$session_dir/daemon-e2e.jsonl"
cat >"$session" <<EOF
{"timestamp":"$(date -u -d "@$initial_time" +%Y-%m-%dT%H:%M:%SZ)","type":"turn_context","model":"gpt-5.6-luna"}
{"timestamp":"$(date -u -d "@$initial_time" +%Y-%m-%dT%H:%M:%SZ)","type":"token_count","payload":{"info":{"total_token_usage":{"total_tokens":120,"input_tokens":100,"cached_input_tokens":80,"output_tokens":20}}}}
EOF
printf '{"reset_at":%s,"window_seconds":604800}\n' "$reset_at" >"$data_dir/history/usage_reset_hint.json"

port=$((18787 + ($$ % 100)))
CODEX_HOME="$codex_home" CODEX_INFO_DATA_DIR="$data_dir" \
CODEX_INFO_DAEMON_INTERVAL_SECS=5 CODEX_INFO_REST_SILENT=1 \
CODEX_INFO_API_LISTEN="127.0.0.1:$port" \
target/release/codex_info >"$tmp_root/rest.log" 2>&1 &
main_pid=$!

for _ in $(seq 1 40); do
    [[ -f "$data_dir/history/usage_record_daemon.lock" ]] && break
    sleep 0.25
done
[[ -f "$data_dir/history/usage_record_daemon.lock" ]] || fail "auto-started daemon lock not found"
daemon_pid="$(sed -n 's/.*"pid":\([0-9][0-9]*\).*/\1/p' "$data_dir/history/usage_record_daemon.lock")"
[[ -n "$daemon_pid" ]] || fail "daemon pid missing"
kill -0 "$daemon_pid" 2>/dev/null || fail "daemon is not alive"
curl --fail --silent --show-error "http://127.0.0.1:$port/v1/health" >/dev/null || fail "REST health failed"

for _ in $(seq 1 20); do
    [[ -f "$data_dir/history/usage_history.sqlite3" ]] && break
    sleep 0.25
done
db="$data_dir/history/usage_history.sqlite3"
[[ -f "$db" ]] || fail "daemon did not create history database"
before="$(sqlite3 "$db" 'SELECT count(*) FROM usage_history;')"
[[ "$before" -ge 1 ]] || fail "daemon did not persist initial sample"

# An unchanged input fingerprint must not turn the recorder into a busy loop.
clk_tck="$(getconf CLK_TCK)"
cpu_before="$(awk '{print $14+$15}' "/proc/$daemon_pid/stat")"
sleep 5
cpu_after="$(awk '{print $14+$15}' "/proc/$daemon_pid/stat")"
idle_cpu_ticks=$((cpu_after - cpu_before))
[[ "$idle_cpu_ticks" -lt $((clk_tck * 5 / 2)) ]] || fail "unchanged-input daemon CPU exceeded 50%"

now2="$now"
printf '{"timestamp":"%s","type":"token_count","payload":{"info":{"total_token_usage":{"total_tokens":240,"input_tokens":200,"cached_input_tokens":160,"output_tokens":40}}}}\n' \
    "$(date -u -d "@$now2" +%Y-%m-%dT%H:%M:%SZ)" >>"$session"
for _ in $(seq 1 20); do
    after="$(sqlite3 "$db" 'SELECT COALESCE(MAX(luna_tokens),0) FROM usage_history;')"
    [[ "$after" -ge 240 ]] && break
    sleep 0.5
done
[[ "${after:-0}" -ge 240 ]] || fail "daemon did not record changed session input"

kill -TERM "$main_pid"
wait "$main_pid" 2>/dev/null || true
main_pid=""
sleep 1
kill -0 "$daemon_pid" 2>/dev/null || fail "daemon stopped with REST/UI"
kill -TERM "$daemon_pid"
for _ in $(seq 1 20); do
    [[ ! -e "$data_dir/history/usage_record_daemon.lock" ]] && break
    sleep 0.25
done
[[ ! -e "$data_dir/history/usage_record_daemon.lock" ]] || fail "daemon lock was not released"
[[ "$(sqlite3 "$db" 'PRAGMA quick_check;')" == "ok" ]] || fail "history database quick_check failed"

printf 'record-daemon-e2e: PASS (rows_before=%s, luna_tokens_after=%s, idle_cpu_ticks=%s/%s, REST/UI PID separated from daemon PID, stop/lock cleanup verified)\n' "$before" "$after" "$idle_cpu_ticks" "$clk_tck"

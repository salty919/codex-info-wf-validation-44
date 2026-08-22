#!/usr/bin/env bash
set -euo pipefail

# Fail-closed release watchdog.  It never converts a missing, interrupted, or
# inconclusive report into PASS.  The default loop exits only after every
# release gate reports PASS; SIGTERM/SIGINT are recorded as an interrupted
# watchdog and therefore do not constitute completion.

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

interval="${CODEX_INFO_WATCHDOG_INTERVAL_SEC:-60}"
log_file="${CODEX_INFO_WATCHDOG_LOG:-/tmp/codex_info_requirements_watchdog.log}"
once=0

usage() {
    printf 'usage: %s [--once] [--interval SEC] [--log FILE]\n' "$0"
}

while (($# > 0)); do
    case "$1" in
        --once) once=1; shift ;;
        --interval)
            (($# >= 2)) || { usage >&2; exit 2; }
            interval="$2"; shift 2 ;;
        --log)
            (($# >= 2)) || { usage >&2; exit 2; }
            log_file="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

[[ "$interval" =~ ^[1-9][0-9]*$ ]] || {
    echo "watchdog: interval must be a positive integer" >&2
    exit 2
}

mkdir -p "$(dirname -- "$log_file")"
touch "$log_file"

interrupted=0
on_interrupt() {
    interrupted=1
    printf '%s watchdog: INTERRUPTED (completion remains blocked)\n' "$(date --iso-8601=seconds)" | tee -a "$log_file" >&2
    exit 130
}
trap on_interrupt INT TERM

run_gate() {
    local name="$1"
    shift
    local output status
    set +e
    output="$($@ 2>&1)"
    status=$?
    set -e
    printf '%s gate=%s status=%s\n%s\n' "$(date --iso-8601=seconds)" "$name" "$status" "$output" | tee -a "$log_file"
    return "$status"
}

while :; do
    all_pass=1
    run_gate completion bash scripts/completion_guard.sh || all_pass=0
    run_gate intake bash scripts/requirements_intake_guard.sh || all_pass=0
    if [[ -x scripts/windows_requirements_extraction_check.sh ]]; then
        run_gate extraction-structure bash scripts/windows_requirements_extraction_check.sh || all_pass=0
    fi
    run_gate regression bash scripts/regression_guard.sh || all_pass=0
    if [[ -x scripts/data_protection_gate.sh ]]; then
        run_gate data-protection bash scripts/data_protection_gate.sh || all_pass=0
    fi

    if ((all_pass)); then
        printf '%s watchdog: PASS (all completion gates passed)\n' "$(date --iso-8601=seconds)" | tee -a "$log_file"
        exit 0
    fi

    printf '%s watchdog: HOLD (unresolved requirements or evidence; retrying in %ss)\n' "$(date --iso-8601=seconds)" "$interval" | tee -a "$log_file" >&2
    ((once)) && exit 1
    sleep "$interval"
done

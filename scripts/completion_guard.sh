#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ ! -f docs/INDEPENDENT_AUDIT_LATEST.md ]] || ! rg -q '^status:[[:space:]]*PASS[[:space:]]*$' docs/INDEPENDENT_AUDIT_LATEST.md; then
    echo 'completion-guard: FAIL (GOV-SUBAGENT-01 requires latest independent subagent PASS)' >&2
    exit 1
fi

audit_sha="$(awk '/^artifact_sha256:[[:space:]]*/ {print $2; exit}' docs/INDEPENDENT_AUDIT_LATEST.md)"
if [[ -z "$audit_sha" ]] || ! rg -q --fixed-strings -- "$audit_sha" docs/AGENT_REQUIREMENTS_TRACKER.md; then
    echo 'completion-guard: FAIL (subagent tracker SHA does not match independent audit artifact)' >&2
    exit 1
fi
if rg -q '\|[[:space:]]*(INCONCLUSIVE|HOLD)[[:space:]]*\|' docs/AGENT_REQUIREMENTS_TRACKER.md; then
    echo 'completion-guard: FAIL (subagent tracker contains incomplete work)' >&2
    exit 1
fi

failures=()

check_table() {
    local file="$1"
    while IFS=$'\t' read -r id status; do
        [[ -n "$id" ]] || continue
        if [[ "$status" != "verified" && "$status" != "implemented" ]]; then
            failures+=("$file: $id=$status")
        fi
    done < <(
        awk -F'|' '
            /^\|[[:space:]]*[A-Z][A-Z0-9-]+[[:space:]]*\|/ {
                id=$2; status=$(NF-1)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", status)
                if (id == "ID") next
                print id "\t" status
            }
        ' "$file"
    )
}

check_table docs/REQUIREMENTS_LEDGER.md
check_table docs/WINDOWS_CLIENT_REQUIREMENTS.md

while IFS=$'\t' read -r id status; do
    [[ -n "$id" ]] || continue
    if [[ "$status" != "verified" && "$status" != "implemented" ]]; then
        failures+=("docs/REQUIREMENTS_AUDIT_2026-08-22.md: $id=$status")
    fi
done < <(
    awk -F'|' '
        /^\|[[:space:]]*AUD-[0-9]+[[:space:]]*\|/ {
            id=$2; status=$4
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", status)
            print id "\t" status
        }
    ' docs/REQUIREMENTS_AUDIT_2026-08-22.md
)

if ((${#failures[@]} > 0)); then
    printf '%s\n' "${failures[@]}"
    echo 'completion-guard: FAIL (unverified requirements remain)' >&2
    exit 1
fi

echo 'completion-guard: PASS (all canonical requirement rows are verified/implemented)'

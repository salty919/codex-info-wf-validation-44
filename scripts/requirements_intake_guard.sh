#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail() { echo "requirements-intake-guard: FAIL: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing $1"; }
require_text() { rg -q --fixed-strings -- "$2" "$1" || fail "missing contract $1: $2"; }

require_file docs/REQUIREMENTS_LEDGER.md
require_file docs/WINDOWS_CLIENT_REQUIREMENTS.md
require_file docs/TEST_GAP_REGISTER_2026-08-22.md
require_file docs/AGENT_REQUIREMENTS_TRACKER.md
require_file docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md
require_file docs/WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md
require_file docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md
require_file docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md
require_file docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md
require_file docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md
require_file docs/evidence/WINDOWS_REQUIREMENTS_EXTRACTION_AUDIT_2026-08-22.md
require_file docs/evidence/WINDOWS_REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_2026-08-22_V1.md
require_file docs/evidence/GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md
require_file docs/evidence/UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md
require_file docs/evidence/ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md
require_text docs/REQUIREMENTS_INTAKE_POLICY.md '実装を始める前に'
require_text docs/REQUIREMENTS_INTAKE_POLICY.md '破損・再起動・更新中・複数プロセス'
require_text docs/AGENT_REQUIREMENTS_TRACKER.md '担当'
require_text docs/AGENT_REQUIREMENTS_TRACKER.md '成果物SHA'

row_count="$(rg -c '^\| WIN-[A-M]-[0-9]{3} \|' docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md || true)"
[[ "$row_count" == "226" ]] || fail "row-contract ledger must contain exactly 226 rows (got $row_count)"
header="$(rg '^\| requirement_id \|' docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md || true)"
[[ "$header" == *'independent_reviewer'* ]] || fail 'row-contract ledger must expose all 11 columns'

# This guard is intentionally blocking while extraction is open. It must never emit a
# misleading PASS merely because legacy ledgers happen to be closed.
if rg -q --fixed-strings '状態: `EXTRACTION_INCOMPLETE`' \
    docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md \
    docs/WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md \
    docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md; then
    fail 'requirements extraction is incomplete; implementation/evaluation/release remain blocked'
fi

if rg -n '\|[[:space:]]*(open|partial|unverified|inconclusive)[[:space:]]*\|' docs/REQUIREMENTS_LEDGER.md docs/WINDOWS_CLIENT_REQUIREMENTS.md; then
    fail 'requirements contain unresolved status; implementation/release gate must remain blocked'
fi
if rg -n '\|[[:space:]]*未確認[[:space:]]*\|' docs/TEST_GAP_REGISTER_2026-08-22.md; then
    fail 'test-gap register contains unresolved gaps; implementation/release gate must remain blocked'
fi

echo 'requirements-intake-guard: PASS (all extraction rows and test gaps are closed)'

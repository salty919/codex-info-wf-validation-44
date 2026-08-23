#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
  echo "windows-requirements-extraction-check: FAIL: $*" >&2
  exit 1
}

required_files=(
  docs/WINDOWS_REQUIREMENTS_CANONICAL_INDEX_2026-08-23.md
  docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md
  docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md
  docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md
  docs/WINDOWS_LEGACY_REQUIREMENT_CROSSWALK_2026-08-22.md
  docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md
  docs/WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md
  docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md
  docs/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md
  docs/REST_API_V1.md
  docs/WINDOWS_CLIENT.md
  docs/WINDOWS_UX_SPEC.md
  docs/UX_DECISION_NON_SCROLL_2026-08-22.md
  docs/UX_DECISION_GRAPH_LABELS_2026-08-22.md
  docs/UX_DECISION_SSH_CONNECTION_2026-08-23.md
  docs/UX_DECISION_ACCESSIBILITY_SCALE_2026-08-23.md
  docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md
  docs/UX_DECISION_RELEASE_SUPPLY_CHAIN_2026-08-23.md
  docs/UX_DECISION_ERROR_RECOVERY_2026-08-23.md
  docs/UX_DECISION_FULL_STATE_MATRIX_2026-08-23.md
  docs/UX_DECISION_HELP_FOCUS_2026-08-23.md
  docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md
  docs/UX_DECISION_KEYBOARD_FOCUS_2026-08-23.md
  docs/THREAD_PIPELINE_FIXTURE_CONTRACT_2026-08-23.md
  DESIGN.md
  docs/atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md
  docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md
  docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md
  docs/atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md
  docs/atomic-contracts/WINDOWS_LEGACY_GAP_PROJECTIONS_2026-08-23.md
  docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md
  docs/REQUIREMENTS_LEDGER.md
  docs/REQUIREMENTS_AUDIT_2026-08-22.md
  docs/WINDOWS_CLIENT_REQUIREMENTS.md
  docs/TEST_GAP_REGISTER_2026-08-22.md
)
for path in "${required_files[@]}"; do
  [[ -f "$path" ]] || fail "missing required file: $path"
done

python3 - <<'PY'
import re
from collections import Counter
from pathlib import Path

BASELINE = Path("docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md")
CANONICAL = Path("docs/WINDOWS_REQUIREMENTS_CANONICAL_INDEX_2026-08-23.md")
CONTRACTS = [
    Path("docs/atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md"),
    Path("docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md"),
    Path("docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md"),
]
CROSSWALK = Path("docs/WINDOWS_LEGACY_REQUIREMENT_CROSSWALK_2026-08-22.md")
TRACE_MATRIX = Path("docs/WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md")
TRACE_DESIGN = Path("docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md")
ROW_CONTRACTS = Path("docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md")
LIFECYCLE = Path("docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md")
CONFLICTS = Path("docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md")
B2B_PROJECTIONS = Path("docs/atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md")
LEGACY_GAP_PROJECTIONS = Path("docs/atomic-contracts/WINDOWS_LEGACY_GAP_PROJECTIONS_2026-08-23.md")
FREEZE_CONTRACT = Path("docs/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md")
TRACKER = Path("docs/AGENT_REQUIREMENTS_TRACKER.md")
LEDGER = Path("docs/REQUIREMENTS_LEDGER.md")
INTAKE = Path("docs/REQUIREMENTS_INTAKE_POLICY.md")
COMPLETION = Path("docs/COMPLETION_PROTOCOL.md")
AGENTS = Path("AGENTS.md")
RID = re.compile(r"WIN-[A-M]-\d{3}")
GOV_IDS = ["GOV-THREAD-END", "GOV-NO-INPUT-END", "GOV-ESCALATION-100X"]
DECISION_INVENTORY = [
    ("docs/UX_DECISION_ACCESSIBILITY_SCALE_2026-08-23.md", "UX-20260823-ACCESSIBILITY-SCALE-001"),
    ("docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md", "UX-20260823-B2B-CUSTOMER-DELIVERY-001"),
    ("docs/UX_DECISION_ERROR_RECOVERY_2026-08-23.md", "UX-20260823-ERROR-001"),
    ("docs/UX_DECISION_FULL_STATE_MATRIX_2026-08-23.md", "UX-20260823-FULL-STATE-001"),
    ("docs/UX_DECISION_GRAPH_LABELS_2026-08-22.md", "UX-20260822-GRAPH-001"),
    ("docs/UX_DECISION_HELP_FOCUS_2026-08-23.md", "UX-20260823-HELP-FOCUS-001"),
    ("docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md", "UX-20260823-INSTALLER-001"),
    ("docs/UX_DECISION_KEYBOARD_FOCUS_2026-08-23.md", "UX-20260823-KEYBOARD-001"),
    ("docs/UX_DECISION_NON_SCROLL_2026-08-22.md", "UX-20260822-UX-002"),
    ("docs/UX_DECISION_RELEASE_SUPPLY_CHAIN_2026-08-23.md", "UX-20260823-RELEASE-SUPPLY-CHAIN-001"),
    ("docs/UX_DECISION_SSH_CONNECTION_2026-08-23.md", "UX-20260822-SSH-001"),
]

EXPECTED_FREEZE_PATHS = [
    "AGENTS.md",
    "DESIGN.md",
    "LICENSE",
    "LICENSE.ja.md",
    "LICENSES/Apache-2.0.txt",
    "LICENSES/BSD-3-Clause-ANGLE.txt",
    "LICENSES/MIT.txt",
    "LICENSES/OFL-1.1.txt",
    "LICENSES/OPENAI-CODEX-NOTICE.txt",
    "README.en.md",
    "README.md",
    "SECURITY.md",
    "THIRD_PARTY_NOTICES.md",
    "VERIFICATION_PLAN.md",
    "assets/NOTICE.txt",
    "docs/AGENT_REQUIREMENTS_TRACKER.md",
    "docs/B2B_RELEASE_ACCEPTANCE.md",
    "docs/COMPLETION_PROTOCOL.md",
    "docs/CUSTOMER_OPERATIONS_RUNBOOK.md",
    "docs/DATA_PROTECTION_POLICY.md",
    "docs/LIVE_STATE_DECISION_MATRIX.md",
    "docs/LOCALIZATION.md",
    "docs/REGRESSION_PREVENTION_POLICY.md",
    "docs/REQUIREMENTS_AUDIT_2026-08-22.md",
    "docs/REQUIREMENTS_INTAKE_POLICY.md",
    "docs/REQUIREMENTS_LEDGER.md",
    "docs/REST_API_V1.md",
    "docs/TEST_GAP_REGISTER_2026-08-22.md",
    "docs/THREAD_PIPELINE_FIXTURE_CONTRACT_2026-08-23.md",
    "docs/UX_DECISION_ACCESSIBILITY_SCALE_2026-08-23.md",
    "docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md",
    "docs/UX_DECISION_ERROR_RECOVERY_2026-08-23.md",
    "docs/UX_DECISION_FULL_STATE_MATRIX_2026-08-23.md",
    "docs/UX_DECISION_GRAPH_LABELS_2026-08-22.md",
    "docs/UX_DECISION_HELP_FOCUS_2026-08-23.md",
    "docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md",
    "docs/UX_DECISION_KEYBOARD_FOCUS_2026-08-23.md",
    "docs/UX_DECISION_NON_SCROLL_2026-08-22.md",
    "docs/UX_DECISION_RELEASE_SUPPLY_CHAIN_2026-08-23.md",
    "docs/UX_DECISION_SSH_CONNECTION_2026-08-23.md",
    "docs/WINDOWS_CLIENT.md",
    "docs/WINDOWS_CLIENT_REQUIREMENTS.md",
    "docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md",
    "docs/WINDOWS_LEGACY_REQUIREMENT_CROSSWALK_2026-08-22.md",
    "docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md",
    "docs/WINDOWS_REQUIREMENTS_CANONICAL_INDEX_2026-08-23.md",
    "docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md",
    "docs/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md",
    "docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md",
    "docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md",
    "docs/WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md",
    "docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md",
    "docs/WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md",
    "docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md",
    "docs/WINDOWS_UX_SPEC.md",
    "docs/atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md",
    "docs/atomic-contracts/WINDOWS_LEGACY_GAP_PROJECTIONS_2026-08-23.md",
    "docs/atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md",
    "docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md",
    "docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md",
    "docs/evidence/ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md",
    "docs/evidence/GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md",
    "docs/evidence/UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md",
    "scripts/windows_requirements_extraction_check.sh",
    "windows-client/THIRD_PARTY_NOTICES.md",
]


def fail(message: str) -> None:
    raise SystemExit(f"windows-requirements-extraction-check: FAIL: {message}")


def table_rows(
    path: Path,
    pattern: re.Pattern[str],
    stop_before: tuple[str, ...] = (),
) -> list[list[str]]:
    result = []
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if any(line.startswith(marker) for marker in stop_before):
            break
        if not pattern.match(line):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        cells.append(str(line_no))
        result.append(cells)
    return result


freeze_entries = [
    (int(match.group(1)), match.group(2))
    for match in re.finditer(r"(?m)^(\d+)\. `([^`]+)`$", FREEZE_CONTRACT.read_text(encoding="utf-8"))
]
if len(EXPECTED_FREEZE_PATHS) != 65:
    fail(f"expected freeze path set must contain 65 paths, got {len(EXPECTED_FREEZE_PATHS)}")
freeze_contract_text = FREEZE_CONTRACT.read_text(encoding="utf-8")
for marker in (
    "上記65 pathを`requirements content entries`と呼ぶ",
    "報告自身を除く65 pathから計算した",
    "§3の65 pathだけをcanonical列化した",
    "§3の65 content entryのbytes/sha256",
    "65 content entriesが変わっていないことを確認",
    "別担当が65 content SHA",
):
    if marker not in freeze_contract_text:
        fail(f"freeze contract lacks exact 63-path marker: {marker}")
if any(obsolete in freeze_contract_text for obsolete in ("上記60 path", "§3の60 path", "上記63 path", "§3の63 path", "上記64 path", "§3の64 path")):
    fail("freeze contract retains an obsolete content-path count")
freeze_numbers = [number for number, _ in freeze_entries]
freeze_paths = [path for _, path in freeze_entries]
current_authority_paths = [path for path, _decision_id in DECISION_INVENTORY]
missing_current_authority = [
    path for path in current_authority_paths
    if path not in freeze_paths
]
if missing_current_authority:
    fail(f"RECALC_REQUIRED: current Decision authority is absent from mutable freeze set: {missing_current_authority}")
missing_current_authority_files = [
    path for path in current_authority_paths if not Path(path).is_file()
]
if missing_current_authority_files:
    fail(f"current Decision authority paths are missing: {missing_current_authority_files}")
if freeze_numbers != list(range(1, len(EXPECTED_FREEZE_PATHS) + 1)):
    fail(f"freeze contract numbering differs: {freeze_numbers}")
if freeze_paths != EXPECTED_FREEZE_PATHS or freeze_paths != sorted(freeze_paths):
    fail(
        "freeze contract exact path set/order differs: "
        f"missing={sorted(set(EXPECTED_FREEZE_PATHS)-set(freeze_paths))} "
        f"extra={sorted(set(freeze_paths)-set(EXPECTED_FREEZE_PATHS))}"
    )
missing_freeze_files = [path for path in freeze_paths if not Path(path).is_file()]
if missing_freeze_files:
    fail(f"freeze contract paths missing: {missing_freeze_files}")


baseline_rows = table_rows(BASELINE, re.compile(r"^\| WIN-[A-M]-\d{3} \|"))
baseline_ids = [row[0] for row in baseline_rows]
if len(baseline_ids) != 226 or len(set(baseline_ids)) != 226:
    fail(f"baseline IDs must be 226 unique rows; rows={len(baseline_ids)} unique={len(set(baseline_ids))}")
baseline_titles = {row[0]: row[1].strip() for row in baseline_rows}
empty_titles = sorted(row_id for row_id, title in baseline_titles.items() if not title)
duplicate_titles = sorted(
    title for title, count in Counter(baseline_titles.values()).items() if count != 1
)
if empty_titles or duplicate_titles or len(baseline_titles) != 226:
    fail(
        "baseline target titles must be non-empty and unique: "
        f"empty={empty_titles} duplicate={duplicate_titles}"
    )

expected_counts = {
    "A": 20, "B": 24, "C": 20, "D": 12, "E": 16, "F": 12, "G": 16,
    "H": 12, "I": 16, "J": 16, "K": 16, "L": 16, "M": 30,
}
actual_counts = Counter(item.split("-")[1] for item in baseline_ids)
if dict(sorted(actual_counts.items())) != expected_counts:
    fail(f"baseline category counts differ: {dict(sorted(actual_counts.items()))}")

contract_rows: list[list[str]] = []
expected_contract_sets = {
    CONTRACTS[0]: {rid for rid in baseline_ids if rid.split("-")[1] in {"A", "B", "C", "D"}},
    CONTRACTS[1]: {rid for rid in baseline_ids if rid.split("-")[1] in {"E", "F", "G", "H", "I"}},
    CONTRACTS[2]: {rid for rid in baseline_ids if rid.split("-")[1] in {"J", "K", "L", "M"}},
}
for path in CONTRACTS:
    # J–M has a later RC-167〜169 supplementary oracle table whose first
    # column intentionally repeats source WIN IDs.  It is not part of the
    # 10-column concrete-contract table and must not be counted as duplicate
    # product rows.
    stop_before = ("## RC-167〜169 exact extension",) if path == CONTRACTS[2] else ()
    rows = table_rows(path, re.compile(r"^\| WIN-[A-M]-\d{3} \|"), stop_before)
    file_ids = {row[0] for row in rows}
    if file_ids != expected_contract_sets[path] or len(rows) != len(expected_contract_sets[path]):
        fail(
            f"{path}: wrong range/set; rows={len(rows)} "
            f"missing={sorted(expected_contract_sets[path] - file_ids)} "
            f"extra={sorted(file_ids - expected_contract_sets[path])}"
        )
    for row in rows:
        line_no = row.pop()
        if len(row) != 10:
            fail(f"{path}:{line_no}: concrete contract must have 10 cells, got {len(row)}")
        if any(not cell for cell in row):
            fail(f"{path}:{line_no}: empty concrete-contract cell")
        if not row[4].startswith("fixture_only:"):
            fail(f"{path}:{line_no}: concrete input must begin with fixture_only:")
        row.append(f"{path}:{line_no}")
        contract_rows.append(row)

contract_ids = [row[0] for row in contract_rows]
duplicates = sorted(item for item, count in Counter(contract_ids).items() if count != 1)
if duplicates:
    fail(f"duplicate concrete IDs: {duplicates}")
missing = sorted(set(baseline_ids) - set(contract_ids))
extra = sorted(set(contract_ids) - set(baseline_ids))
if len(contract_ids) != 226 or missing or extra:
    fail(f"concrete/baseline ID mismatch: rows={len(contract_ids)} missing={missing} extra={extra}")

row_contract_rows = table_rows(ROW_CONTRACTS, re.compile(r"^\| WIN-[A-M]-\d{3} \|"))
row_contract_ids = [row[0] for row in row_contract_rows]
if len(row_contract_ids) != 226 or len(set(row_contract_ids)) != 226:
    fail(
        "row-contract ledger must contain 226 unique IDs: "
        f"rows={len(row_contract_ids)} unique={len(set(row_contract_ids))}"
    )
if set(row_contract_ids) != set(contract_ids):
    fail(
        "row-contract/concrete ID mismatch: "
        f"missing={sorted(set(contract_ids)-set(row_contract_ids))} "
        f"extra={sorted(set(row_contract_ids)-set(contract_ids))}"
    )
for row in row_contract_rows:
    line_no = row.pop()
    if len(row) != 11:
        fail(f"{ROW_CONTRACTS}:{line_no}: row contract must have 11 cells, got {len(row)}")
    if any(not cell for cell in row):
        fail(f"{ROW_CONTRACTS}:{line_no}: empty row-contract cell")
    row.append(line_no)

lifecycle_rows = table_rows(LIFECYCLE, re.compile(r"^\| WIN-[A-M]-\d{3} \|"))
if len(lifecycle_rows) != 58 or len({row[0] for row in lifecycle_rows}) != 58:
    fail(f"{LIFECYCLE}: lifecycle rows must be 58 unique IDs, got {len(lifecycle_rows)}")
for row in lifecycle_rows:
    line_no = row.pop()
    if len(row) != 7:
        fail(f"{LIFECYCLE}:{line_no}: lifecycle row must have 7 cells, got {len(row)}")
    if any(not cell for cell in row):
        fail(f"{LIFECYCLE}:{line_no}: empty lifecycle cell")
    row.append(line_no)

current_atomic_markers = (
    "docs/atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md",
    "docs/atomic-contracts/WINDOWS_LEGACY_GAP_PROJECTIONS_2026-08-23.md",
    "docs/atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md",
    "docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md",
    "docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md",
)
row_contract_text = ROW_CONTRACTS.read_text(encoding="utf-8")
for marker in current_atomic_markers:
    if marker not in row_contract_text:
        fail(f"{ROW_CONTRACTS}: missing current concrete-contract marker {marker}")
for path in (ROW_CONTRACTS, BASELINE):
    forbidden_lines = [
        f"{path}:{line_no}"
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1)
        if "WINDOWS_DOMAIN_ATOMIC_ASSERTIONS" in line
        and ("必須参照" in line or "一意に定義する" in line)
    ]
    if forbidden_lines:
        fail(
            "history-only legacy document is still declared required or uniquely defining in "
            f"{path}: {forbidden_lines}"
        )

known = set(baseline_ids)
DEPENDENCY_KEYS = ("hard_prerequisite", "related_validation_join")
LAYER_BY_CATEGORY = {
    "I": 0, "J": 0,
    "B": 1, "D": 1,
    "A": 2, "C": 2, "E": 2, "F": 2, "G": 2, "H": 2, "K": 2, "M": 2,
    "L": 3,
}


def parse_typed_dependencies(value: str, origin: str) -> tuple[list[str], list[str]]:
    parts = value.split("; ")
    if len(parts) != 2:
        fail(
            f"{origin}: dependency cell must have exactly "
            "hard_prerequisite and related_validation_join separated by '; '")
    fields: dict[str, str] = {}
    for part in parts:
        if "=" not in part:
            fail(f"{origin}: dependency part is not key=value: {part}")
        key_raw, raw_refs_raw = part.split("=", 1)
        key, raw_refs = key_raw.strip(), raw_refs_raw.strip()
        if key_raw != key or raw_refs_raw != raw_refs:
            fail(f"{origin}: dependency key/value has non-canonical whitespace: {part}")
        if key not in DEPENDENCY_KEYS:
            fail(f"{origin}: unknown dependency type {key}")
        if key in fields:
            fail(f"{origin}: duplicate dependency type {key}")
        fields[key] = raw_refs
    if tuple(fields) != DEPENDENCY_KEYS or set(fields) != set(DEPENDENCY_KEYS):
        fail(f"{origin}: dependency types must be ordered exactly as {DEPENDENCY_KEYS}")

    def parse_refs(raw_refs: str) -> list[str]:
        if raw_refs == "—":
            return []
        if not raw_refs:
            fail(f"{origin}: empty dependency list must use em dash")
        refs = raw_refs.split(",")
        if any(item != item.strip() for item in refs):
            fail(f"{origin}: dependency IDs must not contain comma-adjacent whitespace: {raw_refs}")
        if any(not RID.fullmatch(item) for item in refs):
            fail(f"{origin}: dependency list contains non-ID syntax: {raw_refs}")
        return refs

    return parse_refs(fields[DEPENDENCY_KEYS[0]]), parse_refs(fields[DEPENDENCY_KEYS[1]])


dependencies_by_id: dict[str, list[str]] = {}
related_by_id: dict[str, list[str]] = {}
hard_edge_count = 0
related_edge_count = 0
dependency_reason_target_ids: set[str] = set()
for row in contract_rows:
    row_id, dependency, origin = row[0], row[8], row[-1]
    hard_refs, related_refs = parse_typed_dependencies(dependency, origin)
    refs = hard_refs + related_refs
    if row_id in refs:
        fail(f"{origin}: self dependency {row_id}")
    unknown = sorted(set(refs) - known)
    if unknown:
        fail(f"{origin}: unknown dependencies {unknown}")
    duplicate_refs = sorted(item for item, count in Counter(refs).items() if count != 1)
    if duplicate_refs:
        fail(f"{origin}: duplicate dependency references across types {duplicate_refs}")
    type_duplicates = sorted(set(hard_refs) & set(related_refs))
    if type_duplicates:
        fail(f"{origin}: dependency IDs appear in both types {type_duplicates}")
    for target_id in refs:
        if not baseline_titles[target_id]:
            fail(f"{origin}: target baseline title is empty for {target_id}")
        dependency_reason_target_ids.add(target_id)
    dependencies_by_id[row_id] = hard_refs
    related_by_id[row_id] = related_refs
    hard_edge_count += len(hard_refs)
    related_edge_count += len(related_refs)

if (hard_edge_count, related_edge_count, hard_edge_count + related_edge_count) != (412, 165, 577):
    fail(
        "typed dependency counts differ: "
        f"hard={hard_edge_count} related={related_edge_count} "
        f"total={hard_edge_count + related_edge_count}"
    )


dependency_state: dict[str, int] = {}
dependency_stack: list[str] = []


def visit_dependency(row_id: str) -> None:
    state = dependency_state.get(row_id, 0)
    if state == 2:
        return
    if state == 1:
        cycle_start = dependency_stack.index(row_id)
        cycle = dependency_stack[cycle_start:] + [row_id]
        fail(f"concrete dependency graph contains a cycle: {' -> '.join(cycle)}")
    dependency_state[row_id] = 1
    dependency_stack.append(row_id)
    for dependency_id in dependencies_by_id[row_id]:
        visit_dependency(dependency_id)
    dependency_stack.pop()
    dependency_state[row_id] = 2


for dependency_row_id in sorted(dependencies_by_id):
    visit_dependency(dependency_row_id)

backward_hard_edges = []
for consumer_id, producer_ids in dependencies_by_id.items():
    consumer_layer = LAYER_BY_CATEGORY[consumer_id.split("-")[1]]
    for producer_id in producer_ids:
        producer_layer = LAYER_BY_CATEGORY[producer_id.split("-")[1]]
        if producer_layer > consumer_layer:
            backward_hard_edges.append((consumer_id, producer_id))
if backward_hard_edges:
    fail(f"hard dependency graph contains backward layer edges: {sorted(backward_hard_edges)}")
if len(dependency_reason_target_ids) > len(baseline_titles):
    fail("dependency reason target join exceeds baseline title set")

rows_by_id = {row[0]: row for row in contract_rows}

# The RC-167〜169 companion table is an oracle extension, not a second set of
# product rows.  Its join key must be explicit so raw Markdown audits cannot
# mistake the ten source references for duplicate concrete contracts.
jm_contract_text = CONTRACTS[2].read_text(encoding="utf-8")
surface_navigation_addendum = "### WIN-M-006 / WIN-M-007 surface-navigation addendum (RC-083)"
if surface_navigation_addendum not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-083 Graph/Threads surface-navigation addendum is missing")
for navigation_fragment in (
    "WIN-M-012",
    "action.Back",
    "title.Close",
    "keyboard/UIA",
    "Graphのperiod/metric/toggle",
    "Threadsのpage/selection",
    "共有行だけの検査でGraph/Threadsを合格扱いにしない",
):
    if navigation_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-083 addendum lacks exact fragment: {navigation_fragment}")
for source_id in ("| source_id=WIN-M-006 | WIN-M-012 |", "| source_id=WIN-M-007 | WIN-M-012 |"):
    if source_id not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-083 per-surface projection missing: {source_id}")
unexpected_exit_marker = "### WIN-J-011 unexpected-exit projection (RC-066)"
if unexpected_exit_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-066 unexpected-exit projection is missing")
for unexpected_exit_fragment in (
    "source_id=WIN-J-011:unexpected-exit",
    "StopRequested→Stopped",
    "unexpected exit is detected within 2 seconds",
    "exactly one automatic restart is allowed after a 5-second backoff",
    "restart failure or second unexpected exit latches `Failed`",
    "only explicit start or a new systemd activation starts a new epoch",
    "prior complete snapshot, DB, persisted hint, source cursor, and confirmed gap ledger",
    "never fabricate the stopped interval or a successful gap",
):
    if unexpected_exit_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-066 projection lacks exact fragment: {unexpected_exit_fragment}")
reset_hint_marker = "### WIN-J-010 reset-hint/fingerprint/backfill projection (RC-065)"
if reset_hint_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-065 reset-hint/backfill projection is missing")
for reset_hint_fragment in (
    "source_id=WIN-J-010:reset-hint-backfill",
    "canonical sessions root regular non-symlink JSONL",
    "device/inode, size, `mtime_ns`, last complete LF offset, and last complete row SHA-256",
    "unchanged fingerprint yields scan/write/retry=0",
    "append resumes at cursor",
    "rotate/truncate discards cursor and performs one recheck",
    "backfill latch is consumed once only",
    "current AuthEpoch, same current source identity, `reset_at > now`",
    "at most 1024 rows and 1 MiB",
    "expired/tombstoned or AuthEpoch/nonce mismatch rejects hint scan/backfill write",
    "existing rows are not rewritten and missing intervals are not fabricated",
):
    if reset_hint_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-065 projection lacks exact fragment: {reset_hint_fragment}")
singleton_contention_marker = "### WIN-J-012 / WIN-J-013 singleton-vs-contention projection (RC-068)"
if singleton_contention_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-068 singleton/contention projection is missing")
for singleton_contention_fragment in (
    "J-012の複数writer試験は、J-013の同一profile recorder二重起動とは別のfixtureである",
    "J-012行のactor表記「two daemon servers」はこの競合fixtureの許可済みDB writer processを指し、recorder lease ownerを二つ作る意味ではない",
    "source_id=WIN-J-012:contention",
    "same DB、unique key `(partition_id,reset_at,timestamp)`",
    "busy timeout and each attempt deadline `2.000s`",
    "contention is not a second recorder owner and uses another permitted writer process",
    "same-callback polling retry=0",
    "B may reach at most attempt2 only in a later scheduled cycle or explicit operation",
    "source_id=WIN-J-013:singleton",
    "singleton scope is canonical DB path + profile",
    "daemon-A acquires `/fixtures/profile/daemon.lock`, then daemon-B launches",
    "live owner count<=1",
    "different profile or different DB is independent",
    "disabling/bypassing the lease to make a product success is FAIL",
    "age alone never steals",
    "all writers of one DB still use J-012 unique/upsert/busy contract",
):
    if singleton_contention_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-068 projection lacks exact fragment: {singleton_contention_fragment}")
stale_lock_marker = "### WIN-J-013 stale-lock identity projection (RC-069)"
if stale_lock_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-069 stale-lock projection is missing")
for stale_lock_fragment in (
    "RC-069はJ-013のsingleton leaseを再定義せず",
    "source_id=WIN-J-013:stale-lock",
    "lease is UTF-8 JSON `recorder-lease-v1`, maximum 4 KiB",
    "`pid`, `process_start`, `owner_nonce`, `canonical_db_path`, `device_or_volume_serial`, and `file_index_or_inode`",
    "stale recovery requires PID absence or process-start mismatch",
    "reopen the same path and compare the reopened file identity with the identity recorded at acquisition",
    "24 hours is diagnostic elapsed time only, never a deletion condition",
    "age-only deletion, another owner's deletion, path mismatch, or identity mismatch has removal count=0",
    "lease bypass is not a product success path",
):
    if stale_lock_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-069 projection lacks exact fragment: {stale_lock_fragment}")
maintenance_owner_marker = "### WIN-J-006 / WIN-J-014 maintenance-owner projection (RC-070)"
if maintenance_owner_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-070 maintenance-owner projection is missing")
for maintenance_owner_fragment in (
    "RC-070は起動時maintenanceのownerとprune admissionを",
    "source_id=WIN-J-006:maintenance-owner",
    "canonical DB profileの唯一の`MaintenanceOwner`",
    "prune前にwriter admissionを閉じる",
    "`online backup candidate → flush → quick_check/schema/row count/deterministic fingerprint/reset-period境界検証 → verified rotation → prune transaction`",
    "同一activationの新verified BackupGenerationは最大1件",
    "backup失敗、検証失敗、writer競合ではprune=0",
    "source_id=WIN-J-014:backup-rotation",
    "backup files mode=`0600`, DB dir mode=`0700`",
    "one maintenance activation may publish at most one new generation",
    "generation sequenceは`0→1→2→3`",
    "crash/recovery完了前のwriter/prune/publish=0",
):
    if maintenance_owner_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-070 projection lacks exact fragment: {maintenance_owner_fragment}")
generation_journal_marker = "### WIN-J-006 / WIN-J-014 backup-generation journal projection (RC-071)"
if generation_journal_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-071 backup-generation projection is missing")
for generation_journal_fragment in (
    "RC-071はRC-070のowner/admission順序を変更せず",
    "source_id=WIN-J-006:generation-namespace",
    "`.bak.1=最新verified`, `.bak.2=次に新しいverified`, `.bak.3=最古verified`",
    "one activation adds at most one verified generation",
    "accumulates real-time `0→1→2→3`",
    "identical snapshot is not duplicated",
    "source_id=WIN-J-014:rotation-journal",
    "owner-only `backup-rotation-v1` journal records old rank/path/inode/hash",
    "crash/restart uses journal and hashes to choose exactly one rollback or roll-forward",
    "journal reconciliation completes before writer/prune/publish resumes",
    "explicit restore selects only the latest complete verified generation",
    "prune/writer/publish before journal recovery=0",
):
    if generation_journal_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-071 projection lacks exact fragment: {generation_journal_fragment}")
migration_switch_marker = "### WIN-J-015 migration-switch projection (RC-073)"
if migration_switch_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: RC-073 migration-switch projection is missing")
for migration_switch_fragment in (
    "RC-073は旧schema拒否、明示candidate成功、candidate/switch/crash失敗の3経路を",
    "source_id=WIN-J-015:old-schema",
    "old schema read/write/publish=0",
    "source_id=WIN-J-015:candidate-success",
    "explicit `UsageStore::migrate_verified`",
    "writer/API/UI admission closed",
    "unique key `(partition_id,reset_at,timestamp)`",
    "result DataGeneration=DG11 once, pair publication=1",
    "source_id=WIN-J-015:candidate-failure",
    "`migration-switch-v1` owner-only 0600 UTF-8 JSON <=64 KiB",
    "verified commit前はOLD唯一current, switch/delete/publication=0",
    "current missing/double/empty is held until reconcile",
    "foreign/second operation is Busy with mutation 0",
):
    if migration_switch_fragment not in jm_contract_text:
        fail(f"{CONTRACTS[2]}: RC-073 projection lacks exact fragment: {migration_switch_fragment}")
extension_marker = "## RC-167〜169 exact extension for WIN-J-007..016"
if extension_marker not in jm_contract_text:
    fail(f"{CONTRACTS[2]}: missing RC-167〜169 extension marker")
extension_text = jm_contract_text.split(extension_marker, 1)[1]
extension_text = extension_text.split("## 構造ゲート", 1)[0]
if re.search(r"^\| WIN-J-\d{3} \|", extension_text, re.MULTILINE):
    fail(f"{CONTRACTS[2]}: RC extension repeats concrete WIN-J row IDs without source_id prefix")
extension_source_ids = re.findall(r"^\| (source_id=WIN-J-\d{3}) \|", extension_text, re.MULTILINE)
expected_extension_source_ids = [f"source_id=WIN-J-{number:03d}" for number in range(7, 17)]
if extension_source_ids != expected_extension_source_ids:
    fail(
        f"{CONTRACTS[2]}: RC extension source_id join differs: "
        f"actual={extension_source_ids} expected={expected_extension_source_ids}"
    )
if "attempt1 full rollback" in extension_text:
    fail(f"{CONTRACTS[2]}: RC extension makes WIN-J-012 rollback unconditional")
for extension_fragment in (
    "BUSY期限超過時だけattempt1=full_rollback",
    "2秒以内のcommit-within-deadlineはattempt1=commit",
    "same-cycle retry=0",
):
    if extension_fragment not in extension_text:
        fail(f"{CONTRACTS[2]}: RC extension missing conditional WIN-J-012 busy oracle {extension_fragment}")


def require_fragments(row_id: str, *fragments: str) -> None:
    searchable = " ".join(rows_by_id[row_id][1:10])
    absent = [fragment for fragment in fragments if fragment not in searchable]
    if absent:
        fail(f"{row_id}: missing semantic anchors {absent}")


def require_doc_fragments(path: Path, *fragments: str) -> None:
    searchable = path.read_text(encoding="utf-8")
    absent = [fragment for fragment in fragments if fragment not in searchable]
    if absent:
        fail(f"{path}: missing cross-row authority anchors {absent}")


# High-risk meaning anchors. These supplement, but never replace, independent clause review.
require_fragments("WIN-B-002", "plot_end=min(quota.reset_at,now)", "now<reset_at", "now==reset_at", "now>reset_at")
require_fragments("WIN-B-016", "Remaining", "LUNA", "TERRA", "SOL", "同時")
require_fragments("WIN-B-017", "Remaining→LUNA→TERRA→SOL")
require_fragments("WIN-B-019", "0..100", "独立")
require_fragments("WIN-B-020", "表示中", "最大")
require_fragments("WIN-C-020", "起動直後", "10秒", "3秒", "in-flight")
require_fragments("WIN-C-020", "busy=true,enabled=false", "localized busy label/icon", "UIA busy/disabled", "focus/cursor/old_rootを保持", "late root updateはFAIL")
require_fragments("WIN-D-003", "parent_thread_id", "context_usage_tokens", "context_window_tokens", "is_subagent")
require_fragments("WIN-E-010", "ssh.exe", "-N", "-L", "8787:127.0.0.1:8787", "listener_owner_typed_join=(WIN-J-010.daemon_lease,WIN-J-013.singleton_owner_lease,WIN-J-016.REST_publisher_bootstrap_generation_cycle_tuple)", "cycle_tuple=(ProfileScopeId,AccountScopeId,StorageEpoch,SupervisorLeaseIdentity,CollectorEpoch,CycleSeq)")
require_fragments("WIN-E-002", "profile_enum=[none,wsl,sshConfigAlias]", "health→status→auth-start→separate-auth-check→ready")
require_fragments("WIN-E-003", "settings_keys_exact=[language,setupCompleted,connectionConfigured,timeZoneId,connectionProfile,connectionSelector]", "profile_enum=[none,wsl,sshConfigAlias]", "selector_is_non_secret=true", "shell_process_count=0")
require_fragments("WIN-E-004", "raw_manual_user=one-session-only", "one_session_raw_durable_completion=false")
require_fragments("WIN-E-005", "raw_manual_host=one-session-only", "literal_Host_alias_only=true", "alias_selector_grammar=^[A-Za-z0-9][A-Za-z0-9._-]{0,254}$")
require_fragments("WIN-E-006", "client_resolves_alias=false", "ssh.exe_resolution_owner")
require_fragments("WIN-E-006", "connectionProfile=sshConfigAlias", "connectionSelector=literal_Host_alias", "selector_grammar=^[A-Za-z0-9][A-Za-z0-9._-]{0,254}$")
require_fragments("WIN-E-006", "automatic_remote_argv_exact=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,<validated alias>]", "full_argv_token_count=7")
require_fragments("WIN-E-007", "direct_executable_and_ArgumentList_only=true", "shell_process_count=0", "cmd_process_count=0", "PowerShell_process_count=0", "BatchMode=yes")
require_fragments("WIN-E-008", "automatic_remote_argv_contains=BatchMode=yes", "one_session_raw_durable_completion=false", "selector_persistence=literal_alias_only")
require_fragments("WIN-E-001", "Codex Infoへようこそ", "Welcome to Codex Info", "mixed_language_occurrences=0")
require_fragments("WIN-E-007", "safe_argv=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,codex-lab]", "automatic_remote_argv_exact=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,<validated alias>]", "argv_token_count=7")
require_fragments("WIN-E-008", "argv_user_empty=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,codex-lab]", "argv_user_present=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,alice_qa@codex-lab]", "rendered_copy_empty=\"ssh.exe -o BatchMode=yes -N -L 8787:127.0.0.1:8787 codex-lab\"", "argv_token_count=7")
require_fragments("WIN-E-010", "automatic_remote_argv_exact=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,<validated alias>]", "argv_token_count=7", "listener_owner_identity_matches_current_supervised_ssh_generation=true", "preexisting_listener_connected_ready=false", "post_exit_rebind_connected_ready=false", "pid_reuse_connected_ready=false", "foreign_listener_adoption=0")
require_fragments("WIN-E-011", "supervis", "orphan", "SSH_PROCESS_START_OR_EXIT", "error.ssh.process-start-or-exit.cause", "next_launch_auto_reconnect=true", "recorder_continues=true", "same_generation_auto_retry_infinite=0", "listener_owner_identity_matches_current_supervised_ssh_generation=true", "post_exit_rebind_connected_ready=false", "pid_reuse_connected_ready=false", "foreign_listener_adoption=0", "listener_owner_typed_join=(WIN-J-010.daemon_lease,WIN-J-013.singleton_owner_lease,WIN-J-016.REST_publisher_bootstrap_generation_cycle_tuple)")
require_fragments("WIN-E-012", "/v1/health", "application/json", "no-store", "health_is_reachability_only=true", "sequence_next=status=true", "listener_owner_identity_exact_before_after_each_cycle=true", "remote_listener_owner_check=before_after_each_health_status_details_cycle matches current supervised ssh generation", "wsl_owner_check=profile-specific bootstrap/service generation plus Windows↔WSL loopback path", "foreign/rebound/PID-reused/unverified listener accept count=0", "verification_unavailable=reject", "listener_owner_typed_join=(WIN-J-010.daemon_lease,WIN-J-013.singleton_owner_lease,WIN-J-016.REST_publisher_bootstrap_generation_cycle_tuple)")
require_fragments("WIN-E-013", "health_status_auth_ready_separate=true", "ready_only_after_separate_auth_check=true", "ready_wire_boolean_field=0", "ready_predicate=state=ready AND authenticated=true", "setup_confirmation_once=true", "setup_loop=0")
require_fragments("WIN-F-006", "inventory_categories=[GPL,third-party,font,schema,dependency,distribution]", "runtime notice is included under third-party package sources", "no invented privacy notice")
if "inventory_categories=[GPL,runtime,third-party,font,schema,dependency,distribution]" in " ".join(rows_by_id["WIN-F-006"][1:10]):
    fail("WIN-F-006: Legal UI retains obsolete seven-category runtime entry")
require_fragments("WIN-I-014", "status=500", "wire_error_code=internal_error", "failure_class=DB_SERVER_ERROR", "error.data.unavailable.cause", "error.data.unavailable.impact")
if "failure_class=API_UNREACHABLE" in " ".join(rows_by_id["WIN-I-014"][1:10]):
    fail("WIN-I-014: HTTP 500 still maps to API_UNREACHABLE")
require_fragments("WIN-E-014", "WSL_selector=installed_distribution_exact_token", "SSH_selector=literal_Host_alias", "auth_argv_exact={WSL=[wsl.exe,-d,<selector>,--,codex,login],SSH=[ssh.exe,-o,BatchMode=yes,<selector>,codex,login]}", "ArgumentList", "hidden_prompt=0")
require_fragments("WIN-E-015", "old4/corrupt/invalid route=Main_disconnected", "old4_corrupt_invalid.Settings_recovery=true", "recovery_command_count=0", "never enter Welcome/Setup loop")
require_fragments("WIN-E-016", "host", "user", "persisted_occurrence_count=0")
require_fragments("WIN-E-016", "settings_keys_exact=[language,setupCompleted,connectionConfigured,timeZoneId,connectionProfile,connectionSelector]", "profile_enum=[none,wsl,sshConfigAlias]", "selector_grammar=^[A-Za-z0-9][A-Za-z0-9._-]{0,254}$")
require_fragments("WIN-F-003", "local", "UTC", "2026/08/22 00:00 +00:00", "2026/08/22 09:00 +09:00")
require_fragments("WIN-F-004", "API_UNREACHABLE", "error.api.unreachable.impact", "locale=ja")
require_fragments("WIN-F-008", "language", "setupCompleted", "connectionConfigured", "timeZoneId", "connectionSelector", "selector_atomic=true")
require_fragments("WIN-M-015", "SETTINGS_SAVE_FAILED", "action.settings.save.retry")
require_fragments("WIN-F-007", "saved non-secret selector/profile retained only when valid", "one-session raw recovery is not durable completion", "automatic command count=0")
require_fragments("WIN-G-001", "ja", "en", "zh-Hans", "ko", "es", "fr", "de", "pt", "it", "ru")
require_fragments("WIN-G-014", "UX-20260823-KEYBOARD-001", "keydown/keyup", "mouse_event_count=0", "setup_profile_step_projection=profile×step visible+enabled Tab projection", "SetupOperationGeneration=monotonic", "stale_completion_commit=0", "RC-121 profile_action_semantics", "action.StartForward is not a catch-all")
require_fragments("WIN-G-015", "2 logical-pixel", "3:1", "high-contrast")
require_fragments("WIN-G-016", "reduced-motion")
require_fragments("WIN-H-002", "self-contained", "外部.NET", "runtime")
require_fragments("WIN-I-006", "8 KiB", "64 KiB", "32 MiB", "128", "100,000", "256", "3")
require_fragments("WIN-I-007", "history_periods", "history_samples", "history_gaps", "details_top13", "gap5", "gaps 4096", "details_contract_revision=rest-v1-details-gap-20260823", "estimated_cost_label", "monthly", "LIVE_STATE_DECISION_MATRIX", "THREAD_PIPELINE_FIXTURE §2.1/§2.2/§3", "process_identity=(pid,starttime_ticks,exe_device,exe_inode)", "publisher_admission=(ProfileScopeId,AccountScopeId,StorageEpoch,SupervisorLeaseIdentity,CollectorEpoch,CycleSeq)", "PlanType exact 15 values and canonical label/monthly mapping", "period labelはreference-only", "period id、sample partition投影内(reset_at,timestamp)", "Windows label reverse parse", "ready wire boolean=0", "native invalid", "完全accepted REST setのmissing-parent")
require_fragments("WIN-I-013", "input_tokens", "cached_input_tokens", "output_tokens")
require_fragments("WIN-I-014", "API_UNREACHABLE", "raw_body", "last-good")
require_fragments("WIN-I-016", "auth_required", "auth_clearはold account/quota/model/history/thread visible/accessibility occurrence=0", "DB hash=H0", "LIVE_STATE_DECISION_MATRIX", "publisher_admission=(ProfileScopeId,AccountScopeId,StorageEpoch,SupervisorLeaseIdentity,CollectorEpoch,CycleSeq)", "native=[duplicate,dangling,cycle,partial]", "完全accepted REST setのmissing-parent", "preexisting", "post-exit rebind", "pid reuse", "foreign", "listener ProcessIdentity、SSH supervised generationまたはWSL bootstrap/service generation、publisher admission")
require_fragments("WIN-J-006", "retention_max=3", "BackupGeneration count=1", "new_generation_max_per_activation=1")
require_fragments("WIN-J-004", "keyは(partition_id,reset_at,timestamp)", "partition_id欠落", "minuteというkey列名を作らない")
if "keyは(reset_at,timestamp)だけ" in " ".join(rows_by_id["WIN-J-004"][1:10]):
    fail("WIN-J-004: partition-aware canonical key was narrowed to reset_at/timestamp")
require_fragments("WIN-J-005", "period+timestamp upsert", "unique key=(partition_id,reset_at,timestamp)")
if "period+minute upsert" in rows_by_id["WIN-J-005"][2]:
    fail("WIN-J-005: title still names the forbidden minute-key upsert")
require_fragments("WIN-J-012", "each_attempt_busy_deadline=2.000 s", "subcase-B attempt1=BUSY_full_rollback", "subcase-A attempt1=commit_within_deadline", "same_cycle_attempt=1", "same_callback_polling_retry=0", "next normal cycle or explicit operation", "retry_count<=1")
require_fragments("WIN-J-014", "online backup")
require_fragments("WIN-J-014", "generation_sequence=[0,1,2,3]", "new_generation_max_per_activation=1", "same_activation_duplicate=0", "flush/fsync", "directory-fsync", "rotation journal", "pre-recovery prune/write/publish=0")
require_fragments("WIN-J-015", "journal schema=migration-switch-v1", "exact keys=[schema_version,operation_id,operation_generation,owner_identity,phase,current_identity,current_sha256,candidate_identity,candidate_sha256,quarantine_identity,quarantine_sha256,parent_data_generation,result_data_generation_or_null,created_at_utc,updated_at_utc]", "rollbackまたはroll-forward", "missing/double/emptyはreconcile完了まで公開0")
require_fragments("WIN-J-009", "record_policy_sha=76ce097b4412b61b95131c80ae36ddc1768a4fa119591e067f6e3de9d4519d8b", "cases=[LIVE_OVERSIZE_PROVEN_TOOL_THEN_TERMINAL", "LOCAL_PARTIAL_LATER_SNAPSHOT", "local_model_column_order=[input_tokens,cached_input_tokens,output_tokens,input_dollars,cached_input_dollars,output_dollars]")
require_fragments("WIN-K-015", "Main", "Setup", "Settings", "Graph", "Threads", "Legal")
require_fragments("WIN-K-002", "profile/selector authority=RC-061..063", "WSL installed distribution exact token", "SSH literal Host alias", "SSH_PROFILE_INVALID", "SSH_PROCESS_START_OR_EXIT", "SSH_HEALTH_UNAVAILABLE", "automatic_remote_argv_exact=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,<validated alias>]", "safe_argv_token_count=7", "shell/cmd/PowerShell=0", "BatchMode=yes", "hidden_prompt=0")
require_fragments("WIN-K-003", "app-wide supervisor owns exactly one bootstrap/tunnel child", "next launch auto reconnect", "orphan_tunnel=0", "recorder_continues=true", "same_generation_auto_retry_infinite=0", "preexisting_listener", "post_exit_rebind", "pid_reuse", "foreign_listener_adoption=0", "owner_verification_unavailable_ready=false")
require_fragments("WIN-K-004", "health_status_auth_ready_separate=true", "auth-start success alone is not ready", "Setup_confirmation_once=true", "auth_requiredでは同一status bodyにplan/quotaの安全値があっても現行account値として表示せず")
if "安全なquota/plan値だけをauth_requiredとして表示でき" in " ".join(rows_by_id["WIN-K-004"][1:10]):
    fail("WIN-K-004: auth_required still permits plan/quota display contrary to auth-clear authority")
require_fragments("WIN-K-010", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true")
require_fragments("WIN-K-011", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true")
require_fragments("WIN-K-012", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true")
require_fragments("WIN-K-012", "UI child windows=[Setup,Settings,Graph,Threads,Legal]", "Help is Main-internal route and never a child HWND")
if "Help when separate" in " ".join(rows_by_id["WIN-K-012"][1:10]):
    fail("WIN-K-012: Help is still permitted as a separate child window")
require_fragments("WIN-K-009", "LIVE_STATE_DECISION_MATRIX", "THREAD_PIPELINE_FIXTURE_CONTRACT", "§2.2/§3", "ProcessIdentity=(pid,starttime_ticks,exe_device,exe_inode)", "Codex Info祖先observer app-server全process除外", "path→owner set", "publisher_admission=(ProfileScopeId,AccountScopeId,StorageEpoch,SupervisorLeaseIdentity,CollectorEpoch,CycleSeq)", "860d7ec45d6e53357b6f94201154d5a642fee9611bdb7bb410df5f712ea5f249", "76ce097b4412b61b95131c80ae36ddc1768a4fa119591e067f6e3de9d4519d8b", "record_policy_cases=[LIVE_OVERSIZE_PROVEN_TOOL_THEN_TERMINAL", "LOCAL_PARTIAL_LATER_SNAPSHOT", "invalid_utf8", "invalid_json", "invalid_envelope", "invalid_state", "oversize_isolation_requires=bounded_streaming+valid_envelope+liveness_nonchanging_proof", "contract_hashes=[native=d9bf9d1429ef", "owner=860d7ec45d6e53357b6f94201154d5a642fee9611bdb7bb410df5f712ea5f249", "record=76ce097b4412b61b95131c80ae36ddc1768a4fa119591e067f6e3de9d4519d8b", "rest=461e0f28bdde", "presentation=8b2fe86ea389")
require_fragments("WIN-L-003", "226", "96")
require_fragments("WIN-L-015", "physical Windows host", "installer_failure_matrix=[crash_before_commit,crash_after_commit,reboot_mid_operation,journal_replay,duplicate_start,owner_lease_replay]", "operation_journal_replay_idempotent=true", "singleton_owner_lease=one", "owner_replay_match_required=true", "last_good_on_failure=true")
require_fragments("WIN-M-009", "同一viewport")
require_fragments("WIN-M-005", "keys=[language,setupCompleted,connectionConfigured,timeZoneId,connectionProfile,connectionSelector]", "connectionProfile enum=[none,wsl,sshConfigAlias]", "selector_grammar=^[A-Za-z0-9][A-Za-z0-9._-]{0,254}$", "old4_corrupt_invalid_recovery=true")
require_fragments("WIN-M-006", "metric_selector_order=[ドル,トークン]", "metric_initial=ドル")
require_fragments("WIN-M-013", "Help route instance=1", "additional HWND delta=0")
require_fragments("WIN-M-011", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true", "HelpScopeGeneration=monotonic", "HelpCloseToken=single_use", "caller_identity=(PID,starttime_ticks,HWND,WindowInstanceGeneration)", "Main_generation=monotonic", "restore_immediately_before_revalidation=true", "old_HWND_message_count=0", "first_caller_fixed=true")
require_fragments("WIN-M-012", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true", "HelpScopeGeneration=monotonic", "HelpCloseToken=single_use", "caller_identity=(PID,starttime_ticks,HWND,WindowInstanceGeneration)", "Main_generation=monotonic", "restore_immediately_before_revalidation=true", "old_HWND_message_count=0", "first_caller_fixed=true")
require_fragments("WIN-M-013", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true", "HelpScopeGeneration=monotonic", "HelpCloseToken=single_use", "caller_identity=(PID,starttime_ticks,HWND,WindowInstanceGeneration)", "Main_generation=monotonic", "restore_immediately_before_revalidation=true", "old_HWND_message_count=0", "first_caller_fixed=true")
require_fragments("WIN-M-014", "2125223e9996", "e3b0c44298fc", "a30afe326a99")
require_fragments("WIN-M-014", "valid_selector.auto_reconnect=true", "old4_corrupt_invalid=Main_disconnected+Settings_recovery", "recovery_command_count=0", "Setup_confirmation_on_reconnect=0")
require_fragments("WIN-M-007", "stage_boundary_count=4", "contract_hash_count=5", "stage_hashes=[native=d9bf9d1429ef", "owner=860d7ec45d6e53357b6f94201154d5a642fee9611bdb7bb410df5f712ea5f249", "record=76ce097b4412b61b95131c80ae36ddc1768a4fa119591e067f6e3de9d4519d8b", "rest=461e0f28bdde", "presentation=8b2fe86ea389", "record_policy_cases=[LIVE_OVERSIZE_PROVEN_TOOL_THEN_TERMINAL", "LOCAL_PARTIAL_LATER_SNAPSHOT", "local_model_column_order=[input_tokens,cached_input_tokens,output_tokens,input_dollars,cached_input_dollars,output_dollars]")
require_fragments("WIN-M-010", "stage_boundary_count=4", "contract_hash_count=5", "stage_hashes=[native=d9bf9d1429ef", "owner=860d7ec45d6e53357b6f94201154d5a642fee9611bdb7bb410df5f712ea5f249", "record=76ce097b4412b61b95131c80ae36ddc1768a4fa119591e067f6e3de9d4519d8b", "rest=461e0f28bdde", "presentation=8b2fe86ea389", "record_policy_cases=[LIVE_OVERSIZE_PROVEN_TOOL_THEN_TERMINAL", "LOCAL_PARTIAL_LATER_SNAPSHOT", "local_model_column_order=[input_tokens,cached_input_tokens,output_tokens,input_dollars,cached_input_dollars,output_dollars]")
require_fragments("WIN-M-015", "every ERROR-001 class=[API_UNREACHABLE,SSH_PROFILE_INVALID,SSH_LOCAL_PORT_IN_USE,SSH_INTERACTION_REQUIRED,SSH_PROCESS_START_OR_EXIT,SSH_HEALTH_UNAVAILABLE,AUTH_REQUIRED_OR_EXPIRED,AUTH_LAUNCH_FAILED,SETTINGS_CORRUPT,SETTINGS_SAVE_FAILED,STATUS_INVALID,DETAILS_INVALID,HISTORY_UNAVAILABLE,THREADS_UNAVAILABLE,DB_SERVER_ERROR,CLIPBOARD_WRITE_FAILED,INSTALL_OR_UPDATE_FAILED,UNINSTALL_FAILED,CLIENT_SHUTDOWN_TIMEOUT]", "SSH_PROFILE_INVALID", "SSH_LOCAL_PORT_IN_USE", "SSH_INTERACTION_REQUIRED", "SSH_PROCESS_START_OR_EXIT", "SSH_HEALTH_UNAVAILABLE", "SETTINGS_SAVE_FAILED", "action.settings.save.retry", "UNINSTALL_FAILED", "CLIENT_SHUTDOWN_TIMEOUT", "gate=19 class mappings")
require_fragments("WIN-M-018", "saved_selector_auto_reconnect=true", "Setup_confirmation_on_reconnect=0", "same_generation_auto_retry_infinite=0", "app_wide_supervisor_single_tunnel=true")
require_fragments("WIN-M-025", "UX-20260823-KEYBOARD-001", "keydown/keyup", "mouse")
require_fragments("WIN-M-025", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true", "HelpScopeGeneration=monotonic", "HelpCloseToken=single_use", "caller_identity=(PID,starttime_ticks,HWND,WindowInstanceGeneration)", "Main_generation=monotonic", "restore_immediately_before_revalidation=true", "old_HWND_message_count=0", "first_caller_fixed=true", "setup_profile_step_projection=profile×step visible+enabled Tab projection", "SetupOperationGeneration=monotonic", "stale_completion_commit=0", "RC-121 profile_action_semantics")
require_fragments("WIN-M-026", "reduced-motion")
require_fragments("WIN-M-028", "help_caller_identity=(PID,HWND,WindowInstanceGeneration)", "destroy_or_reuse_fallback=Main_pre_Help_route_nav.Help", "ShuttingDown_restore=0", "idempotent_close_Back_Escape=true", "HelpScopeGeneration=monotonic", "HelpCloseToken=single_use", "caller_identity=(PID,starttime_ticks,HWND,WindowInstanceGeneration)", "Main_generation=monotonic", "restore_immediately_before_revalidation=true", "old_HWND_message_count=0", "first_caller_fixed=true")
require_fragments(
    "WIN-M-030",
    "UX-20260823-ERROR-001",
    "UX-20260823-FULL-STATE-001",
    "UX-20260822-GRAPH-001",
    "UX-20260823-HELP-FOCUS-001",
    "UX-20260823-INSTALLER-001",
    "UX-20260823-KEYBOARD-001",
    "UX-20260822-UX-002",
    "UX-20260822-SSH-001",
    "UX-20260823-ACCESSIBILITY-SCALE-001",
    "UX-20260823-B2B-CUSTOMER-DELIVERY-001",
    "UX-20260823-RELEASE-SUPPLY-CHAIN-001",
)
require_fragments("WIN-M-030", "SSH-001/RC-061..063", "six-key schema", "silent REST GUI-zero", "PRODUCT_PENDING service command", "freeze_anchor=docs/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md")
require_fragments("WIN-M-030", "ACCESSIBILITY-SCALE-001", "B2B-CUSTOMER-DELIVERY-001", "RELEASE-SUPPLY-CHAIN-001", "eleven required decision records", "eleven records", "B2B-CUSTOMER-DELIVERY-001 required fields")
require_fragments("WIN-G-013", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "text_scale_dpi_separate=true", "UIA_Name_Description_owner=all_actionable_controls")
require_fragments("WIN-G-014", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "focus_scale_matrix_exact=true", "no_scroll=1")
require_fragments("WIN-G-015", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "text_scale_apply_once=true", "no_content_loss_200_225=true")
require_fragments("WIN-G-016", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "notification_dedup_exact=true", "unchanged_poll_notification=0")
require_fragments("WIN-M-021", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "text_scale_typography_exact=true", "no_content_loss_200_225=true")
require_fragments("WIN-M-024", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "semantic_owner_scale_join_exact=true", "UIA_full_text=true")
require_fragments("WIN-M-025", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "keyboard_scale_matrix_exact=true", "focus_notification_boundary=true")
require_fragments("WIN-M-026", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "state_scale_matrix_exact=true", "notification_dedup=true")
require_fragments("WIN-M-027", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "geometry_scale_boundary_exact=true", "scale_dpi_separate=true")
require_fragments("WIN-M-028", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "notification_input_boundary_exact=true", "no_new_window=true")
require_fragments("WIN-M-029", "UX-20260823-ACCESSIBILITY-SCALE-001", "text_scale_percent=[100,125,150,175,200,225]", "responsive_scale_matrix_exact=true", "text_scale_cells=6", "UIA_full_semantics=true")
require_fragments(
    "WIN-A-001",
    "registered_surface_inventory=[Main,Setup,Settings,Graph,Threads,Legal]",
    "Help={owner:Main,additional_hwnd:0}",
    "WIN-M-013",
)
require_fragments(
    "WIN-A-007",
    "timeZoneId enum=[local,UTC]",
    "host IANA",
    "UTC秒差",
    "arbitrary IANA",
)
require_fragments(
    "WIN-B-018",
    "WIN-I-013",
    "new Graph line/legend/color",
    "Other/その他",
)
require_fragments(
    "WIN-C-019",
    "floor(logical*dpi/96+0.5)",
    "logical threshold AND visible_frame fully contained in current rcWork",
    "topology sequence=[open on monitor B with negative origin",
    "taskbar shrink invalidates rect",
    "different-DPI remaining monitor",
)
require_fragments(
    "WIN-C-006",
    "reset_at=1780400000",
    "delta=86401",
    "delta=86400",
    "delta=86399",
    "ShuttingDown(UIなし)>AuthRequired>ErrorNoData",
    "quota danger(<=2%)>quota warning(<=10%)>reset warning(<=86400s)>Normal",
    "danger+resetはdanger",
    "warning+resetはwarning",
)
require_fragments(
    "WIN-C-012",
    "locale resolution=[ja-JP→ja,xx-YY→en,C/POSIX→en]",
    "catalog cases=[ja valid,en valid,en missing,en invalid UTF-8/key-set]",
    "0s=`Resetting soon`",
    "Remaining 1h 30m",
    "Remaining 1d 2h",
    "catalogのmissing/invalid UTF-8/required-key欠落はunknown locale fallbackと区別",
    "0日0時間",
)
require_fragments(
    "WIN-C-017",
    "logical client=900x480",
    "body left=30,width=840,right_exclusive=870",
    "floor(logical*dpi/96+0.5)",
    "左右差0",
)
require_fragments("WIN-A-019", "matrix=[quota valid+label valid", "quota null+label valid", "quota invalid+label valid", "quota valid+label invalid", "quota null+label validもroot/pairを受理", "quota未取得を表示してlabelだけ非表示", "invalid whole-candidate reject")
require_fragments("WIN-D-004", "thread_count=[0,1,2,3,4,6,7,256]", "page_count=ceil(thread_count/3)", "各IDは全page合計exactly once", "current/total page、Prev、Next、Back、Close、Refresh", "refresh減少時はnew_page_count=0ならcurrent_page=0")
require_fragments(
    "WIN-C-018",
    "exact 17 stateだけ",
    "wire API `state=ready`は入力事実でありcanonical UI state IDではない",
    "Graphは700x480",
    "fixed surfaceは900x480",
    "clip/overlap/root-or-inner-scroll/押し下げ0",
    "N/Aはreason",
)

# Cross-row contradiction guards. These compare independently owned contracts
# against their value authorities; they do not promote OPEN authority decisions
# or product evidence to PASS.
full_state_states = (
    "`initializing`, `auth_required`, `normal`, `quota_warning`, `quota_danger`, `reset_warning`, `zero`,",
    "`full`, `api_error`, `transport_error`, `status_invalid`, `details_invalid`, `history_error`,",
    "`thread_error`, `db_error`, `stale`, `no_history`",
)
require_doc_fragments(Path("docs/UX_DECISION_ERROR_RECOVERY_2026-08-23.md"), "SETTINGS_SAVE_FAILED", "action.settings.save.retry", "action.cancel", "旧primary設定bytes", "DB/history", "connection process")
require_doc_fragments(Path("docs/UX_DECISION_FULL_STATE_MATRIX_2026-08-23.md"), *full_state_states)
require_doc_fragments(
    Path("docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md"),
    "Main/Setup/Settings/Threads/Legal=logical `initial=min=max=900×480` fixed",
    "Graph=`initial=940×640,min=700×480,max=unbounded,resizable`",
    "Help=Main内900×480/additional HWND=0",
    "`language`、`setupCompleted`、`connectionConfigured`、`timeZoneId`、`connectionProfile`、`connectionSelector`の6 keyだけ",
    "profile enumは`none`、`wsl`、`sshConfigAlias`のいずれか",
)
for authority_path in (
    Path("docs/REST_API_V1.md"),
    Path("docs/DATA_PROTECTION_POLICY.md"),
):
    require_doc_fragments(
        authority_path,
        "application/json; charset=utf-8",
        "charset欠落",
    )
require_doc_fragments(
    Path("docs/REST_API_V1.md"),
    "`ready=true` fieldを作らず",
    "state == \"ready\" && authenticated == true",
    "概念的なshell例であり、Windowsクライアントの実行argv順序を定義しない",
    "Windowsのcanonical `ArgumentList`",
    "auth_required`のsecurity visibility transitionはdata pairのcommitではない",
    "status/details store、DB、pair generationは変更しない",
)
require_doc_fragments(
    Path("docs/CUSTOMER_OPERATIONS_RUNBOOK.md"),
    "状態: `EXTRACTION_CONTRACT / PRODUCT_PENDING`",
    "./run.sh",
    "CODEX_INFO_API_LISTEN=127.0.0.1:8787 ./run.sh",
    "Slint component/window/event-loop生成=0",
    "systemctl --user start codex-info-server.target",
    "GET /v1/health",
    "GET /v1/status",
    "systemctl --user stop codex-info-server.target",
    "systemctl --user restart codex-info-server.target",
    "./codex-info-server-setup install",
    "./codex-info-server-setup update",
    "codex-info-server-setup rollback",
    "codex-info-server-setup uninstall",
    "失敗時はnewを成功表示せずpreviousへatomic rollback",
    "通常uninstallはsettings、history DB、3 backup、source logsを削除しない",
    "codex-info-server-setup restore --generation 1",
    "codex-info-server-setup migrate --dry-run",
    "codex-info-server-setup migrate --apply",
    "失敗時は旧DB、旧memory、旧backupを保持",
    "Cargo、repository、`run.sh`を要求しない",
)
require_doc_fragments(
    Path("docs/REST_API_V1.md"),
    "RecorderDaemon",
    "独立writerであり、HTTP listenerを持たない",
    "SnapshotPublisher",
    "native UIとREST workerへ同じpairをread-onlyで渡す",
    "REST専用workerは`SnapshotPublisher`のread-only consumer",
    "UI/REST/`run.sh`はrecorderをspawnしない",
    "UI/RESTの終了はrecorderを停止しない",
    "daemonが単独で動く間に",
    "HTTP listenerを暗黙生成せず",
)
require_doc_fragments(
    Path("DESIGN.md"),
    "### REST snapshot publisher・read-only境界",
    "`record --interval 60` modeの`RecorderDaemon`はsource JSONLを検証して`UsageStore`へtransactional writeするだけでHTTP listenerを持たない",
    "`SnapshotPublisher`はcommit済みの完全な`DataGeneration/DataHash`",
    "native UIとREST workerは同じpairを読む",
    "片側だけ新しくしない",
)
require_doc_fragments(
    Path("docs/DATA_PROTECTION_POLICY.md"),
    "auth_required+authenticated=false`のauth-clearはsecurity visibility transition",
    "details/store/DB/pair bytesは不変",
)
require_doc_fragments(
    Path("DESIGN.md"),
    "保存の一意キーはlogical partitionを含む`(partition_id,reset_at,timestamp)`",
    "同一partition内の同じ`(reset_at,timestamp)`へ複数の有効snapshot",
    "wire projection内の表示列は`reset_at,timestamp`を保持するが、DB keyからpartitionを省略しない",
)
design_db_text = Path("DESIGN.md").read_text(encoding="utf-8")
for obsolete_design_key in ("`(reset_at,timestamp)`が一意キーで",):
    if obsolete_design_key in design_db_text:
        fail(f"DESIGN.md: storage key still omits partition_id ({obsolete_design_key})")
ssh_decision_path = Path("docs/UX_DECISION_SSH_CONNECTION_2026-08-23.md")
require_doc_fragments(
    ssh_decision_path,
    "`state=auth_required,authenticated=false`",
    "`state=ready,authenticated=true`",
    "`state=ready AND authenticated=true`の場合だけMain readyとする",
    "wireに`ready` booleanは存在しない",
)
ssh_decision_text = ssh_decision_path.read_text(encoding="utf-8")
for forbidden_fragment in ("`auth_required,false`", "`ready,true`", "`ready,false`", "`auth_required,true`"):
    if forbidden_fragment in ssh_decision_text:
        fail(f"{ssh_decision_path}: retains obsolete wire-state shorthand {forbidden_fragment}")
runbook_path = Path("docs/CUSTOMER_OPERATIONS_RUNBOOK.md")
require_doc_fragments(
    runbook_path,
    "`state=ready AND authenticated=true`の導出順序",
    "wire `ready` boolean field=0",
)
if "`ready=true`の順序" in runbook_path.read_text(encoding="utf-8"):
    fail("CUSTOMER_OPERATIONS_RUNBOOK: ready=true is ambiguous wire-state wording")
require_doc_fragments(
    Path("docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md"),
    "wireの`state=ready`は入力事実であり、client canonical UI state IDではなく",
    "17-state projection内の`normal`/`quota_warning`/`quota_danger`/`reset_warning`",
)
require_fragments("WIN-F-003", "persisted timeZoneId", "arbitrary IANA", "host IANA")
require_fragments("WIN-G-010", "enum=[local,UTC]", "local_once_host_resolution=true", "arbitrary_IANA_rejected=true")
require_fragments(
    "WIN-G-015",
    "registered_surface_inventory.count=6",
    "surface_controls={Main:[Minimize,Close],Setup:[Minimize,Close],Settings:[Minimize,Close],Graph:[Minimize,Maximize/Restore,Close,Resize],Threads:[Minimize,Close],Legal:[Minimize,Close]}",
    "cells.count=7560",
    "cells.count_dimensions=geometry×topology×DPI×theme×state×text_scale_percent direct product",
    "Help.owner=Main/additional_hwnd=0",
)
require_fragments("WIN-M-004", "setup_profile_step_projection=profile×step visible+enabled Tab projection", "SetupOperationGeneration=monotonic", "stale_completion_commit=0", "RC-121 profile_action_semantics", "action.StartForward is not a catch-all")
for installer_row_id in ("WIN-H-001", "WIN-H-002", "WIN-H-003", "WIN-H-004", "WIN-H-005", "WIN-H-006", "WIN-H-007", "WIN-H-008", "WIN-H-009", "WIN-H-010", "WIN-H-011", "WIN-H-012"):
    require_fragments(installer_row_id, "installer_failure_matrix=[crash_before_commit,crash_after_commit,reboot_mid_operation,journal_replay,duplicate_start,owner_lease_replay]", "operation_journal_replay_idempotent=true", "singleton_owner_lease=one", "owner_replay_match_required=true", "last_good_on_failure=true")
for lease_row_id in ("WIN-J-010", "WIN-J-013"):
    require_fragments(lease_row_id, "listener_owner_typed_join=(WIN-J-010.daemon_lease,WIN-J-013.singleton_owner_lease,WIN-J-016.REST_publisher_bootstrap_generation_cycle_tuple)", "cycle_tuple=(ProfileScopeId,AccountScopeId,StorageEpoch,SupervisorLeaseIdentity,CollectorEpoch,CycleSeq)")
require_fragments("WIN-J-016", "listener owner tuple=(ProfileScopeId,AccountScopeId,StorageEpoch,SupervisorLeaseIdentity,CollectorEpoch,CycleSeq)")
require_fragments(
    "WIN-I-001",
    "client_mismatch_rejected_before_connect=true",
    "server_404_is_separate_boundary=true",
    "health_status_auth_ready_separate=true",
    "installed_service_command=PRODUCT_PENDING",
)
require_fragments("WIN-I-001", "allowed_urls={health:http://127.0.0.1:8787/v1/health,status:http://127.0.0.1:8787/v1/status,details:http://127.0.0.1:8787/v1/details}", "editable=false", "external_destination_count=0", "redirect_follow=false")
require_fragments("WIN-I-002", "automatic_remote=true", "ArgumentList_only=true", "automatic_remote_BatchMode=yes", "hidden_prompt=0", "PowerShell_process_count=0")
require_fragments("WIN-I-015", "selector/profile are the only connection values persisted", "old4/corrupt/invalid selector recovery is Main_disconnected+Settings_recovery", "one-session raw is never durable completion")
require_fragments(
    "WIN-J-001",
    "route_method_status_matrix exact",
    "application/json; charset=utf-8",
    "cache_control=no-store",
)
require_fragments("WIN-J-001", "unknown_path=/v1/unknown", "case_altered_path=/v1/Status", "expected_status={GET:200,POST:405,PUT:405,DELETE:405}", "write_count=0", "DB fingerprint before=sha256:fixed", "DB/snapshot/memory/backup fingerprintは前後不変")
require_fragments(
    "WIN-J-003",
    "floor(event_epoch/60)*60=1787402040",
    "key=(partition_id,reset_at,timestamp)",
    "minute_column_absent=true",
)
require_fragments(
    "WIN-J-004",
    "12:34:00Z→1787402040",
    "12:34:59Z→1787402040",
    "12:35:00Z→1787402100",
    "minute_column_absent=true",
)
require_fragments(
    "WIN-J-005",
    "timestamp=floor(event_epoch/60)*60",
    "remaining tie-break",
    "minute_column_absent=true",
)
require_fragments(
    "WIN-K-013",
    "all topology_cases present",
    "single positive-origin same-DPI fixture",
    "supported predicate exact",
)
require_fragments("WIN-K-014", "topology_recovery center", "timer/poll/drag recenter=0", "dpi_integer")
require_fragments(
    "WIN-K-015",
    "Setup:[Minimize,Close]",
    "Settings:[Minimize,Close]",
    "Graph only exposes",
    "all_child_open=6",
)
require_fragments("WIN-L-008", "geometry_cases=7/topology_cases=5", "floor(logical*dpi/96+0.5)", "supported predicate")
require_fragments(
    "WIN-M-005",
    "timeZoneId persisted domain=[local,UTC]",
    "invalid_IANA_rejected=true",
)
require_fragments(
    "WIN-M-013",
    "registered_surface_inventory exact six",
    "runtime_total_hwnd=1..6",
    "Help={owner:Main,additional_hwnd:0}",
)
require_fragments("WIN-M-027", "floor(logical_axis*dpi/96+0.5)", "supported predicate exact", "one-shot topology recovery")
require_fragments("WIN-M-029", "all six Minimize+Close", "supported predicate exact", "floor(logical_axis*dpi/96+0.5)")

if "Welcome/Codex Infoへようこそ" in rows_by_id["WIN-E-001"][6]:
    fail("WIN-E-001 exact_expected contains a mixed-language Setup heading")
for automatic_row_id in ("WIN-E-006", "WIN-E-007", "WIN-E-008", "WIN-E-010", "WIN-K-002"):
    automatic_text = " ".join(rows_by_id[automatic_row_id][1:10])
    if (
        "[ssh.exe,-N,-L,8787:127.0.0.1:8787" in automatic_text
        or "ssh.exe -N -L 8787:127.0.0.1:8787" in automatic_text
        or "argv_token_count=5" in automatic_text
        or "fixed5" in automatic_text
        or "固定5 token列" in automatic_text
    ):
        fail(f"{automatic_row_id} retains the old five-token automatic SSH argv")
    if "automatic_remote_argv_exact=[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,<validated alias>]" not in automatic_text:
        fail(f"{automatic_row_id} lacks the exact automatic SSH argv")
if any(
    legacy in rows_by_id["WIN-E-011"][6]
    for legacy in ("SSH_FORWARD_FAILED", "ssh.forward.failed", "SSH_PROCESS_EXITED", "error.ssh.exited")
):
    fail("WIN-E-011 exact_expected contains a noncanonical failure class/key")
if "DPI=[100,150,200]" in " ".join(rows_by_id["WIN-M-025"][1:10]):
    fail("WIN-M-025 retains the obsolete non-integer fixture DPI set")
if "API_BACKEND_FAILURE" in rows_by_id["WIN-I-014"][6]:
    fail("WIN-I-014 exact_expected contains a noncanonical failure class")
if "概算ドル" in rows_by_id["WIN-M-006"][4] or "概算ドル" in rows_by_id["WIN-M-006"][6]:
    fail("WIN-M-006 metric selector must use the canonical label ドル")

ei_text = CONTRACTS[1].read_text(encoding="utf-8")
if "200000" in ei_text or "200,000" in ei_text:
    fail("E-I concrete contract contains unauthorized 200,000 history-sample limit")
setup_cancel_marker = "### WIN-M-004 / WIN-G-014 setup-cancel projection (RC-082)"
if setup_cancel_marker not in ei_text:
    fail(f"{CONTRACTS[1]}: RC-082 setup-cancel projection is missing")
for setup_cancel_fragment in (
    "visible_cancel=true,cancel_and_reap_product_process=true,user_confirmation_before_exit=true,setup_complete=false",
    "visible_cancel=true,discard_unsaved_input=true,route=Settings",
    "WIN-F-007",
    "setup_complete=true,route=Settings,write_count=0",
    "WIN-E-011 orphan/tunnel/reap",
    "WIN-E-016 settings bytes/secret persistence",
    "source_id=WIN-M-004:first-launch",
    "source_id=WIN-M-004:reopen",
):
    if setup_cancel_fragment not in ei_text:
        fail(f"{CONTRACTS[1]}: RC-082 projection lacks exact fragment: {setup_cancel_fragment}")

trace_rows = table_rows(TRACE_MATRIX, re.compile(r"^\| WIN-[A-M]-\d{3} \|"))
trace_ids = [row[0] for row in trace_rows]
if len(trace_ids) != 226 or len(set(trace_ids)) != 226 or set(trace_ids) != set(baseline_ids):
    fail(
        "traceability matrix must contain the exact 226-ID set: "
        f"rows={len(trace_ids)} unique={len(set(trace_ids))} "
        f"missing={sorted(set(baseline_ids)-set(trace_ids))} "
        f"extra={sorted(set(trace_ids)-set(baseline_ids))}"
    )
trace_text = TRACE_MATRIX.read_text(encoding="utf-8")
for fragment in (
    "atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md",
    "atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md",
    "atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md",
    "atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md",
    "旧V14（履歴のみ）",
    "意味契約・受入oracle・正本値には使用しない",
):
    if fragment not in trace_text:
        fail(f"traceability matrix lacks canonical-authority marker: {fragment}")
trace_design_text = TRACE_DESIGN.read_text(encoding="utf-8")
for fragment in (
    "hard_prerequisite=412",
    "related_validation_join=165",
    "consumer row -> target producer",
    "hard graphの非自明SCCは0",
    "layer 0: WIN-I / WIN-J",
    "related edgeは実行を遮断せず",
    "target ID + baseline unique title",
):
    if fragment not in trace_design_text:
        fail(f"traceability design lacks typed-dependency marker: {fragment}")

canonical_decision_rows = table_rows(CANONICAL, re.compile(r"^\| `docs/UX_DECISION_[^`]+` \|"))
canonical_decision_inventory = [
    (row[0].strip("`"), row[1].strip("`"))
    for row in canonical_decision_rows
]
if canonical_decision_inventory != DECISION_INVENTORY:
    fail(
        "canonical Decision inventory differs from exact 11-record set: "
        f"actual={canonical_decision_inventory} expected={DECISION_INVENTORY}"
    )
for decision_path, decision_id in DECISION_INVENTORY:
    decision_text = Path(decision_path).read_text(encoding="utf-8")
    marker = f"Decision ID: `{decision_id}`"
    if decision_text.count(marker) != 1:
        fail(f"{decision_path}: exact Decision ID marker count is not one for {decision_id}")
    if decision_path in {
        "docs/UX_DECISION_ACCESSIBILITY_SCALE_2026-08-23.md",
        "docs/UX_DECISION_RELEASE_SUPPLY_CHAIN_2026-08-23.md",
        "docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md",
    } and "PRODUCT_PENDING" not in decision_text:
        fail(f"{decision_path}: current product evidence boundary PRODUCT_PENDING is missing")
if len(canonical_decision_inventory) != 11 or len(set(canonical_decision_inventory)) != 11:
    fail("canonical Decision inventory must contain 11 unique path/ID pairs")

rest_authority_text = Path("docs/REST_API_V1.md").read_text(encoding="utf-8")
data_authority_text = Path("docs/DATA_PROTECTION_POLICY.md").read_text(encoding="utf-8")
for dp_number in range(1, 12):
    dp_id = f"DP-REST-{dp_number:03d}"
    if dp_id not in rest_authority_text or dp_id not in data_authority_text:
        fail(f"{dp_id}: REST/API and DATA authority typed join is incomplete")

source_specs = [
    (Path("docs/REQUIREMENTS_LEDGER.md"), re.compile(r"^(?:DP-\d{3}|LIVE-\d{3})$"), 11),
    (Path("docs/REQUIREMENTS_AUDIT_2026-08-22.md"), re.compile(r"^AUD-\d{3}$"), 27),
    (
        Path("docs/WINDOWS_CLIENT_REQUIREMENTS.md"),
        re.compile(r"^(?:WIN-(?:INSTALL|PAR|DES|I18N|SET|ACC)-\d{2}|REG-\d{2})$"),
        40,
    ),
    (Path("docs/TEST_GAP_REGISTER_2026-08-22.md"), re.compile(r"^TG-[A-Z]+-\d{2}$"), 18),
]
source_ids: list[str] = []
for path, pattern, expected in source_specs:
    rows = table_rows(path, re.compile(r"^\| [A-Z]"))
    ids = [row[0] for row in rows if pattern.fullmatch(row[0])]
    if len(ids) != expected or len(set(ids)) != expected:
        fail(f"{path}: expected {expected} unique legacy source IDs, got {len(ids)}/{len(set(ids))}")
    source_ids.extend(ids)

if len(source_ids) != 96 or len(set(source_ids)) != 96:
    fail(f"legacy source union must be 96 unique IDs, got {len(source_ids)}/{len(set(source_ids))}")

crosswalk_rows = table_rows(CROSSWALK, re.compile(r"^\| (?:DP|LIVE|AUD|WIN-|REG|TG-)"))
normalized_crosswalk = []
for row in crosswalk_rows:
    line_no = row.pop()
    if len(row) != 4:
        fail(f"{CROSSWALK}:{line_no}: crosswalk row must have 4 cells, got {len(row)}")
    if any(not cell for cell in row):
        fail(f"{CROSSWALK}:{line_no}: empty crosswalk cell")
    row.append(line_no)
    normalized_crosswalk.append(row)

crosswalk_ids = [row[0] for row in normalized_crosswalk]
if len(crosswalk_ids) != 96 or len(set(crosswalk_ids)) != 96:
    fail(f"crosswalk must have 96 unique source IDs, got {len(crosswalk_ids)}/{len(set(crosswalk_ids))}")
if set(crosswalk_ids) != set(source_ids):
    fail(
        "crosswalk/source mismatch: "
        f"missing={sorted(set(source_ids)-set(crosswalk_ids))} "
        f"extra={sorted(set(crosswalk_ids)-set(source_ids))}"
    )

source_set = set(source_ids)
for source_id, targets, reason, _scope, line_no in normalized_crosswalk:
    target_items = [item.strip() for item in targets.split(",")]
    if not target_items or any(not item for item in target_items):
        fail(f"{CROSSWALK}:{line_no}: {source_id} has an empty target")
    if len(target_items) != len(set(target_items)):
        fail(f"{CROSSWALK}:{line_no}: {source_id} has duplicate targets")
    for target in target_items:
        if target in source_set:
            continue
        if target.startswith("WIN-"):
            if target not in known:
                fail(f"{CROSSWALK}:{line_no}: {source_id} references unknown target {target}")
            continue
        if target.startswith("GLOBAL:"):
            global_id = target.removeprefix("GLOBAL:")
            if not re.match(r"^(?:DP|LIVE|AUD)-", global_id):
                fail(f"{CROSSWALK}:{line_no}: unsupported global promotion namespace {target}")
            if global_id not in source_set:
                fail(f"{CROSSWALK}:{line_no}: {source_id} references unknown global source {global_id}")
            continue
        fail(f"{CROSSWALK}:{line_no}: unsupported target syntax {target}")
    reason_parts = [part.strip() for part in reason.split(";")]
    reason_fields: dict[str, str] = {}
    for part in reason_parts:
        if "=" not in part:
            fail(f"{CROSSWALK}:{line_no}: {source_id} has a non-keyed reason part: {part}")
        key, value = (item.strip() for item in part.split("=", 1))
        if key in reason_fields:
            fail(f"{CROSSWALK}:{line_no}: {source_id} duplicates reason field {key}")
        reason_fields[key] = value
    expected_reason_fields = {"actor", "observable", "negative", "retention", "operation"}
    if set(reason_fields) != expected_reason_fields:
        fail(
            f"{CROSSWALK}:{line_no}: {source_id} reason fields differ: "
            f"missing={sorted(expected_reason_fields-set(reason_fields))} "
            f"extra={sorted(set(reason_fields)-expected_reason_fields)}"
        )
    empty_reason_fields = sorted(key for key, value in reason_fields.items() if not value)
    if empty_reason_fields:
        fail(f"{CROSSWALK}:{line_no}: {source_id} has empty reason fields {empty_reason_fields}")

crosswalk_by_id = {row[0]: row for row in normalized_crosswalk}
reg_global_authorities = {
    "REG-02": {"GLOBAL:AUD-002", "GLOBAL:LIVE-001"},
    "REG-05": {"GLOBAL:AUD-011"},
}
for source_id, required_globals in reg_global_authorities.items():
    target_set = {item.strip() for item in crosswalk_by_id[source_id][1].split(",")}
    missing_globals = sorted(required_globals - target_set)
    if missing_globals:
        fail(f"{source_id} lacks concrete global authority targets {missing_globals}")
    forbidden_self_target = f"GLOBAL:{source_id}"
    if forbidden_self_target in target_set:
        fail(f"{source_id} uses unresolved self-declared global target {forbidden_self_target}")


baseline_order = {row_id: index for index, row_id in enumerate(baseline_ids)}
approved_conflict_scope_targets = {
    "DP/LIVE/AUD/旧Windows/REG/TG 96件",
    "抽出状態文",
    "要求freeze集合",
    "旧96 crosswalk",
    "E-I trace row count",
    "全226の補助11列",
    "current ledger/status fields / `scripts/completion_guard.sh`",
}
source_ids_by_prefix: dict[str, dict[int, str]] = {}
for source_id in source_ids:
    prefix, number = source_id.rsplit("-", 1)
    source_ids_by_prefix.setdefault(prefix, {})[int(number)] = source_id


def expand_conflict_id_token(token: str, origin: str) -> list[str]:
    """Expand one conflict target, rejecting unknown IDs, namespaces, and endpoints."""
    if token in approved_conflict_scope_targets:
        return [token]

    global_prefix = token.startswith("GLOBAL:")
    raw_token = token.removeprefix("GLOBAL:") if global_prefix else token
    if global_prefix and (raw_token.startswith("INSTALL-") or raw_token.startswith("TG-INST-")):
        fail(f"{origin}: invalid GLOBAL promotion prefix {token}")
    if global_prefix and not re.match(r"^(?:DP|LIVE|AUD)-", raw_token):
        fail(f"{origin}: unsupported global promotion namespace {token}")

    current_match = re.fullmatch(r"WIN-([A-M])-(\d{3})", raw_token)
    if current_match:
        if global_prefix:
            fail(f"{origin}: GLOBAL promotion cannot target a current contract ID {token}")
        current_id = raw_token
        if current_id not in known:
            fail(f"{origin}: unknown current target {token}")
        return [token]

    current_range = re.fullmatch(
        r"WIN-([A-M])-(\d{3})\.\.(?:WIN-)?([A-M]-\d{3}|\d{3})",
        raw_token,
    )
    if current_range:
        if global_prefix:
            fail(f"{origin}: GLOBAL promotion cannot target a current contract range {token}")
        start_category, start_number, end_part = current_range.groups()
        end_category, end_number = (
            (start_category, end_part)
            if re.fullmatch(r"\d{3}", end_part)
            else end_part.split("-", 1)
        )
        start_id = f"WIN-{start_category}-{start_number}"
        end_id = f"WIN-{end_category}-{end_number}"
        if start_id not in known or end_id not in known:
            fail(f"{origin}: unknown current range endpoint {token}")
        start_index, end_index = baseline_order[start_id], baseline_order[end_id]
        if start_index > end_index:
            fail(f"{origin}: descending current range {token}")
        expanded = baseline_ids[start_index : end_index + 1]
        if not expanded or expanded[0] != start_id or expanded[-1] != end_id:
            fail(f"{origin}: current range endpoint expansion mismatch {token}")
        return [f"GLOBAL:{item}" if global_prefix else item for item in expanded]

    source_direct = raw_token in source_set
    if source_direct:
        return [f"GLOBAL:{raw_token}" if global_prefix else raw_token]

    source_range = re.fullmatch(
        r"([A-Z][A-Z0-9-]*)-(\d{2,3})\.\.(?:(?:([A-Z][A-Z0-9-]*)-)?(\d{2,3}))",
        raw_token,
    )
    if source_range:
        start_prefix, start_number, end_prefix, end_number = source_range.groups()
        end_prefix = end_prefix or start_prefix
        if end_prefix != start_prefix:
            fail(f"{origin}: source range changes namespace/prefix {token}")
        source_by_number = source_ids_by_prefix.get(start_prefix, {})
        start_number_int, end_number_int = int(start_number), int(end_number)
        if start_number_int > end_number_int:
            fail(f"{origin}: descending legacy source range {token}")
        missing_source_ids = [
            f"{start_prefix}-{number:0{len(start_number)}d}"
            for number in range(start_number_int, end_number_int + 1)
            if number not in source_by_number
        ]
        if missing_source_ids:
            fail(f"{origin}: unknown legacy source range endpoint/member {missing_source_ids}")
        expanded = [source_by_number[number] for number in range(start_number_int, end_number_int + 1)]
        return [f"GLOBAL:{item}" if global_prefix else item for item in expanded]

    fail(f"{origin}: unsupported conflict target namespace or range {token}")


conflict_rows = table_rows(CONFLICTS, re.compile(r"^\| RC-\d{3} \|"))
conflict_numbers: list[int] = []
conflict_targets_by_number: dict[int, set[str]] = {}
conflict_target_token_count = 0
conflict_expanded_target_count = 0
allowed_conflict_states = {
    "OPEN",
    "OPEN_AUTHORITY_CONFLICT",
    "FIX_IN_PROGRESS",
    "FIXED_PENDING_FRESH_AUDIT",
    "CLOSED",
}
for row in conflict_rows:
    line_no = row.pop()
    if len(row) != 5:
        fail(f"{CONFLICTS}:{line_no}: conflict row must have 5 cells, got {len(row)}")
    if any(not cell for cell in row):
        fail(f"{CONFLICTS}:{line_no}: empty conflict cell")
    match = re.fullmatch(r"RC-(\d{3})", row[0])
    if match is None:
        fail(f"{CONFLICTS}:{line_no}: malformed conflict ID {row[0]}")
    conflict_numbers.append(int(match.group(1)))
    target_tokens = [item.strip() for item in row[1].split(",")]
    if not target_tokens or any(not item for item in target_tokens):
        fail(f"{CONFLICTS}:{line_no}: empty conflict target token")
    conflict_target_token_count += len(target_tokens)
    expanded_targets: list[str] = []
    current_category_context: str | None = None
    for target_token in target_tokens:
        shorthand_number = re.fullmatch(r"\d{3}", target_token)
        if shorthand_number:
            if current_category_context is None:
                fail(
                    f"{CONFLICTS}:{line_no}:{row[0]}: shorthand current target lacks a category: "
                    f"{target_token}"
                )
            target_token = f"WIN-{current_category_context}-{target_token}"
        expanded_targets.extend(
            expand_conflict_id_token(target_token, f"{CONFLICTS}:{line_no}:{row[0]}")
        )
        current_match = re.fullmatch(r"WIN-([A-M])-\d{3}", target_token)
        current_range = re.fullmatch(
            r"WIN-([A-M])-\d{3}\.\.(?:WIN-)?([A-M])-\d{3}", target_token
        )
        current_range_same_category = re.fullmatch(
            r"WIN-([A-M])-\d{3}\.\.\d{3}", target_token
        )
        if current_match:
            current_category_context = current_match.group(1)
        elif current_range:
            current_category_context = current_range.group(2)
        elif current_range_same_category:
            current_category_context = current_range_same_category.group(1)
    duplicate_expanded_targets = sorted(
        target for target, count in Counter(expanded_targets).items() if count != 1
    )
    if duplicate_expanded_targets:
        fail(f"{CONFLICTS}:{line_no}: duplicate expanded conflict targets {duplicate_expanded_targets}")
    conflict_expanded_target_count += len(expanded_targets)
    conflict_targets_by_number[int(match.group(1))] = {
        target for target in expanded_targets if target in known
    }
    state = row[4].split(" / ", 1)[0]
    if state not in allowed_conflict_states:
        fail(f"{CONFLICTS}:{line_no}: unknown conflict state {state}")
if not conflict_numbers or conflict_numbers != list(range(1, max(conflict_numbers) + 1)):
    fail(f"{CONFLICTS}: conflict IDs must be unique contiguous RC-001..RC-max: {conflict_numbers}")
conflict_text_by_number = {
    int(row[0].removeprefix("RC-")): " ".join(row[1:5])
    for row in conflict_rows
}

# B2B values have one authority owner (§14) and a row-specific typed projection.
# Recalculate the projection from the conflict ledger; never accept a hand-maintained
# count or a generic source-owner marker as evidence of per-row propagation.
b2b_conflict_numbers = [*range(122, 130), *range(150, 160)]
expected_b2b_projection: dict[str, list[str]] = {}
for conflict_number in b2b_conflict_numbers:
    target_ids = conflict_targets_by_number.get(conflict_number, set())
    if not target_ids:
        fail(f"{CONFLICTS}: RC-{conflict_number:03d} has no current B2B concrete target")
    for target_id in target_ids:
        expected_b2b_projection.setdefault(target_id, []).append(f"RC-{conflict_number:03d}")
for target_id in expected_b2b_projection:
    expected_b2b_projection[target_id].sort(key=lambda value: int(value.removeprefix("RC-")))

b2b_projection_rows = table_rows(B2B_PROJECTIONS, re.compile(r"^\| `WIN-[A-M]-\d{3}` \|"))
actual_b2b_projection: dict[str, list[str]] = {}
for row in b2b_projection_rows:
    line_no = row.pop()
    if len(row) != 2:
        fail(f"{B2B_PROJECTIONS}:{line_no}: projection row must have 2 cells, got {len(row)}")
    row_id = row[0].removeprefix("`").removesuffix("`")
    raw_rc_set = row[1].removeprefix("`").removesuffix("`")
    if row_id in actual_b2b_projection:
        fail(f"{B2B_PROJECTIONS}:{line_no}: duplicate projection row {row_id}")
    rc_set = raw_rc_set.split(",")
    if any(re.fullmatch(r"RC-(?:12[2-9]|15[0-9])", value) is None for value in rc_set):
        fail(f"{B2B_PROJECTIONS}:{line_no}: invalid B2B RC set {raw_rc_set}")
    canonical_rc_set = sorted(set(rc_set), key=lambda value: int(value.removeprefix("RC-")))
    if rc_set != canonical_rc_set:
        fail(f"{B2B_PROJECTIONS}:{line_no}: RC set is duplicate or non-canonical {raw_rc_set}")
    actual_b2b_projection[row_id] = rc_set

if actual_b2b_projection != expected_b2b_projection:
    fail(
        "B2B row projection differs from expanded conflict targets: "
        f"missing={sorted(set(expected_b2b_projection)-set(actual_b2b_projection))} "
        f"extra={sorted(set(actual_b2b_projection)-set(expected_b2b_projection))} "
        f"mismatch={sorted(row_id for row_id in set(actual_b2b_projection)&set(expected_b2b_projection) if actual_b2b_projection[row_id] != expected_b2b_projection[row_id])}"
    )
projection_category_counts = Counter(row_id[4] for row_id in actual_b2b_projection)
if len(actual_b2b_projection) != 79 or sum(projection_category_counts.get(category, 0) for category in "ABCD") != 0:
    fail(f"B2B projection cardinality differs: total={len(actual_b2b_projection)} categories={dict(projection_category_counts)}")
if sum(projection_category_counts.get(category, 0) for category in "EFGHI") != 33:
    fail(f"B2B E-I projection count differs: {dict(projection_category_counts)}")
if sum(projection_category_counts.get(category, 0) for category in "JKLM") != 46:
    fail(f"B2B J-M projection count differs: {dict(projection_category_counts)}")

b2b_projection_text = B2B_PROJECTIONS.read_text(encoding="utf-8")
b2b_decision_text = Path("docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md").read_text(encoding="utf-8")
for fragment in (
    "UX-20260823-B2B-CUSTOMER-DELIVERY-001",
    "b2b-customer-delivery-v1",
    "current_target_count=0",
    "projection_target_count=33",
    "projection_target_count=46",
    "total_projection_target_count=79",
    "implementation_resume=0",
):
    if fragment not in b2b_projection_text:
        fail(f"{B2B_PROJECTIONS}: missing projection anchor {fragment}")
for contract_path, expected_fragment in zip(
    CONTRACTS,
    ("A-D target=0", "E-I target=33", "J-M target=46"),
    strict=True,
):
    contract_text = contract_path.read_text(encoding="utf-8")
    if str(B2B_PROJECTIONS) not in contract_text or expected_fragment not in contract_text:
        fail(f"{contract_path}: missing B2B projection companion/count {expected_fragment}")

required_b2b_authority_fragments = (
    "CodexInfo.WindowsClient.Setup.exe --install",
    "%LOCALAPPDATA%\\Programs\\Codex Info Monitor",
    "CodexInfo.WindowsClient.exe --export-diagnostics",
    "state=ready AND authenticated=true",
    "REMOTE_SCP_STAGING",
    "REMOTE_SSH_SERVER_CONTROL",
    "ja,en,zh-Hans,ko,es,fr,de,pt,it,ru",
    "unknown→`en`",
    "themeは`normal,high_contrast`",
    "surface_does_not_own_state|surface_does_not_own_failure|control_absent_by_design",
    "7 allowed flow",
    "customer document kindは次の6種類に固定する",
    "全scenarioで`dr_claim=customer_procedure_only`",
    "appがpointerを合成・移動しない",
    "scroll input 0",
    "keyboard/UIA route",
    "focus restore",
)
for fragment in required_b2b_authority_fragments:
    if fragment not in b2b_decision_text:
        fail(f"B2B §14 lacks exact selected authority fragment: {fragment}")
for forbidden_fragment in (
    "CodexInfoSetup.exe",
    "%LOCALAPPDATA%\\Programs\\CodexInfo",
    "CodexInfo.exe --export-diagnostics",
    "themeは`light,dark,high_contrast`",
):
    if forbidden_fragment in b2b_decision_text:
        fail(f"B2B authority retains conflicting obsolete value: {forbidden_fragment}")
b2b_privacy_section = b2b_decision_text.split("### 14.5 privacy、telemetry、operational flow", 1)[1].split("### 14.6", 1)[0]
expected_b2b_flows = {
    "LOCAL_REST_V1",
    "WSL_LOCAL_EXEC",
    "REMOTE_SCP_STAGING",
    "REMOTE_SSH_SERVER_CONTROL",
    "MANAGED_SSH_TUNNEL",
    "CODEX_DELEGATED_AUTH_USAGE",
    "LOCAL_SUPPORT_EXPORT",
}
actual_b2b_flows = set(re.findall(r"(?m)^\| `([A-Z0-9_]+)` \|", b2b_privacy_section))
if actual_b2b_flows != expected_b2b_flows:
    fail(f"B2B §14.5 flow set differs: actual={sorted(actual_b2b_flows)}")

b2b_setup_section = b2b_decision_text.split("### 14.2 Windows Setupのinvocationとexit contract", 1)[1].split("### 14.3", 1)[0]
expected_b2b_operations = {"install", "update", "repair", "rollback", "uninstall", "help", "version"}
actual_b2b_operations = set(re.findall(
    r"(?m)^CodexInfo\.WindowsClient\.Setup\.exe --(install|update|repair|rollback|uninstall|help|version) ",
    b2b_setup_section,
))
if actual_b2b_operations != expected_b2b_operations:
    fail(f"B2B §14.2 operation set differs: actual={sorted(actual_b2b_operations)}")
expected_b2b_exit_results = {
    0: "success_or_no_change",
    2: "invalid_invocation",
    3: "unsupported_mode_scope_or_platform",
    4: "security_or_policy_rejected",
    5: "busy_or_foreign_owner",
    6: "artifact_signature_provenance_or_version_rejected",
    7: "staging_io_or_resource_failure",
    8: "operation_failed_rollback_complete",
    9: "recovery_required",
    10: "owned_child_failed",
    11: "evidence_or_invariant_failure",
}
actual_b2b_exit_results = {
    int(code): result
    for code, result in re.findall(r"(?m)^\| (\d+) \| `([^`]+)` \|", b2b_setup_section)
}
if actual_b2b_exit_results != expected_b2b_exit_results:
    fail(f"B2B §14.2 exit contract differs: actual={actual_b2b_exit_results}")

b2b_document_section = b2b_decision_text.split("### 14.4 release notes、known limitations、versioned guide", 1)[1].split("### 14.5", 1)[0]
expected_b2b_document_kinds = {
    "release_notes",
    "administrator_guide",
    "operator_guide",
    "end_user_guide",
    "support_and_privacy_guide",
    "accessibility_and_dr_statement",
}
actual_b2b_document_kinds = set(re.findall(
    r"(?m)^(release_notes|administrator_guide|operator_guide|end_user_guide|support_and_privacy_guide|accessibility_and_dr_statement)$",
    b2b_document_section,
))
if actual_b2b_document_kinds != expected_b2b_document_kinds:
    fail(f"B2B §14.4 document-kind set differs: actual={sorted(actual_b2b_document_kinds)}")
for fragment in (
    "document_version=<product SemVer>+doc.<positive revision>",
    "公開channelはrelease package",
    "installed local customer-docsだけ",
    "SHA-256とUTF-8 byte数",
    "独立reviewer",
):
    if fragment not in b2b_document_section:
        fail(f"B2B §14.4 document lineage lacks anchor: {fragment}")

b2b_dr_section = b2b_decision_text.split("### 14.8 DR claimと顧客責任", 1)[1].split("### 14.9", 1)[0]
expected_b2b_dr_scenarios = {
    "daemon_process_or_host_reboot",
    "sqlite_busy_full_or_io_failure",
    "sqlite_corruption_or_quick_check_failure",
    "migration_failure",
    "backup_rotation_failure",
    "explicit_restore_failure",
    "wsl_distribution_or_windows_host_loss",
}
actual_b2b_dr_scenarios = set(re.findall(
    r"(?m)^(daemon_process_or_host_reboot|sqlite_busy_full_or_io_failure|sqlite_corruption_or_quick_check_failure|migration_failure|backup_rotation_failure|explicit_restore_failure|wsl_distribution_or_windows_host_loss)$",
    b2b_dr_section,
))
if actual_b2b_dr_scenarios != expected_b2b_dr_scenarios:
    fail(f"B2B §14.8 DR scenario set differs: actual={sorted(actual_b2b_dr_scenarios)}")
for fragment in (
    "declared_rpo_value_and_unit_or_not_offered=not_offered",
    "declared_rto_value_and_unit_or_not_offered=not_offered",
    "数値fieldは存在してはならない",
    "数値RPO、数値RTO、SLA、availability、",
    "support windowを提供しない",
):
    if fragment not in b2b_dr_section:
        fail(f"B2B §14.8 DR forbidden-field boundary lacks anchor: {fragment}")

b2b_ui_section = b2b_decision_text.split("### 14.9 customer document UI exposureと公開条件", 1)[1].split("### 14.10", 1)[0]
expected_b2b_ui_facts = {
    "release notes / known limitations",
    "administrator/operator/end-user guide",
    "privacy / telemetry / support contact",
    "diagnostics export",
    "accessibility / DR statement",
    "license / third-party notices",
}
actual_b2b_ui_facts = set(re.findall(r"(?m)^\| ([^|]+?) \| [^|]+ \| [^|]+ \|$", b2b_ui_section))
actual_b2b_ui_facts.discard("customer fact/action")
actual_b2b_ui_facts.discard("---")
if actual_b2b_ui_facts != expected_b2b_ui_facts:
    fail(f"B2B §14.9 UI-owner fact set differs: actual={sorted(actual_b2b_ui_facts)}")
for fragment in (
    "表示ownerは",
    "HelpはMain内で追加HWND 0",
    "appがpointerを合成・移動しない",
    "scroll input 0",
    "keyboard/UIA route",
    "focus restore",
    "document version/digestが現行releaseと不一致ならbutton公開0",
):
    if fragment not in b2b_ui_section:
        fail(f"B2B §14.9 UI/focus/viewport boundary lacks anchor: {fragment}")

for fragment in (
    "source SHA、projection table SHA、Decision SHA、independent reviewerを同じrequirements freezeへjoinする",
    "projection_pass=0",
    "implementation_resume=0",
    "customer_delivery_eligible=0",
):
    if fragment not in b2b_projection_text:
        fail(f"{B2B_PROJECTIONS}: missing same-freeze/fail-closed oracle anchor: {fragment}")
for fragment in (
    "requirements_content_set_sha256",
    "semantic_audits",
    "docs/atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md",
):
    if fragment not in freeze_contract_text:
        fail(f"{FREEZE_CONTRACT}: missing B2B same-freeze schema join: {fragment}")

# RC-164..171 retain eight legacy meanings whose acceptance is cross-row. Rebuild
# the current target projection from the conflict ledger and require one 10-field
# companion contract per legacy source instead of accepting generic row markers.
legacy_gap_conflict_numbers = list(range(164, 172))
expected_legacy_gap_projection: dict[str, list[str]] = {}
for conflict_number in legacy_gap_conflict_numbers:
    target_ids = conflict_targets_by_number.get(conflict_number, set())
    if not target_ids:
        fail(f"{CONFLICTS}: RC-{conflict_number:03d} has no current legacy-gap target")
    for target_id in target_ids:
        expected_legacy_gap_projection.setdefault(target_id, []).append(f"RC-{conflict_number:03d}")
for target_id in expected_legacy_gap_projection:
    expected_legacy_gap_projection[target_id].sort(key=lambda value: int(value.removeprefix("RC-")))

legacy_gap_projection_rows = table_rows(LEGACY_GAP_PROJECTIONS, re.compile(r"^\| `WIN-[A-M]-\d{3}` \|"))
actual_legacy_gap_projection: dict[str, list[str]] = {}
for row in legacy_gap_projection_rows:
    line_no = row.pop()
    if len(row) != 2:
        fail(f"{LEGACY_GAP_PROJECTIONS}:{line_no}: projection row must have 2 cells, got {len(row)}")
    row_id = row[0].removeprefix("`").removesuffix("`")
    raw_rc_set = row[1].removeprefix("`").removesuffix("`")
    if row_id in actual_legacy_gap_projection:
        fail(f"{LEGACY_GAP_PROJECTIONS}:{line_no}: duplicate projection row {row_id}")
    rc_set = raw_rc_set.split(",")
    if any(re.fullmatch(r"RC-1(?:6[4-9]|7[01])", value) is None for value in rc_set):
        fail(f"{LEGACY_GAP_PROJECTIONS}:{line_no}: invalid legacy-gap RC set {raw_rc_set}")
    canonical_rc_set = sorted(set(rc_set), key=lambda value: int(value.removeprefix("RC-")))
    if rc_set != canonical_rc_set:
        fail(f"{LEGACY_GAP_PROJECTIONS}:{line_no}: RC set is duplicate or non-canonical {raw_rc_set}")
    actual_legacy_gap_projection[row_id] = rc_set

if actual_legacy_gap_projection != expected_legacy_gap_projection:
    fail(
        "legacy-gap row projection differs from expanded conflict targets: "
        f"missing={sorted(set(expected_legacy_gap_projection)-set(actual_legacy_gap_projection))} "
        f"extra={sorted(set(actual_legacy_gap_projection)-set(expected_legacy_gap_projection))} "
        f"mismatch={sorted(row_id for row_id in set(actual_legacy_gap_projection)&set(expected_legacy_gap_projection) if actual_legacy_gap_projection[row_id] != expected_legacy_gap_projection[row_id])}"
    )
legacy_gap_category_counts = Counter(row_id[4] for row_id in actual_legacy_gap_projection)
if len(actual_legacy_gap_projection) != 53:
    fail(f"legacy-gap projection cardinality differs: total={len(actual_legacy_gap_projection)}")
if sum(legacy_gap_category_counts.get(category, 0) for category in "ABCD") != 10:
    fail(f"legacy-gap A-D projection count differs: {dict(legacy_gap_category_counts)}")
if sum(legacy_gap_category_counts.get(category, 0) for category in "EFGHI") != 11:
    fail(f"legacy-gap E-I projection count differs: {dict(legacy_gap_category_counts)}")
if sum(legacy_gap_category_counts.get(category, 0) for category in "JKLM") != 32:
    fail(f"legacy-gap J-M projection count differs: {dict(legacy_gap_category_counts)}")

legacy_gap_rows = table_rows(LEGACY_GAP_PROJECTIONS, re.compile(r"^\| LEGACY-GAP-RC-1(?:6[4-9]|7[01]) \|"))
legacy_gap_ids = []
legacy_gap_text_by_rc: dict[int, str] = {}
for row in legacy_gap_rows:
    line_no = row.pop()
    if len(row) != 10:
        fail(f"{LEGACY_GAP_PROJECTIONS}:{line_no}: atomic contract must have 10 cells, got {len(row)}")
    if any(not cell for cell in row):
        fail(f"{LEGACY_GAP_PROJECTIONS}:{line_no}: atomic contract contains an empty cell")
    match = re.fullmatch(r"LEGACY-GAP-RC-(1(?:6[4-9]|7[01]))", row[0])
    if match is None:
        fail(f"{LEGACY_GAP_PROJECTIONS}:{line_no}: malformed atomic contract ID {row[0]}")
    rc_number = int(match.group(1))
    legacy_gap_ids.append(row[0])
    legacy_gap_text_by_rc[rc_number] = " ".join(row[1:])
expected_legacy_gap_ids = [f"LEGACY-GAP-RC-{number:03d}" for number in legacy_gap_conflict_numbers]
if legacy_gap_ids != expected_legacy_gap_ids or len(set(legacy_gap_ids)) != 8:
    fail(f"legacy-gap atomic IDs/order differ: actual={legacy_gap_ids} expected={expected_legacy_gap_ids}")

legacy_source_by_rc = {
    164: "TG-SET-02",
    165: "TG-INST-01",
    166: "TG-THREAD-01",
    167: "TG-DAEMON-01",
    168: "TG-DB-01",
    169: "TG-DB-02",
    170: "TG-INST-02",
    171: "TG-CI-01",
}
legacy_gap_required_fragments = {
    164: ("restart_count=2", "exit→launch1→exit→launch2", "Setup/Welcome occurrence=0", "2 PID"),
    165: ("client_state=[running,graceful_shutdown_in_progress,abnormal_exit_residue]", "15秒deadline", "foreign PID/handle/lock"),
    166: ("cases=[empty,single,multi,partial,malformed,duplicate,rpc_failure,stale]", "fresh_image_count=8", "reused_image_count=0"),
    167: ("event=[append,rotate,truncate,replace]", "device,inode,size,prefix generation", "cursor=0", "4MiB", "256MiB", "2GiB"),
    168: ("faults=[BUSY,LOCKED,IOERR,FULL,READONLY,PERMISSION,CORRUPT,BACKUP_VALIDATION,BACKUP_ROTATION,PRUNE_CONTENTION,MIGRATION_LOCK]", "partial row/switch/delete/synthetic recovery=0", "quick_check"),
    169: ("interrupts=[pre_switch_crash,source_lock,candidate_lock,rename_failure,post_intent_pre_commit_crash]", "old DBだけcurrent", "二重migration=0"),
    170: ("operation=[install,update,rollback,uninstall]", "registry_delete_denied", "shortcut_delete_denied", "resume_after_reboot"),
    171: ("checkpoints=[evidence_start,artifact_finalized]", "tracked_diff_count=0", "build-input untracked_count=0", "build後source change=0"),
}
for rc_number, source_id in legacy_source_by_rc.items():
    text = legacy_gap_text_by_rc.get(rc_number, "")
    missing = [fragment for fragment in (source_id, *legacy_gap_required_fragments[rc_number]) if fragment not in text]
    if missing:
        fail(f"LEGACY-GAP-RC-{rc_number:03d}: missing semantic anchors {missing}")
    conflict_text = conflict_text_by_number.get(rc_number, "")
    if source_id not in conflict_text:
        fail(f"RC-{rc_number:03d}: missing legacy source join {source_id}")
    crosswalk_current_targets = {
        target.strip()
        for target in crosswalk_by_id[source_id][1].split(",")
        if target.strip() in known
    }
    absent_crosswalk_targets = sorted(crosswalk_current_targets - conflict_targets_by_number[rc_number])
    if absent_crosswalk_targets:
        fail(f"RC-{rc_number:03d}: conflict projection omits legacy crosswalk targets {absent_crosswalk_targets}")

legacy_gap_text = LEGACY_GAP_PROJECTIONS.read_text(encoding="utf-8")
for fragment in (
    "total_projection_target_count=53",
    "A-D=10、E-I=11、J-M=32",
    "legacy_gap_projection_pass=0",
    "legacy96_pass=0",
    "implementation_resume=0",
    "release_publication=0",
):
    if fragment not in legacy_gap_text:
        fail(f"{LEGACY_GAP_PROJECTIONS}: missing projection/gate anchor {fragment}")
for contract_path, expected_fragment in zip(CONTRACTS, ("A-D target=10", "E-I target=11", "J-M target=32"), strict=True):
    contract_text = contract_path.read_text(encoding="utf-8")
    if str(LEGACY_GAP_PROJECTIONS) not in contract_text or expected_fragment not in contract_text:
        fail(f"{contract_path}: missing legacy-gap projection companion/count {expected_fragment}")
legacy_gap_markers = (
    str(LEGACY_GAP_PROJECTIONS),
    str(LEGACY_GAP_PROJECTIONS).removeprefix("docs/"),
    LEGACY_GAP_PROJECTIONS.name,
)
for path in (CANONICAL, ROW_CONTRACTS, TRACE_DESIGN, TRACE_MATRIX, FREEZE_CONTRACT, TRACKER):
    if not any(marker in path.read_text(encoding="utf-8") for marker in legacy_gap_markers):
        fail(f"{path}: missing legacy-gap companion join")

concrete_text_by_id = {row[0]: " ".join(row[1:10]) for row in contract_rows}
lifecycle_text_by_id = {row[0]: " ".join(row[1:7]) for row in lifecycle_rows}
for conflict_number in range(139, 160):
    target_ids = conflict_targets_by_number.get(conflict_number, set())
    if not target_ids:
        fail(f"{CONFLICTS}: RC-{conflict_number:03d} has no current concrete target")
    phase_key = "phase_RC139_149=[" if conflict_number <= 149 else "phase_RC150_159=["
    marker = f"RC-{conflict_number:03d}"
    for target_id in sorted(target_ids):
        concrete_text = concrete_text_by_id.get(target_id)
        if concrete_text is None or phase_key not in concrete_text or marker not in concrete_text:
            fail(
                f"{target_id}: missing {marker} propagation marker "
                f"{phase_key} in concrete contract"
            )
        lifecycle_text = lifecycle_text_by_id.get(target_id)
        if lifecycle_text is not None and (phase_key not in lifecycle_text or marker not in lifecycle_text):
            fail(
                f"{target_id}: missing {marker} propagation marker "
                f"{phase_key} in lifecycle contract"
            )

# Governance continuation/input/escalation contracts are a separate namespace.
# They gate completion, but they never change the product 226-ID set or its hard DAG.
gov_rows = table_rows(LEDGER, re.compile(r"^\| GOV-(?:THREAD-END|NO-INPUT-END|ESCALATION-100X) \|"))
normalized_gov_rows: list[list[str]] = []
for row in gov_rows:
    line_no = row.pop()
    if len(row) != 10:
        fail(f"{LEDGER}:{line_no}: governance row must have 10 cells, got {len(row)}")
    if any(not cell for cell in row):
        fail(f"{LEDGER}:{line_no}: governance row contains an empty cell")
    row.append(line_no)
    normalized_gov_rows.append(row)
gov_ids = [row[0] for row in normalized_gov_rows]
if gov_ids != GOV_IDS or len(set(gov_ids)) != 3:
    fail(f"governance ledger IDs/order differ: actual={gov_ids} expected={GOV_IDS}")

for path in [BASELINE, *CONTRACTS, CROSSWALK]:
    forbidden_text = path.read_text(encoding="utf-8")
    present = [gov_id for gov_id in GOV_IDS if gov_id in forbidden_text]
    if present:
        fail(f"{path}: governance IDs must not enter product/legacy ID sets or typed DAG: {present}")

canonical_gov_rows = table_rows(CANONICAL, re.compile(r"^\| `GOV-(?:THREAD-END|NO-INPUT-END|ESCALATION-100X)` \|"))
canonical_gov_ids = [row[0].strip("`") for row in canonical_gov_rows]
if canonical_gov_ids != GOV_IDS:
    fail(f"canonical governance inventory differs: actual={canonical_gov_ids} expected={GOV_IDS}")

governance_text_by_id = {row[0]: " ".join(row[1:10]) for row in normalized_gov_rows}
required_gov_fragments = {
    "GOV-THREAD-END": [
        "ACTIVE_GOAL→HOLD_OR_INCONCLUSIVE→REASSIGNMENT_RECORDED→ACTIVE_GOAL→VERIFIED→TERMINAL",
        "turn_end_unobserved",
        "next_turn_required=true",
        "continuation_epoch",
        "terminal_pass=0",
        "実eventなし",
        "no-op",
    ],
    "GOV-NO-INPUT-END": [
        "WAITING_FOR_INPUT",
        "nonterminal",
        "WAITING_FOR_INPUT→TERMINAL",
        "synthetic decision",
        "continuation_epoch",
        "waiting_terminal_count=0",
        "no-op",
    ],
    "GOV-ESCALATION-100X": [
        "approved_N=226ならtarget=226000",
        "target=max(226000,N×100)",
        "discovery",
        "escalation_epoch",
        "source",
        "split",
        "prevention",
        "implementation_resume=0",
        "product_id_set=226",
        "governance_work_unit_target",
    ],
}
for gov_id, fragments in required_gov_fragments.items():
    missing = [fragment for fragment in fragments if fragment not in governance_text_by_id[gov_id]]
    if missing:
        fail(f"{gov_id}: missing governance semantic anchors {missing}")

conflict_text_by_number = {int(row[0].removeprefix("RC-")): " ".join(row[1:5]) for row in conflict_rows}
for conflict_number, gov_id in zip(range(172, 175), GOV_IDS):
    if gov_id not in conflict_text_by_number.get(conflict_number, ""):
        fail(f"RC-{conflict_number:03d}: missing 1:1 governance ID join {gov_id}")

intake_text = INTAKE.read_text(encoding="utf-8")
completion_text = COMPLETION.read_text(encoding="utf-8")
agents_text = AGENTS.read_text(encoding="utf-8")
for marker in GOV_IDS:
    if marker not in intake_text and marker != "GOV-THREAD-END":
        fail(f"intake policy lacks governance join {marker}")
    if marker not in completion_text and marker != "GOV-ESCALATION-100X":
        fail(f"completion protocol lacks governance join {marker}")
if "no_api_turn_liveness_claim" not in agents_text and "turn終了後にもagent/processが生存するというliveness claim" not in completion_text:
    fail("governance truth boundary lacks no-API-turn-liveness claim")
for marker in (
    "product_id_set=226",
    "governance_work_unit_target",
    "approved_N=226なら226000",
    "approved_N>226ならmax(226000,N×100)",
):
    if marker not in intake_text:
        fail(f"intake policy lacks governance count namespace marker: {marker}")

tracker_text = TRACKER.read_text(encoding="utf-8")
current_tracker_block = tracker_text.split("| GOV-2026-08-22-REQUIREMENT-CONTINUATION", 1)[0]
for marker in (
    "GOV-2026-08-23-THREAD-END-ATOMIC",
    "GOV-2026-08-23-NO-INPUT-END-ATOMIC",
    "GOV-2026-08-23-ESCALATION-100X-ATOMIC",
):
    if current_tracker_block.count(marker) != 1:
        fail(f"current tracker block must contain one individual governance assignment: {marker}")
for marker in (
    "PAR-2026-08-23-AUTHORITY-REPAIR",
    "PAR-2026-08-23-DEPENDENCY-DAG",
    "SEMANTIC_HOLD",
    "MACHINE_STRUCTURE_PASS",
):
    if marker not in current_tracker_block:
        fail(f"current tracker block lacks fail-closed marker: {marker}")
current_has_machine_pass = "MACHINE_STRUCTURE_PASS" in current_tracker_block
current_has_machine_fail = "MACHINE_FAIL" in current_tracker_block
if current_has_machine_pass == current_has_machine_fail:
    fail("current tracker block must contain exactly one of MACHINE_STRUCTURE_PASS or MACHINE_FAIL")

print("id_structure=PASS current=226 legacy=96 baseline_titles=nonempty/unique")
print(
    "contract_structure=PASS columns=10 empty=0 "
    f"hard_prerequisite={hard_edge_count} related_validation_join={related_edge_count} "
    f"typed_total={hard_edge_count + related_edge_count} hard_cycle=0 hard_scc=0 hard_backward=0 "
    "dependencies=known/non-self/type-duplicate-free fixture_boundary=226"
)
print("row_contract_structure=PASS rows=226 columns=11 ids=exact concrete_set legacy_domain=history-only")
print("lifecycle_structure=PASS rows=58 columns=7 ids=unique")
print("authority_anchors=PASS")
print("decision_inventory=PASS records=11 exact_paths_and_ids=unique")
print("freeze_inventory=PASS entries=65 ordered exact_paths current_decisions=11_present")
print("crosswalk_structure=PASS targets=known global_sources=known")
print(
    "conflict_target_structure=PASS "
    f"raw_tokens={conflict_target_token_count} expanded_targets={conflict_expanded_target_count} "
    "current_ranges=known legacy_ranges=known global_promotions=DP/LIVE/AUD-source_set approved_scopes=known"
)
print(
    f"conflict_structure=PASS rows={len(conflict_rows)} "
    f"ids=RC-001..RC-{max(conflict_numbers):03d} columns=5 states=known"
)
print("phase_propagation=PASS RC-139..159 concrete_and_lifecycle_targets_joined")
print("b2b_projection=PASS conflicts=RC-122..129,RC-150..159 targets=79 A-D=0 E-I=33 J-M=46 operations=7 exit_codes=11 documents=6 flows=7 dr_scenarios=7 ui_facts=6 same_freeze_contract=PASS")
print("legacy_gap_projection=PASS conflicts=RC-164..171 sources=8 atomic_rows=8 targets=53 A-D=10 E-I=11 J-M=32")
print(
    "governance_contracts=PASS rows=3 "
    "ids=GOV-THREAD-END,GOV-NO-INPUT-END,GOV-ESCALATION-100X "
    "conflicts=RC-172..RC-174 product_id_set=226 api_turn_liveness_claim=0"
)
PY

rg -q --fixed-strings '状態: `EXTRACTION_INCOMPLETE / PRODUCT_CHANGE_FROZEN`' \
  docs/WINDOWS_REQUIREMENTS_CANONICAL_INDEX_2026-08-23.md \
  || fail "canonical index must remain extraction-blocked before independent PASS"

rg -q --fixed-strings '状態: `REQUIREMENTS_AUTHORITY / EXTRACTION_INCOMPLETE`' \
  docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md \
  || fail "value authority must remain extraction-incomplete before independent PASS"

if rg -q '仕様曖昧は0件|承認済みlocale|仕様衝突解消済み' \
  docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md \
  docs/WINDOWS_UX_SPEC.md \
  docs/UX_DECISION_NON_SCROLL_2026-08-22.md; then
  fail "current extraction documents contain a false resolved/approved claim"
fi

echo 'windows-requirements-extraction-check: MACHINE_GATE_PASS (overall extraction remains HOLD until independent semantic audits and conflict closure pass)'
exit 0

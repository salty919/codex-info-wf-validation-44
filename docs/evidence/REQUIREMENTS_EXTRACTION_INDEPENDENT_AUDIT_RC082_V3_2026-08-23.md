# Requirements extraction independent audit: RC-082 V3

- Audit date: 2026-08-23
- Scope: RC-082 only (WIN-M-004, WIN-G-014, WIN-E-011, WIN-E-016)
- Role: independent read-only evaluator
- Change policy: no source, contract, conflict, or guard edits; this requested audit artifact is the only added file
- Overall verdict: HOLD

## Gate commands and raw results

    git diff --check
    EXIT_CODE=0

    bash -n scripts/windows_requirements_extraction_check.sh
    EXIT_CODE=0

    bash scripts/windows_requirements_extraction_check.sh
    id_structure=PASS current=226 legacy=96 baseline_titles=nonempty/unique
    contract_structure=PASS columns=10 empty=0 hard_prerequisite=412 related_validation_join=165 typed_total=577 hard_cycle=0 hard_scc=0 hard_backward=0 dependencies=known/non-self/type-duplicate-free fixture_boundary=226
    row_contract_structure=PASS rows=226 columns=11 ids=exact concrete_set legacy_domain=history-only
    lifecycle_structure=PASS rows=58 columns=7 ids=unique
    authority_anchors=PASS
    decision_inventory=PASS records=11 exact_paths_and_ids=unique
    freeze_inventory=PASS entries=65 ordered exact_paths current_decisions=11_present
    crosswalk_structure=PASS targets=known global_sources=known
    conflict_target_structure=PASS raw_tokens=620 expanded_targets=1665 current_ranges=known legacy_ranges=known global_promotions=DP/LIVE/AUD-source_set approved_scopes=known
    conflict_structure=PASS rows=174 ids=RC-001..RC-174 columns=5 states=known
    phase_propagation=PASS RC-139..159 concrete_and_lifecycle_targets_joined
    b2b_projection=PASS conflicts=RC-122..129,RC-150..159 targets=79 A-D=0 E-I=33 J-M=46 operations=7 exit_codes=11 documents=6 flows=7 dr_scenarios=7 ui_facts=6 same_freeze_contract=PASS
    legacy_gap_projection=PASS conflicts=RC-164..171 sources=8 atomic_rows=8 targets=53 A-D=10 E-I=11 J-M=32
    governance_contracts=PASS rows=3 ids=GOV-THREAD-END,GOV-NO-INPUT-END,GOV-ESCALATION-100X conflicts=RC-172..RC-174 product_id_set=226 api_turn_liveness_claim=0
    windows-requirements-extraction-check: MACHINE_GATE_PASS (overall extraction remains HOLD until independent semantic audits and conflict closure pass)
    EXIT_CODE=0

    bash scripts/requirements_intake_guard.sh
    requirements-intake-guard: FAIL: requirements extraction is incomplete; implementation/evaluation/release remain blocked
    EXIT_CODE=1

The intake FAIL is the expected incomplete-requirements block, not an implementation defect. It prevents a PASS verdict.

## RC-082 state evidence

docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md:92 reports:

    RC-082 | WIN-M-004,WIN-G-014,WIN-E-011,WIN-E-016 | Setup各stepにvisible Cancelが必要だが、具体契約はBack/Closeだけで、初回とSettings再表示時の副作用が未定義 | WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md の WIN-M-004 / WIN-G-014 setup-cancel projection (RC-082) で、既存Escape branchの初回/再表示Cancel semanticsとWIN-E-011/E-016 failure oracleを部分joinした。ただしBackの「直前step」規則、初回Cancel後のMain disconnected＋Settings復帰、再表示Cancel時の旧6-key bytes保持を一つの現行正本で完全に確定できていないため、推測で閉じずOPENを維持する | OPEN / SETUP-UX

The OPEN state is correct and must not be promoted to FIXED or CLOSED by this audit.

## Direct contract and guard evidence

docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:95-102 explicitly joins the existing facts:

- First-launch Cancel: visible_cancel=true, cancel_and_reap_product_process=true, user_confirmation_before_exit=true, setup_complete=false.
- Setup reopened from Settings: visible_cancel=true, discard_unsaved_input=true, route=Settings, setup_complete=true, write_count=0.
- WIN-F-007 is the existing Cancel-confirmation and Settings-return source.
- WIN-E-011 is the orphan/tunnel/owned-process-reap failure oracle.
- WIN-E-016 is the settings-key and secret-persistence oracle.
- The contract explicitly says unconfirmed values must not be promoted to PASS.

Existing source facts independently visible in the bounded contract source include:

- WIN-E-011 (WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md:97): Cancel/failure preserves old settings/payload/user data and recovery route; an independent error fixture is required.
- WIN-E-016 (WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md:102): Cancel/failure does not persist credentials; settings/payload and user data are retained.
- WIN-F-007 (WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md:109): initial-setup return flow exists, with Cancel/failure preserving old data.
- WIN-G-014 (WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md:128): keyboard Tab traversal is an independently reviewed fixture; the RC-082 projection adds the setup-step Cancel join.

The machine guard checks the partial join directly:

    scripts/windows_requirements_extraction_check.sh:906
    setup_cancel_marker = "### WIN-M-004 / WIN-G-014 setup-cancel projection (RC-082)"
    scripts/windows_requirements_extraction_check.sh:907-908
    missing marker -> fail
    scripts/windows_requirements_extraction_check.sh:909-920
    required fragments include first-launch Cancel, reopen Cancel, WIN-F-007, setup_complete=true, route=Settings, write_count=0, WIN-E-011, WIN-E-016, and both source IDs

These facts support PASS for the explicit Cancel join only. They do not establish the unresolved Back predecessor-step rule, the first-Cancel Main disconnected + Settings recovery route, or complete preservation of the prior six-key settings bytes on re-display Cancel.

## Verdict

HOLD. RC-082 remains OPEN / SETUP-UX. The partial join is present and machine-checked, but the three unresolved semantics above remain OPEN. Other OPEN/authority conflicts, freeze closure, and product evidence were not evaluated or closed by this scoped audit.

## Audit target SHA256

    3d2062cdaa377e55a8286fada870cd9387e001913578d92bb40710371cd162c5  scripts/windows_requirements_extraction_check.sh
    9470eb4107e7ca61329b580b4c3ec5873111227aab8141536b6ebab734e75867  scripts/requirements_intake_guard.sh
    ca86fecd48252f6fd931b4a0605bac16c080f9b56581fa99423a2282a8fd61fa  docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md
    e5c66e4b6b320d43056ed7e58ee525b48b23859abc40cfb5419ff488413b550d  docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md
    b08599613c2425e26022e3123fc8ff5d58c423252a9d429bccf50edcbda33b85  docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md
    FILL_AFTER_FINAL_HASH  docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC082_V3_2026-08-23.md

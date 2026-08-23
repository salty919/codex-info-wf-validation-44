# 要件抽出機械ゲート実行記録（2026-08-23）

状態: MACHINE_GATE_PASS / EXTRACTION_INCOMPLETE / SEMANTIC_HOLD

## 実行

    scripts/windows_requirements_extraction_check.sh

結果:

    id_structure=PASS current=226 legacy=96 baseline_titles=nonempty/unique
    contract_structure=PASS columns=10 empty=0 hard_prerequisite=412 related_validation_join=165 typed_total=577 hard_cycle=0 hard_scc=0 hard_backward=0 dependencies=known/non-self/type-duplicate-free fixture_boundary=226
    row_contract_structure=PASS rows=226 columns=11 ids=exact concrete_set legacy_domain=history-only
    lifecycle_structure=PASS rows=58 columns=7 ids=unique
    authority_anchors=PASS
    decision_inventory=PASS records=11 exact_paths_and_ids=unique
    freeze_inventory=PASS entries=65 ordered exact_paths current_decisions=11_present
    crosswalk_structure=PASS targets=known global_sources=known
    conflict_target_structure=PASS raw_tokens=617 expanded_targets=1662 current_ranges=known legacy_ranges=known global_promotions=DP/LIVE/AUD-source_set approved_scopes=known
    conflict_structure=PASS rows=174 ids=RC-001..RC-174 columns=5 states=known
    phase_propagation=PASS RC-139..159 concrete_and_lifecycle_targets_joined
    b2b_projection=PASS conflicts=RC-122..129,RC-150..159 targets=79 A-D=0 E-I=33 J-M=46 operations=7 exit_codes=11 documents=6 flows=7 dr_scenarios=7 ui_facts=6 same_freeze_contract=PASS
    legacy_gap_projection=PASS conflicts=RC-164..171 sources=8 atomic_rows=8 targets=53 A-D=10 E-I=11 J-M=32
    governance_contracts=PASS rows=3 ids=GOV-THREAD-END,GOV-NO-INPUT-END,GOV-ESCALATION-100X conflicts=RC-172..RC-174 product_id_set=226 api_turn_liveness_claim=0
    windows-requirements-extraction-check: MACHINE_GATE_PASS (overall extraction remains HOLD until independent semantic audits and conflict closure pass)

## 解釈境界

- この記録はID集合、列構造、型付き依存、投影集合、正本アンカーの機械検査だけを示す。
- MACHINE_GATE_PASS は EXTRACTION_COMPLETE、製品実装、実機証拠、独立意味監査のPASSを意味しない。
- docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md はRC-001..171がOPENを含み、独立評価も全226行PASSではないため、抽出状態は EXTRACTION_INCOMPLETE のまま保持する。
- 監査対象外の値・サービスコマンド・製品artifact・実環境挙動は推測で補完しない。

## 高リスクcross-row突合の再実行

A-D/E-I/J-M具体契約へ意味アンカーを追加し、次のauthorityとの直接突合を機械化した。

- FULL-STATEの17 state集合（旧aliasやmonthly/idleのstate昇格を除外）
- Windowの900×480、Graph 940×640/700×480、Help追加HWND=0
- 設定6-keyと`none|wsl|sshConfigAlias` enum
- REST全応答の`application/json; charset=utf-8`とready導出（wire `ready` fieldなし）
- wire `state=ready`（API入力）とclient canonical 17-state（`normal`/`quota_*`/`reset_warning`）を同一IDと誤認しない境界
- A-DのStatus優先順、locale/catalog失敗分離、canonical grid、minimum viewport
- J-MのRC-167〜169補助oracleは`source_id=WIN-J-007..016` joinとして本体226行と分離し、補助表のraw ID重複を拒否

再実行結果は上記の同一 `MACHINE_GATE_PASS` 出力と一致した。これは直接矛盾がないことを示す構造・意味アンカー検査であり、OPEN authority、未取得freeze lineage、旧96 fresh失敗、製品実証の不足を解消したものではない。

# RC-083 独立評価証跡（2026-08-23）

## 判定

- RC-083 scoped verdict: **PASS**
- 全体要求抽出 verdict: **HOLD**

RC-083の契約伝播・競合状態・機械ゲートを、実装担当の結論を参照せずに確認した。ソースコードや既存のOPEN/OPEN_AUTHORITY_CONFLICT行は変更していない。

## 対象と受入条件の突合

- `docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md` の `WIN-M-006 / WIN-M-007 surface-navigation addendum (RC-083)` は、`WIN-M-012` の共有ナビゲーション所有権をGraph/Threadsへ明示的にjoinしている。
- 共通規則はGraph/Threads双方に、同一viewport内の `action.Back` と `title.Close` の可視性、keyboard/UIA到達性、Back/CloseのMain route遷移、表示値・last-good・DB・settingsの保持、非破壊（変更なし）を適用している。
- `source_id=WIN-M-006` と `source_id=WIN-M-007` の個別行は、各々 `action.Back`/`title.Close` の `visible+keyboard/UIA`、Graphの period/metric/toggle または Threadsの page/selection と last-good/DB/settings、surface別 bounds/focus/key/route/hash evidence scope を持つ。共有行だけでの合格を明示的に禁止している。
- `docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md` のRC-083は `FIXED_PENDING_FRESH_AUDIT / GRAPH+THREADS-UX` である。
- 他の未解決行は閉じていない。文書ヘッダは `OPEN / PRODUCT_CHANGE_FROZEN` のままで、後続のOPENおよびOPEN_AUTHORITY_CONFLICT行も保持されている。したがって全体判定はHOLDとした。

## 検証コマンドと生結果

### Machine gate

Command:

```text
bash scripts/windows_requirements_extraction_check.sh
```

Exit: `0`

Raw output:

```text
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
```

### Diff whitespace check

Command:

```text
git diff --check -- docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md scripts/windows_requirements_extraction_check.sh
```

Exit: `0`

Raw output: empty.

## SHA-256（監査時点）

監査対象3ファイルのSHA-256とHEADは、この証跡作成時に取得した値を下記へ記録する。

- HEAD: `0b6020bd786104154ad91810d8951ef6f5c23f39`
- `docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md`: `d40f5aa64632fe1dd98e69d5253bc002839db6e485e5eb4ef5c2c70fd4f9dd28`
- `docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md`: `d78791d8111223cabe3efb5d4327f32395b842f7bf1631b321d4f229c26b8da9`
- `scripts/windows_requirements_extraction_check.sh`: `1baabc3013db943ec4105e7216d85a4951ac0aa2938ad40e75aef546dcffff93`

## 未検証事項

このbounded auditの対象はRC-083の契約文書・競合行・machine gateであり、Graph/Threadsの実Windowsプロセス、UIA実測、fresh画像そのものは評価対象外である。addendumが要求する個別実行証拠が揃うまでは、RC-083は文書状態どおり `FIXED_PENDING_FRESH_AUDIT` に留まり、全体HOLDを解除しない。

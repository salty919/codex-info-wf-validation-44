# 要件抽出 fresh 独立監査 V5（2026-08-23）

判定: **FAIL（全体PASSへ昇格不可、編集なし）**

監査担当: `fresh_requirements_audit_v5`（現行bytesの独立読取専用評価）

## raw gate

- `requirements_intake_guard.sh`: exit=1、`requirements extraction is incomplete; implementation/evaluation/release remain blocked`
- `windows_requirements_extraction_check.sh`: exit=0、ただし構造ゲートのみ
- current=226 / legacy=96
- hard=412 / related=165 / total=577
- hard cycle=0 / SCC=0 / backward=0
- fixture_boundary=226
- conflict_structure=PASS、RC-001..RC-174、174行
- freeze_inventory=PASS、65 path、11 Decision inclusion
- B2B projection=79（A-D=0/E-I=33/J-M=46）
- legacy-gap projection=53（A-D=10/E-I=11/J-M=32）
- 最終出力は`MACHINE_GATE_PASS (overall extraction remains HOLD until independent semantic audits and conflict closure pass)`を明記

## 独立判定

- `MACHINE_STRUCTURE_PASS`/`MACHINE_GATE_PASS`はID・列・依存・path・projectionの構造PASSであり、意味監査、RC閉鎖、製品証拠、抽出完了を意味しない。
- canonicalは`EXTRACTION_INCOMPLETE / PRODUCT_CHANGE_FROZEN`、独立要求抽出監査未PASS。
- RC-020/023/028/031/036/047/054等が`OPEN`、`FIXED_PENDING_FRESH_AUDIT`も残る。
- 旧96 fresh記録は`88 PASS / 7 FAIL / 1 INCONCLUSIVE`。RC-164..171の伝播・再監査待ち。
- Decision/REST/DATA/UX参照先は実在し、Decision inventory 11件は構造PASS。ただしDecision IDの存在を承認・製品PASSへ昇格しない。
- 226具体契約の`fixture_only:`境界と、未知wire field・根拠のない固定値・派生値・自己依存を拒否する文書/ゲートは確認された。ただし意味監査PASSではない。
- freezeは65 pathの順序・実在・Decision inclusionまで。生成済み同一SHA freeze JSON/artifact lineageと製品artifact証拠は未確認で`INCONCLUSIVE`。

## 継続条件

RC OPEN/FIXED_PENDING、旧96の7 FAIL/1 INCONCLUSIVEを解消し、fresh独立意味監査を実施する。`requirements_intake_guard.sh`がPASSになるまで`EXTRACTION_INCOMPLETE`と未解決状態を保持し、freeze JSON/content SHA/reviewer分離/同一release lineageを推測で補わない。

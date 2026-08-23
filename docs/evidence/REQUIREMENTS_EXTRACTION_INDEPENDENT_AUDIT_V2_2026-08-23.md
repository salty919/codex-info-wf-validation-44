# 要件抽出 fresh 独立監査 V2（2026-08-23）

判定: **FAIL（抽出完了条件未達）**

監査担当: `fresh_requirements_audit_v2`（実装・文書修正担当とは別の読取専用評価）

## raw gate

- `windows_requirements_extraction_check.sh`: `MACHINE_GATE_PASS`
- ID: current=226 / legacy=96
- concrete contract: 226 rows / 10 columns / empty=0
- row contract: 226 rows / 11 columns
- dependency: hard=412 / related=165 / total=577 / hard cycle=0 / SCC=0 / backward=0
- conflicts: 174 rows / `RC-001..RC-174`
- crosswalk structure: `PASS`
- `requirements_intake_guard.sh`: `FAIL`（抽出未完了のため実装・評価・releaseをブロック）

監査rawは evaluator が実行環境の一時領域へ保存した。ここでは未取得のraw本文、SHA、freeze値を推測して補完しない。

## 判定内訳

1. 226 ID集合・構造: `PASS`。10列具体契約と11列補助台帳の構造は確認された。
2. 必須フィールドの意味監査: `INCONCLUSIVE`。指定packetの `docs/WINDOWS_REQUIREMENTS_BASELINE_2026-08-22.md` は実在せず、実在するbaselineは `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`。read manifest mismatchを解消するまで意味PASSへ昇格しない。
3. 依存DAG: 構造 `PASS`（412/165/577）。全226行の独立意味監査完了は示さない。
4. RC矛盾: `FAIL`。RC-001..171のOPEN/OPEN_AUTHORITY_CONFLICT、RC-122..159の顧客・供給網・REST/daemon authority未決、RC-164..171の旧96 gapが残る。RC-172..174は工程namespaceの継続gateであり、製品抽出PASSへ昇格しない。
5. 旧96 crosswalk: 構造 `PASS` だが fresh記録は `88 PASS / 7 FAIL / 1 INCONCLUSIVE`。FAILは `TG-SET-02, TG-INST-01, TG-THREAD-01, TG-DAEMON-01, TG-DB-01, TG-DB-02, TG-INST-02`、INCONCLUSIVEは `TG-CI-01`。
6. HOLD/INCONCLUSIVE: canonicalは226意味監査中・旧96再監査待ち・`EXTRACTION_INCOMPLETE`。E-Iの実artifact/runtime証拠も全72件 `INCONCLUSIVE`。
7. ハルシネーション境界: `fixture_only` を製品証拠へ昇格させず、missing/stale/different SHAは `INCONCLUSIVE/HOLD` とする規則は確認された。一方、`docs/evidence/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_2026-08-23.json` は未生成で、freeze/release SHAの独立再計算は未検証。

## 継続条件

- baselineの正しい実在pathを同一source revisionの監査packetへ明示する。
- freeze manifestを、定義済みの65 path・226/96 ID set・同一machine gate出力から生成する。ただし全conflict CLOSED、旧96 PASS、全226意味監査PASS前に `EXTRACTION_COMPLETE` へ変更しない。
- RC-001..171と旧96 gapを、未決authorityの推測なしに解決またはOPEN保持し、fresh独立再監査を再実施する。

この証跡は構造PASSを抽出完了へ読み替えず、未確認値・製品実証・freeze SHAを発明しない。

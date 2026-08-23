# 要件抽出 fresh 独立監査 V4（2026-08-23）

判定: **FAIL**（構造サブゲートPASS、製品証拠INCONCLUSIVE）

監査担当: `fresh_requirements_audit_v4`（現行bytesの読取専用評価、編集なし）

## 根拠

- `windows_requirements_extraction_check.sh`: exit 0。current=226、legacy=96、hard=412、related=165、total=577、hard cycle/backward=0。具体契約・projection・Decision inventory・governance contractは構造PASS。ただし出力は`overall extraction remains HOLD`を明記。
- `requirements_intake_guard.sh`: exit 1。`requirements extraction is incomplete; implementation/evaluation/release remain blocked`。
- 正本状態: canonical=`EXTRACTION_INCOMPLETE / PRODUCT_CHANGE_FROZEN`、traceability design=`IN_PROGRESS / HOLD`、traceability matrix=`EXTRACTION_INCOMPLETE / PRODUCT_EVIDENCE_PENDING`、row contracts=`EXTRACTION_INCOMPLETE`、freeze contract=`CONTRACT_DEFINED / FREEZE_NOT_CAPTURED`、B2B projection=`REQUIREMENTS_SELECTED / PRODUCT_PENDING / FRESH_AUDIT_REQUIRED`。
- `OPEN_CONFLICTS`: `OPEN / PRODUCT_CHANGE_FROZEN`。RC-064/066/088/091/142..171等にOPENまたは`OPEN_AUTHORITY_CONFLICT`が残存し、RC-172..174のみCLOSED。
- freeze JSON実体は確認できず、契約だけが存在する。製品欄・row contracts・U-01..U-05は同一SHAの実プロセス/画像/DB/artifact証拠未取得でINCONCLUSIVE。
- Decision/REST/DATA/UX参照先は実在し、Decision inventoryは11件でpath/ID重複なし。ただし存在確認PASSを要求抽出完了・製品PASSへ昇格しない。
- unknown ID、自己依存、型重複、DAG cycle、synthetic ID兆候は構造検査で検出されなかった。226とgovernance target 226,000のnamespace分離も維持されている。

## 未検証と継続条件

226行全意味監査PASS、全RCのOPEN解消、freeze JSONとsource/artifact lineage、Windows実機・画像・process・DB・installer証拠は未検証。OPENを推測で閉じず、freeze capture後の同一bytesへ別fresh evaluatorを再実施し、`requirements_intake_guard.sh`がPASSになるまで実装・評価・releaseへ昇格しない。

なお、トラッカーの`MACHINE_STRUCTURE_PASS`は機械構造サブゲートを示すだけで、正本の`EXTRACTION_INCOMPLETE/HOLD`と矛盾しない。`MACHINE_GATE_PASS`をoverall PASSへ読み替えない。

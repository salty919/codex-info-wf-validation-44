# 並列作業・統合監視台帳（2026-08-22）

状態: `IN_PROGRESS / HOLD`

目的は要求を分割して速く確認することであり、部分PASSやスレッド数を完了根拠にしない。全行の統合と
同一artifact SHAの最終判定は `/root` が所有する。サブスレッドがタイムアウト・中断・未報告なら、対象行を
`HOLD/INCONCLUSIVE` として別担当へ再配置する。

| 作業ID | スレッド | 所有範囲 | 変更許可 | ゲート | 状態 |
| --- | --- | --- | --- | --- | --- |
| PAR-2026-08-22-UX-RECONCILE | ux_design_reconcile | DESIGN旧scroll記述とUX Decision整合 | DESIGN.md、UX監査文書のみ | 正本Decision参照、旧記述を整合対象と明記、製品PASSは不可 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-LIFECYCLE-AUDIT | lifecycle_contract_audit | WIN-D/K/M lifecycle契約 | 抽出監査文書のみ | 行固有条件・欠落欄の監査、実機未検証はINCONCLUSIVE | IN_PROGRESS/HOLD |
| PAR-2026-08-22-DATA-PROTECTION-AUDIT | data_protection_contract_audit | WIN-I/J・DB/daemon保護契約 | データ保護監査文書のみ | 複数writer、backup/migration、復旧、負荷、SHA契約の行固有突合 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-EXTRACTION-GATE-V3 | extraction_gate_audit_v3 | 全226・11列・U契約・UX正本 | 抽出監査文書のみ | read-only独立評価、未達はPASSに丸めない | INCONCLUSIVE/HOLD |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V3 | domain_contract_reaudit_v3 | 最新のD/K/M 58行、I/J 32行、全226原子assertionの行固有性 | 抽出契約文書のみ | 最新文書を独立再突合し、共通テンプレート残存・ID欠落・境界不足をFAIL/INCONCLUSIVEで記録 | INTERRUPTED/INCONCLUSIVE |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V4 | atomic_contract_row_expander | 全226原子assertionの要求固有展開 | atomic assertions文書のみ | 226 ID完全性、5列の要求固有境界/観測/否定/依存/oracle、製品証拠未取得の分離 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V5 | domain_contract_reaudit_v3 | V3 override全226行の具体値・閾値・観測field再監査 | atomic assertions文書のみ | 最新V3を再読込し、カテゴリ共通テンプレート残存を正規化・行サンプルで判定 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V6 | domain_contract_reaudit_v3 | V4 override全226行の要求別case/expected/dependency/oracle再監査 | atomic assertions文書のみ | 最新V4の要求別値、実在依存ID、oracle期待値、製品証拠分離を判定 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V7 | domain_contract_reaudit_v3 | V5 override全226行のsemantic field・valid dependency・要求別oracle再監査 | atomic assertions文書のみ | unknown/self dependency=0、カテゴリ重複、synthetic field残存、製品証拠を独立判定 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V8 | domain_contract_reaudit_v3 | V7 override全226行の実在意味field・値域・負の入力・依存・oracle再監査 | atomic assertions文書のみ | 最新V7のみを読み、synthetic fallback・自己/未知依存・カテゴリ共通否定・製品証拠を判定 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V9 | domain_contract_reaudit_v3 | V8 override全226行のID別実在field・case・negative・oracle再監査 | atomic assertions文書のみ | 最新V8のみを読み、A/H共有field、カテゴリ内重複、製品証拠を判定 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-DOMAIN-RE-AUDIT-V12 | domain_contract_reaudit_v3 | V11 override全226行のID別実在field・negative・依存再監査 | atomic assertions文書のみ | 最新V11のみを読み、共有field/negative重複と製品証拠を判定 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-ATOMIC-GAP-REVIEW | atomic_contract_gap_review | atomic assertionsの共通テンプレート反復監査 | 監査文書のみ | 226 ID集合、行固有性FAILの具体例、製品証拠INCONCLUSIVEを独立記録 | COMPLETED/FAIL→V4対応 |
| PAR-2026-08-22-CONCRETE-A-D | atomic_contract_gap_review | WIN-A..D 76行の具体的な原子契約 | `docs/atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md`のみ | 76 ID、具体入力・期待値・否定保持・有効依存・独立oracle。自己反復/汎用語0 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-CONCRETE-E-I | domain_contract_reaudit_v3 | WIN-E..I 72行のSSH/settings/i18n/installer/API原子契約 | `docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md`のみ | 72 ID、値域・安全境界・失敗保持・独立oracle。自己反復/汎用語0 | IN_PROGRESS/HOLD |
| PAR-2026-08-22-CONCRETE-J-M | atomic_contract_row_expander | WIN-J..M 78行のDB/daemon/異常/evidence/UX原子契約 | `docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md`のみ | 78 ID、競合・復旧・境界・視覚oracle。自己反復/汎用語0 | IN_PROGRESS/HOLD |

統合条件は、全作業のraw報告を台帳へ取り込み、要求IDの重複・未所有・未評価を0件にし、次の段階で同じ成果物SHAへ
結び付けられる受入oracleを確定すること。ここで製品コード、テスト、ビルド、インストール、画面キャプチャは開始しない。

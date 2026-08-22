# EXTRACTION GATE INDEPENDENT AUDIT V4（2026-08-22）

## 判定概要

| 対象 | 判定 | 根拠 |
|---|---|---|
| baseline / register / 11-column contracts | PASS（構造） | baseline・register・11-column contractsは各226 ID。11-column contractsは226行すべて11列で、baselineとのID差分0、行内要求文不一致0。 |
| 最新V14 atomic | FAIL（case/expected欠落） | V14は226行、226 unique ID、未解析0、field 226 unique、negative 226 unique、dependency 441件、unknown/self 0。ただし要求列のcase/expected条件を満たさない行がWIN-E-008、WIN-J-001の2件。 |
| lifecycle / data-protection extensions | PASS（ID範囲） | Lifecycle 58 ID（D/K/M）、Data Protection 32 ID（I/J）を確認し、各IDは226要求集合内。 |
| U fixture contracts | FAIL（抽出リンク） | Graph parity fixtureはU-01〜U-04、UI label/input fixtureはU-04/U-05を定義するが、WIN-IDが0件。主台帳→原子assertion→fixtureのIDリンクがない。 |
| UX decisions | FAIL（抽出ゲート） | Decision文書自身のWIN-IDは突合可能だが、U-04の登録済みmapping IDやUX決定を226行のtraceabilityへ戻すper-IDリンクがない。Non-scroll文書は旧DESIGN.mdのScrollViewer記述が残ると明記している。 |
| traceability | FAIL | TRACEABILITY_MATRIX.mdは現行WIN-A〜MのIDを0件しか持たず、DP-001〜010、WIN-INSTALL、WIN-PAR等の旧ID体系を参照する。設計文書もカテゴリ設計中心で226行の実リンク台帳ではない。 |
| atomic全体のID衛生 | FAIL（履歴汚染） | baselineは226 IDだがatomic全体は228 ID。旧履歴行にbaseline外のWIN-D-014、WIN-F-016が残り、WIN-L-014/016のdependencyから参照される。最新V14自体は226 ID。 |
| 製品証拠 | INCONCLUSIVE（後段分離） | 実画像、実プロセス、DB/hostログ、artifact SHAは取得・検証していない。fixtureのPRODUCT_EVIDENCE_PENDINGは抽出FAILへ丸めず後段に分離する。 |

## 読み取り範囲

次の対象だけを読み取り、コード、テスト、ビルド、実機操作は行っていない。

- WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md
- WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md
- WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md
- WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md（末尾のV14を最新正本として評価）
- WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md
- WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md
- GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md
- UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md
- UX_DECISION_GRAPH_LABELS_2026-08-22.md
- UX_DECISION_NON_SCROLL_2026-08-22.md
- WINDOWS_UX_SPEC.md
- TRACEABILITY_MATRIX.md
- WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md

## 正本・ID・列の突合

- baseline、register、11-column contractsはいずれも226 IDで、A-001〜M-030の集合差分はない。
- 11-column contractsの行は226、各行のセル数は11。baseline requirement本文は全226行で同じ行のcontract内に見つかり、未マップ句は0。
- V14見出しはatomicの2855行付近。V14だけを抽出すると226行、226 unique ID、未解析0、baselineとの差分0。
- V14の全列（requirement/input/observable/negative/dependency/evidence）は非空で、field種類数226、negative種類数226。前回の共有field・negative重複はV14では解消された。
- V14のdependencyは441参照で、V14 ID集合外または自己依存は0。
- V14のsynthetic markerは0。
- case/case_valueまたはobservable/evidenceのexpected/expected_case条件を満たさない行はWIN-E-008、WIN-J-001の2件。要求抽出ゲートでは欠落としてFAILにする。

## 未マップ句・拡張リンク

### 未マップ句

baselineの226要求本文を、register、11-column contracts、最新V14、Lifecycle/Data拡張、Decision、UX、traceability設計の結合テキストへ照合した結果、要求本文そのものの未マップは0だった。これは本文の文字列出現を示すだけで、fixture・Decision・traceabilityのper-IDリンクを証明しない。

### Lifecycle / Data Protection

Lifecycle文書はD/K/Mの58 ID、Data Protection文書はI/Jの32 IDを持ち、いずれもbaseline外IDを持たない。11-column contractsの各行に拡張参照が置かれており、ID範囲の突合はPASSとした。ただし対象文書の状態はEXTRACTION_INCOMPLETEであり、製品証拠が未取得であることとは分離する。

### U fixture

GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md はU-01〜U-04を定義し、3071 points、seq 0..3070、remaining 0..100、plot許容差±1px、reset/now/timezone、同一SHA、欠測・終端・idle等の境界を具体化している。しかしWIN-A〜MのIDを一つも記録していない。

UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md はU-04/U-05を定義し、locale×timezone×stateのセル、label key、意味field、単位、period、系列順、色owner、X/Windows文字列、mapping ID、clip/overlap、SetCursorPos等の入力禁止境界を記録する。しかし同様にWIN-IDが0件である。U-04のUX-20260822-GRAPH-001は、226要求のどの行・どのDecisionが所有するか未定義である。

このため、fixtureの境界値自体は抽出されているが、主台帳→原子assertion→該当fixtureの要求追跡が閉じておらず、抽出ゲートはFAILとする。

## UX Decision・旧仕様衝突

- UX_DECISION_GRAPH_LABELSは7件、UX_DECISION_NON_SCROLLは8件、WINDOWS_UX_SPECは4件の現行WIN-IDを参照し、その局所IDはbaseline集合内である。
- TRACEABILITY_MATRIX.mdはWIN-A〜Mを参照せず、DP-001〜010、WIN-INSTALL-01〜04、WIN-PAR/WIN-DES、WIN-I18N/WIN-SET/WIN-ACC、REG-01〜11という旧ID体系を正本としている。これは現行226要求へのID差分でありFAIL。
- UX_DECISION_NON_SCROLLはDESIGN.mdに残るScrollViewer記述をDecisionへ整合させる対象とし、文書注記だけでは矛盾解消と扱わないと明記する。非スクロールの仕様選択は記録済みだが、旧仕様との衝突が未解消である。
- WINDOWS_UX_SPECは設計をFROZENとしながらEXTRACTION_INCOMPLETE/HOLDおよび実装後証拠未取得を併記する。設計決定の凍結と抽出ゲートの完了を混同してはならず、現状は抽出完了判定不可である。
- WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGNは11列、カテゴリ境界、D/K/M 58行、I/J 32行、atomic参照を規定するが、226個の行ごとのlink recordではない。設計規則としては有用だが、traceability matrixの代替証拠にはならない。

## 旧履歴のID差分

atomic全体のID集合は228件で、baseline/register/row contractsの226件を超える。baseline外のWIN-D-014とWIN-F-016は旧Override履歴のWIN-L-014/016 dependencyに現れる（atomic 1453/1455行付近）。V1〜V13を監査履歴として保持するというV14前置きはあるが、履歴内に現行台帳外IDを残したままなので、旧仕様・旧抽出の衝突候補として列挙する。V14セクションだけのID集合には混入していない。

## 未定義境界・矛盾

- U-01〜U-05の入力境界・数値境界は定義されているが、どのWIN-IDのoracleで受けるかが未定義。
- U-04の登録済みmapping ID UX-20260822-GRAPH-001は記載されるが、現行226行のowner/traceability先が未定義。
- U-05の静的API scan、process log、隔離VM opt-in、host入力を奪う試験のFAIL条件は記載されるが、WIN-G/K/Mのどの行へ帰属するかが未定義。
- V14ではE-008、J-001のcase/expected条件が欠落し、11-columnからatomicへの必須列変換が閉じていない。
- TRACEABILITY_MATRIXの旧ID体系と現行baselineのWIN-A〜M体系が併存し、正本のID名前空間が一意でない。
- UX Decisionは旧ScrollViewer記述の解消前提を明示し、WINDOWS_UX_SPECはFROZENとEXTRACTION_INCOMPLETE/HOLDを併記する。決定済み仕様と抽出完了状態を分離しない限りPASSできない。

## 製品証拠の分離

GRAPH_PARITY_FIXTURE、UI_LABEL_INPUT_FIXTURE、Lifecycle/Data、UX文書は、fresh画像、raw/process/DB/hostログ、同一SHA、独立reviewerなどを要求するが、実取得物の存在・内容・SHA一致はこの抽出監査では確認していない。したがって製品受入はINCONCLUSIVEであり、fixtureのID未リンクやtraceability旧IDによる抽出FAILとは別判定で記録する。

## 最終判定

- 抽出構造（baseline/register/11列/V14/Lifecycle/Data）: 部分PASS。
- 抽出ゲート総合: FAIL。V14の2行のcase/expected欠落、U fixtureのper-IDリンク欠落、現行traceability不在、旧ID体系、atomic履歴のbaseline外ID、旧ScrollViewer衝突が残る。
- 製品証拠: INCONCLUSIVE。後段の同一SHA実環境証拠が未取得。

本報告は読み取り専用監査の結果であり、対象文書、抽出状態、実装、テスト、ビルド、実機証拠は変更していない。


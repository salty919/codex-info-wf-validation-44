# Windows版 要求正本索引（2026-08-23）

状態: `EXTRACTION_INCOMPLETE / PRODUCT_CHANGE_FROZEN`

## 1. 目的

本書は、Windows版の実装開始前に読む要求正本を一意にする。件数だけが226件でも、
要求本文の意味、境界、保持条件、依存、独立oracleのいずれかが欠ければ抽出完了ではない。
旧台帳96件も削除・縮退せず、226件との逆引きを同時に閉じる。

要求抽出が完了する前は、Windows製品コード、テスト、ビルド、インストール、実機操作、
画面評価、成果物差し替えを行わない。文書の構造PASS、担当者の自己PASS、過去成果物のPASSは
この凍結を解除しない。

## 2. 正本集合と優先順位

矛盾時は次の順に解決する。下位文書や現行実装へ合わせて上位要求を黙って変更しない。

1. ユーザーの明示要求と後発の明示的訂正
2. リポジトリ直下 `AGENTS.md` の作業・証拠・完了制約
3. ユーザーの後発要求を記録したDecision/contract ID（現在は次の11件:
   `UX-20260822-UX-002`、`UX-20260822-GRAPH-001`、`UX-20260822-SSH-001`、
   `UX-20260823-ERROR-001`、`UX-20260823-KEYBOARD-001`、
   `UX-20260823-HELP-FOCUS-001`、`UX-20260823-FULL-STATE-001`、
   `UX-20260823-INSTALLER-001`、`UX-20260823-ACCESSIBILITY-SCALE-001`、
   `UX-20260823-RELEASE-SUPPLY-CHAIN-001`、
   `UX-20260823-B2B-CUSTOMER-DELIVERY-001`）。
   IDの存在だけからユーザー承認済みとは推定しない
4. `WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md` と、同書が直接参照する
   `REST_API_V1.md`、`DATA_PROTECTION_POLICY.md`、`LIVE_STATE_DECISION_MATRIX.md`、
   `DESIGN.md`、`WINDOWS_UX_SPEC.md`、`WINDOWS_CLIENT.md`、`LOCALIZATION.md`、
   `CUSTOMER_OPERATIONS_RUNBOOK.md`
5. ID集合の正本 `WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`
6. 行の具体契約3冊:
   - `atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md`（76件）
   - `atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md`（72件）
   - `atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md`（78件）
   - B2Bのrow別typed joinは
     `atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md`（current target 79件）を
     companion正本とし、3冊の同一ID行と一体で解決する
   - 旧要求のcross-row state/evidence joinは
     `atomic-contracts/WINDOWS_LEGACY_GAP_PROJECTIONS_2026-08-23.md`（current target 53件）を
     companion正本とし、RC-164..171・旧source 8件・3冊の同一ID行と一体で解決する
7. 旧要求の逆引き `WINDOWS_LEGACY_REQUIREMENT_CROSSWALK_2026-08-22.md`（96件）
8. 行の実装対象・証拠計画を補う
   `WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md`、領域別拡張台帳、
   `WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`（補助11列。各IDの意味は現行3具体契約へjoin）、
   `WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md`、
   `THREAD_PIPELINE_FIXTURE_CONTRACT_2026-08-23.md`、`REGRESSION_PREVENTION_POLICY.md`、
   `REQUIREMENTS_INTAKE_POLICY.md`、`COMPLETION_PROTOCOL.md`

古いNormative Override V1..V14、古い監査結果、`verified`と記載された旧台帳行、過去SHA、
過去画像は履歴資料であり、上記6の具体契約または現行製品受入の正本ではない。

上位資料同士が矛盾する場合は、後発かつ対象を具体的に限定した明示要求だけを優先できる。
その関係をDecisionまたはcrosswalkへ記録できない場合は `OPEN_AUTHORITY_CONFLICT` とし、
抽出ゲートをFAILにする。

### 2.1 現行Decisionのexact path/ID inventory

次の11 path/ID組を一意なDecision inventoryとして扱う。Decision本文の状態、未確定値、
証拠計画は各source documentをownerとし、IDだけを承認済み・製品実装済みの証拠へ昇格しない。

| exact path | Decision ID |
| --- | --- |
| `docs/UX_DECISION_ACCESSIBILITY_SCALE_2026-08-23.md` | `UX-20260823-ACCESSIBILITY-SCALE-001` |
| `docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md` | `UX-20260823-B2B-CUSTOMER-DELIVERY-001` |
| `docs/UX_DECISION_ERROR_RECOVERY_2026-08-23.md` | `UX-20260823-ERROR-001` |
| `docs/UX_DECISION_FULL_STATE_MATRIX_2026-08-23.md` | `UX-20260823-FULL-STATE-001` |
| `docs/UX_DECISION_GRAPH_LABELS_2026-08-22.md` | `UX-20260822-GRAPH-001` |
| `docs/UX_DECISION_HELP_FOCUS_2026-08-23.md` | `UX-20260823-HELP-FOCUS-001` |
| `docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md` | `UX-20260823-INSTALLER-001` |
| `docs/UX_DECISION_KEYBOARD_FOCUS_2026-08-23.md` | `UX-20260823-KEYBOARD-001` |
| `docs/UX_DECISION_NON_SCROLL_2026-08-22.md` | `UX-20260822-UX-002` |
| `docs/UX_DECISION_RELEASE_SUPPLY_CHAIN_2026-08-23.md` | `UX-20260823-RELEASE-SUPPLY-CHAIN-001` |
| `docs/UX_DECISION_SSH_CONNECTION_2026-08-23.md` | `UX-20260822-SSH-001` |

### 2.2 RC-139..159 typed authority boundary

`RC-139..149`は`REST_API_V1.md`のwire/route ownerと
`DATA_PROTECTION_POLICY.md`のstate/retention ownerを、`DP-REST-001..011`のIDでtyped joinする。
同文節の複製を意味の証拠にせず、health/error/request-boundary、read-only effect、storage identity、
cursor/DB、generation、restore、boot、lineage、combined-loadの各未決値を勝手に固定しない。
`RC-150..159`はB2B/customer-document、release-supply、accessibility、data/restoreのsource ownerへ
joinし、HOLD/OPEN_AUTHORITY_CONFLICT/PRODUCT_PENDINGをdelivery・公開・製品PASSへ昇格しない。
欠落、別release、stale、独立評価未取得は既存last-goodと要求状態を保持する。
`RC-122..129`と`RC-150..159`のcurrent targetはB2B projection companionへexact RC集合として
79件（A-D=0、E-I=33、J-M=46）を登録し、conflict台帳から毎回再展開して集合一致させる。
Decisionの値を79行へ複製せず、`decision_version=b2b-customer-delivery-v1`の§14を解決する。
欠落・extra・重複・別version・generic markerだけの行は伝播済みと認めない。

`RC-164..171`のcurrent targetはlegacy-gap projection companionへexact RC集合として53件
（A-D=10、E-I=11、J-M=32）を登録し、conflict台帳から毎回range展開して集合一致させる。
8件の旧source要件は同書の10-field atomic contractへ1:1 joinし、base 226行の意味を置換せず、
valid設定2再起動、3 client lifecycle、8 thread状態、4 source event、DB fault集合、5 migration割込み、
installer resource failure/resume、2 clean provenance checkpointをcross-row ANDとして追加する。
case欠落、旧PID/画像、別generation、部分成功、generic markerだけでは旧96件をPASSへ昇格しない。

### 2.3 工程ガバナンス原子inventory

工程制約は次の3 IDを`REQUIREMENTS_LEDGER.md`から1:1で参照する。これらは製品226 ID、旧96 ID、
3冊の具体契約、typed product dependency DAGには含めないが、抽出・実装・完了ゲートとはAND結合する。

| GOV ID | conflict join | canonical purpose | current status |
| --- | --- | --- | --- |
| `GOV-THREAD-END` | RC-172 | unresolved/forced turn boundaryをterminal PASSへ丸めず、実eventに基づくcontinuation/reassignmentを追跡 | REQUIREMENTS_PASS / CONTINUOUS_GATE |
| `GOV-NO-INPUT-END` | RC-173 | `WAITING_FOR_INPUT`をnonterminalに固定し、入力待ち終了・synthetic decision・暗黙承認を禁止 | REQUIREMENTS_PASS / CONTINUOUS_GATE |
| `GOV-ESCALATION-100X` | RC-174 | 承認後omissionでapproved N、target式、escalation epoch、fresh再抽出、実装再開禁止を追跡 | REQUIREMENTS_PASS / CONTINUOUS_GATE |

現時点は`product_id_set=226`、`governance_contract_id_count=3`である。承認後omissionが発見された場合だけ
`governance_work_unit_target`を、N=226なら226000、N>226ならmax(226000,N×100)として発火させる。
発火後はtarget単位を一意な原子要求へ実際に展開し、重複・synthetic ID・旧PASSで件数を埋めない。
platform/API turn終了後のlivenessは主張せず、raw continuation eventがない状態はHOLD/INCONCLUSIVEのまま保持する。

## 3. 件数とID集合

| 正本 | 必須ID | 件数 |
| --- | --- | ---: |
| A–D具体契約 | A001..020、B001..024、C001..020、D001..012 | 76 |
| E–I具体契約 | E001..016、F001..012、G001..016、H001..012、I001..016 | 72 |
| J–M具体契約 | J001..016、K001..016、L001..016、M001..030 | 78 |
| 現行要求合計 | 上記の和集合 | 226 |
| 旧要求crosswalk | DP/LIVE 11、AUD 27、Windows旧要求/REG 40、TG 18 | 96 |

226件と96件は別母集団であり、足して新しい322件の製品要求へ水増ししない。96件は、
226件またはWindows外に残る同名 `GLOBAL:*` 正本へ意味を失わず逆引きする。

## 4. 1行の抽出完了条件

各 `WIN-*` 行は、具体契約とトレーサビリティ資料を合わせて次をすべて持つ。

- 一意ID、actor、entry、precondition、具体入力、action、完全に観測可能な期待値
- 型、単位、範囲、null、順序、重複、時刻、丸め、表示所有者
- invalid、失敗、部分結果、last-good、復旧、再入、終了時の保持条件
- 該当する永続化、秘密情報、通信上限、周期、競合、負荷の境界
- 実在する依存IDと、依存が必要な理由
- `depends_on` は `hard_prerequisite=<comma-separated WIN IDs or —>; related_validation_join=<comma-separated WIN IDs or —>` を厳格に使い、既存参照を削除せず hard 409 / related 154 / total 563 とする。hardのみをcycle/layer検査し、relatedは非遮断joinだが不一致時はconsumer/targetの両行をFAILとする。
- B2B対象79行とlegacy-gap対象53行は、各companionの同一ID/RC集合を追加入力にし、conflict rangeから再計算した集合と完全一致する
- fixtureと製品固定値の区別
- 実装後に取得する証拠種別、artifact固有SHA、独立oracle、三値判定式
- 非該当領域は空欄にせず、なぜ非該当かを明記

「同じ」「適切」「安全」「通常」「問題なし」「satisfies」だけの期待値、要求文の言い直し、
根拠のない固定値、存在しないwire field、自己依存はFAILである。

各具体契約の `independent_oracle` は、同一IDのtraceability matrix行およびfreeze/release manifest
契約と機械的にjoinする。証拠種別はtraceability行、要求freeze SHAは要求freeze manifest、製品artifact
SHAは後段release manifestが所有し、具体契約の作業途中SHAへ固定しない。抽出段階のPASSは
「行固有のpositive/negative oracle、証拠種別、将来のartifact join、三値式が完全」である場合だけで、
実artifact未取得を捏造PASSにしない。製品段階のPASSは同一release artifact SHAでpositiveとnegativeを
両方観測した場合だけ、矛盾はFAIL、証拠欠落・stale・別release混在はINCONCLUSIVEとする。

### 4.1 依存型と補助台帳の抽出条件

依存セルは226行すべてに2キーを持ち、参照なしでも `—` を両キーへ明示する。targetはbaseline
の非空・一意titleへjoinし、unknown/self/type-duplicate=0を機械確認する。layerは
`I/J → B/D → A/C/E/F/G/H/K/M → L` とし、hard edgeのproducer layerがconsumer layerを
逆行しない。hard graphはcycle-0（SCC=0）、related graphの相互joinは意味突合として保持する。

補助11列の行固有semanticsは、現行3冊の具体契約と同一IDへjoinする。旧
`WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md` とNormative Override V1..V14はhistory-onlyで、
現行要求・acceptance oracle・overrideの必須正本ではない。旧文書の文言で現行3冊または補助11列を
上書きせず、現行3冊の3 path markerと226 ID集合を正本として突合する。

## 5. 抽出ゲート

次のANDが成立した場合だけ、別変更で各正本文書を同時に `EXTRACTION_COMPLETE` へ更新できる。

1. baselineと3具体契約のID集合が完全一致し、合計226、一意226、欠落/余剰0
2. 各具体契約が10列で空欄0、depends_onが厳格parseされ、hard=409 / related=154 / total=563、依存は実在・非自己参照・型重複0、hard SCC=0・backward=0
3. §4の意味条件を全226行について独立監査し、FAIL/OPEN/INCONCLUSIVEが0
4. 旧96 source IDが完全一致し、target欠落0、存在しないtarget 0、意味欠落0。RC-164..171の8 source/53 current target projection、8件の10-field cross-row契約、3 concrete companion joinも完全一致
5. 固定値正本との矛盾0、未記録supersession 0、`OPEN_AUTHORITY_CONFLICT` 0
6. 要求抽出専用の機械ゲートがPASS
7. 実装担当と異なるfresh evaluatorが最新版だけを読み、上記1〜6をPASS

製品証拠は抽出後に取得するため、抽出監査は「証拠の実物」と「証拠計画」を混同しない。
ただし証拠計画またはoracleが未確定なら抽出FAILである。製品実装のPASSは、後段で同一release
lineageに結び付いた実物証拠が揃うまで `PRODUCT_INCONCLUSIVE` のままにする。

## 6. 抽出後の製品受入ゲート

抽出完了後も、実装、Linux/X、REST、daemon/DB、Windows client、self-contained installer、
Start Menu、更新/削除、全locale、全状態、全surface、DPI/複数monitor、非スクロール、入力非奪取、
fresh画像、物理Windows host、顧客資料、独立B2B監査の全ゲートを同じsource release manifestへ
結合する。X/Linux binary、Windows payload、installerはartifact別SHAを持ち、異なるbinaryへ同じSHAを
要求しない。

一つでもFAIL/HOLD/INCONCLUSIVE/未確認があれば、製品を完成扱いにしない。

## 7. 現在の閉鎖状態

- 226 ID構造: 作成済み、意味監査中
- 96 ID crosswalk構造: 作成済み、直近独立監査FAILの修正後再監査待ち
- 値の正本: 作成済み、具体契約との矛盾監査中
- 独立要求抽出監査: 未PASS
- 製品変更凍結: 継続

したがって現時点は `EXTRACTION_INCOMPLETE` であり、実装へ移らない。

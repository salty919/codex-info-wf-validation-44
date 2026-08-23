# Windows 226要求 トレーサビリティ設計（実装凍結中）

状態: `IN_PROGRESS / HOLD`

現行226 IDの集合は `WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`、意味契約は
`atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md`、
`atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md`、
`atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md` を正本とする。
B2B対象79行はさらに
`atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md`の同一ID/RC集合へtyped joinし、
Decision値の複製ではなく`b2b-customer-delivery-v1`を解決する。
旧要求cross-row対象53行はさらに
`atomic-contracts/WINDOWS_LEGACY_GAP_PROJECTIONS_2026-08-23.md`の同一ID/RC集合へtyped joinし、
RC-164..171の8件の10-field契約を旧source 8件・base行・evidenceへ1:1で解決する。
`WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md` と
`WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`（11列台帳）は実装対象・証拠計画を補う索引であり、
具体契約の欠落や矛盾を補完したことにはしない。本書は列・責務・ゲートの設計規則であり、行単位契約の代替にはしない。

この文書は、`WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md` の226原子要求へ、
責務・対象・試験・証拠・独立評価を割り当てるための設計である。カテゴリ既定値を継承できる
のは、行固有の例外がなく、受入式がその行の観測値へ具体化されている場合だけである。
継承しただけでverifiedにはしない。

## 1. 全行必須レコード

各 `WIN-X-NNN` は、具体契約の次の10列をすべて持つ。これが意味抽出の判定単位である。

| 列 | 内容 |
| --- | --- |
| ID | baselineと完全一致する一意ID |
| actor | 操作・監視・保存を行う主体 |
| entry | 開始イベントと入口 |
| precondition | 接続、認証、設定、期間、locale、DPI、モニタ、権限 |
| input | `fixture_only:`で始まる具体入力・境界値。製品固定値は正本を併記 |
| action | 操作順、再入、失敗注入、観測点 |
| exact_expected | field、型、単位、順序、表示所有者を含む完全な期待値 |
| negative_retention | 否定条件、失敗分類、last-good、部分結果、復旧・永続化境界 |
| depends_on | `hard_prerequisite=<WIN IDs or —>; related_validation_join=<WIN IDs or —>` の厳格な2型。各targetはbaselineの一意で空でない要求titleへedge reason joinする |
| independent_oracle | 実装者以外が三値判定できるexact oracle |

さらに各IDは次の11列を補助台帳として持つ。これは実装対象と製品証拠計画を補うが、
上記10列の意味契約を置換しない。補助台帳の行固有値は
`WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md` に置く。

| 列 | 内容 |
| --- | --- |
| requirement_id | 一意ID、旧ID、分割/統合履歴 |
| actor_entry | 利用者、Windows、Linux API、SSH、daemon、DBと開始条件 |
| precondition | 接続、認証、設定、期間、locale、DPI、モニタ、権限 |
| observable | 画面/API/ファイル/プロセスで観測可能な結果 |
| data_visual_contract | 型、単位、範囲、時刻、軸、線、色、余白、文字、所有者 |
| failure_persistence_contract | 失敗分類、last-good、部分結果、再試行、保存、削除、復旧 |
| security_performance_contract | loopback/SSH、秘密情報、サイズ、権限、周期、CPU上限 |
| implementation_target | 所有ファイル、責務境界、変更禁止範囲 |
| test_oracle | fixture、入力、期待値、境界、否定条件、再現手順 |
| evidence | rawログ、fresh画像、プロセス、DB/hash、artifact SHA |
| independent_reviewer | 実装者と別の担当、判定、判定根拠、判定時刻 |

具体10列または補助11列の空欄、推測、`適切`、`正常`、`同じ`だけの記載は未抽出とする。

`independent_oracle` は同一IDで補助台帳とmanifestへjoinし、製品証拠JSONはexact key
`requirement_id,evidence_type,artifact_reference,artifact_sha256,source_freeze_sha256,positive_observations,negative_observations,verdict,reviewer,captured_at_utc`
を持つ。抽出中の `artifact_reference` は後段release manifestの同一ID slotを指す計画であり、
`artifact_sha256`を作業中ファイルのhashやplaceholderでPASSさせない。製品PASSでは64-hex artifact SHAと
source freeze SHA、時刻、独立reviewerを必須にし、positive/negativeの全期待一致をPASS、観測矛盾をFAIL、
欠落・別SHA・staleをINCONCLUSIVEとする。抽出監査はこのschema/行固有oracleの完全性を判定し、
実製品証拠がまだないことだけを抽出FAILへ読み替えない。

## 2. カテゴリ責務マップ

| 範囲 | 主担当 | 実装対象の境界 | 試験根拠 | 実機/raw証拠 | 独立評価 |
| --- | --- | --- | --- | --- | --- |
| WIN-A | Windows parity owner | Main/Setup/Settings/Graph/Threads/Legal; Help=Main内 surface inventory | native→Windows feature matrix | 全画面fresh画像、操作ログ | parity reviewer |
| WIN-B | Graph semantics owner | graph period/path/axis/series transformation | same-fixture deterministic oracle | X/Windows path列、fresh graph画像 | graph reviewer |
| WIN-C | Main state owner | Main view model/status/layout | state transition matrix | 状態別fresh画像 | UI reviewer |
| WIN-D | Thread/legal owner | active snapshot projection、Legal notices | live-state matrix、notice inventory | Threads/Legal raw+画像 | live-state reviewer |
| WIN-E | Setup/transport UX owner | Setup、SSH、auth command boundary | success/failure/auth matrix | process trace、setup画像 | security/UX reviewer |
| WIN-F | Settings/persistence owner | settings JSON、restart/recovery | malformed/atomic/restart matrix | file/hash、restart trace | persistence reviewer |
| WIN-G | Localization/accessibility owner | catalogs、timezone、numeric、AutomationProperties | locale/DPI/keyboard matrix | locale画像、focus/keyboardログ | accessibility reviewer |
| WIN-H | Installer owner | publish/payload/shortcut/registry/uninstall | isolated install/rollback matrix | Windows host log、SHA、registry | installer reviewer |
| WIN-I | API/security owner | REST schema/client limits/redaction | malformed/oversize/secret matrix | endpoint trace、redacted log | security reviewer |
| WIN-J | Data protection owner | history/SQLite/daemon/backup/migration | concurrent writer、backup、migration matrix | DB SHA/quick_check、daemon log | data reviewer |
| WIN-K | Boundary/lifecycle owner | abnormal input、DPI/multi-monitor、window lifecycle | boundary/concurrency matrix | isolated process geometry/log | lifecycle reviewer |
| WIN-L | Release evidence owner | traceability、freshness、independence、release guard | same-SHA release matrix | manifest、raw evidence、audit | B2B reviewer |
| WIN-M | UX governance owner | purpose、navigation、non-scroll、design decisions | UX matrix、decision records | all-view fresh images、keyboard log | UX reviewer |

### 2.1 責務の分離境界

- WIN-D は live-thread projection と Legal notice を同一行へ混在させない。スレッド行は受理済み完全snapshotのcanonical fingerprint、stale判定、active row集合、last-goodを所有する。REST wireに存在しない`epoch`やthread `status`を発明しない。Legal行はnotice inventory、表示所有者、artifact manifestとの版・hash関係を所有する。
- WIN-K は abnormal input、window lifecycle、DPI/multi-monitor を別の行契約として扱う。entry/re-entry、cancel、close、singleton、異常終了、再開の条件を行固有の observable とする。
- WIN-M は UX目的・導線・非スクロール・視覚設計を分ける。非スクロール行には viewport、overflow、clip、focus、DPI、状態別の受入セルを必須とし、実装の `ScrollViewer` 存在だけではPASSにしない。
- Legal の表示所有者は WIN-D-Legal、導線・ナビゲーションの所有者は WIN-M とし、同じ文言を両方へ追加しない。

## 3. 依存関係

依存セルは次の構文以外を受け付けない。各リストはカンマ区切りのbaseline IDであり、参照なしは
`—` とする。現行3冊の226行を再計数した結果、
`hard_prerequisite=412`、`related_validation_join=165`、総参照 `577` となる。

```text
hard_prerequisite=<WIN-X-NNN[,WIN-X-NNN...] or —>; related_validation_join=<WIN-X-NNN[,WIN-X-NNN...] or —>
```

依存の向きは `consumer row -> target producer` である。hard edgeではtarget producerの
具体契約・oracleがconsumerの実行前提になり、producerがFAIL/HOLD/INCONCLUSIVE/未確定なら
consumerをverifiedへ昇格させない。related edgeは実行を遮断せず、相互の意味・観測・保持を
同一fixtureで突合するvalidation joinである。ただしjoinの不一致、target titleの欠落、型違いが
あればconsumer行とtarget行をともにFAILとし、片側だけのPASSへ丸めない。hardだけが閉路・layer
順序・backward edgeのDAG検査対象で、relatedの相互参照やSCCは遮断条件ではない。

### 3.1 cycle-0 hard layer

```text
layer 0: WIN-I / WIN-J  (transport, schema, data boundary)
layer 1: WIN-B / WIN-D  (graph and live/thread semantics)
layer 2: WIN-A / WIN-C / WIN-E / WIN-F / WIN-G / WIN-H / WIN-K / WIN-M
layer 3: WIN-L          (same-SHA evidence and independent release gate)
```

hard edge `consumer -> producer` は `layer(producer) <= layer(consumer)` を満たし、
layerを逆行するedgeは0本でなければならない。同一layer内の同一責務依存は許可するが、
hard graphの非自明SCCは0（cycle-0）とする。現在の型付け後は hard SCC=0、hard backward=0、
unknown/self/type-duplicate=0 である。

### 3.2 producer/consumer と edge reason

各edge reasonは依存セルへ自由記述を追加せず、target IDを
`WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md` の同じID行へ機械joinする。
targetの要求title（baselineの要求列）は空でなく226件で一意でなければならない。従って
`target ID + baseline unique title` がhard/related共通のreason identityとなり、具体契約側の
producer（target）とconsumer（依存元）を同じtitleへ結合できる。baseline targetがunknown、
self、重複、空、または型リスト内に同じIDが二重に現れる場合は抽出FAILである。

`WIN-A-001` と `WIN-A-014` の依存なし行も、
`hard_prerequisite=—; related_validation_join=—` と両型を明示する。

## 4. 行固有の具体化規則

カテゴリ既定値だけでは「表示する」「提供する」「維持する」を受入可能な要求にできない。
各行は次の変換規則で具体化する。

| 対象 | 必須の具体化 |
| --- | --- |
| `表示する` | 正本field、null/invalid時の文字、単位、桁、locale、表示所有者、重複禁止、0/中間/最大の期待値 |
| `提供する` | 操作名、入口、キーボード操作、disabled/busy/re-entry、成功後状態、失敗後状態、戻る/閉じる |
| `維持する` | 入力世代、比較する前後値、許される変化、欠測・再起動・認証切替・更新時の保持境界 |
| `同じ`/`同等` | 同一fixture、正本列、許容差、順序、軸、色の意味、文字列差分許容、差分禁止面 |
| `安全に` | 攻撃/破損入力、上限、拒否動作、last-good、秘密情報の非表示、ログredaction |
| `可能にする` | 前提、利用者操作、完了条件、キャンセル、途中失敗、再実行、永続化の有無 |
| `自動` | 起動トリガー、周期/one-shot、singleton、CPU/メモリ上限、停止、異常終了、再開、gapの扱い |
| `対応する` | 対応範囲の列挙、未対応時のfallback、機能/データ/表示/導入/運用の各面、証拠種別 |
| `UI/デザイン` | viewport、余白、整列、文字階層、色、アイコン、focus/hover/disabled、DPI、非スクロール条件 |
| `確認する` | fresh process、artifact SHA、raw log、画像サイズ、独立評価者、PASS/FAIL/INCONCLUSIVE式 |

この変換後も一つの行に2つ以上の異なる責務が残る場合は、行を分割し、旧IDとの対応を
記録する。行数を226件に合わせるための無根拠な統合は禁止する。

### 4.1 補助ドメイン台帳と旧atomic資料

11列の補助レコードだけでは状態直積を行固有化できない領域は、同じIDを持つ拡張台帳で実装・証拠計画を補う。
WIN-D/K/Mは `WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md`、WIN-I/Jは
`WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md` を参照する。拡張台帳は補助列の
precondition、observable、failure/persistence、security/performance、test_oracle、evidenceを具体化し、
entry→re-entry→cancel→close→singleton→abnormal termination→restart/resume、否定条件、依存IDを含める。
具体契約と拡張行のID・意味が一致しない、または必要な拡張行が未作成の場合は抽出FAILである。

`WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md` とNormative Override V1..V14は履歴資料である。
それらの文言を具体契約の代替oracleとして使わず、矛盾時に旧資料を優先しない。

## 5. 証拠の同一性規則

- 要求抽出文書の実SHAは `WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md` に従い、
  抽出freeze時に一括取得する。作業途中のmutable file SHAを行契約へ固定せず、entry変更後は
  manifest全体を失効・再生成する。
- source commitを一つのrelease manifestへ固定し、X/Linux binary、Windows client payload、
  installer、installed executableにはplatform/artifact別SHAを記録する。異なるbinaryへ同一SHAを
  要求せず、各SHAが同じsource commit・release manifest・fixture/data hashから生成された関係を
  manifestで連結する。同一artifactを検査する画像、rawログ、host trace、独立監査は、そのartifact
  固有SHAと完全一致させる。
- 古い画像、別SHAのテスト、静的コードだけの実機要求、実装者自身のPASS宣言は代替にならない。
- 画像が会話添付で出所・SHA不明の場合は参考資料に限定し、canonical acceptance evidenceにしない。
- 実マウスを動かす証拠は通常受入へ使わず、静的API不使用と隔離OS automation/message入力を別々に記録する。

## 6. 抽出ゲート

226行すべてに具体10列と補助11列が埋まり、D/K/M 58行・I/J 32行の補助ドメイン台帳が登録され、
B2B 79行とlegacy-gap 53行のprojectionがconflict台帳からの再展開集合に完全一致し、
legacy-gap 8 atomic契約が旧source 8件へ1:1 joinされ、
新depends_on構文が226行で厳格parseされ、hard=412/related=165/total=577、hard-onlyの
cycle=0・backward=0、unknown/self/type-duplicate=0、全target titleの非空・一意joinが成立し、
仕様未解決が0、製品証拠前提がfixture契約へ変換され、独立抽出突合がPASSになるまで、状態を
`EXTRACTION_INCOMPLETE` のまま保持する。製品証拠未取得を抽出契約PASSへ丸めず、抽出後の製品受入ゲートへ引き渡す。
この設計文書だけでは抽出完了、実装完了、受入PASSを意味しない。

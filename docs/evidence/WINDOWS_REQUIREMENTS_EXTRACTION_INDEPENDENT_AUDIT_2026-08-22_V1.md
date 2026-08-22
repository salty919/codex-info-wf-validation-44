# Windows requirements extraction independent audit V1

監査日: 2026-08-22
監査担当: Luna 独立評価
対象: 要求抽出文書と関連する文書契約のみ。コード、テスト、ビルド、インストール、画面評価は実施していない。

この文書の本文は文書修正前の監査スナップショットであり、現在の正本判定ではない。修正後の追補を
末尾に追加している。現在の正本は `WINDOWS_REQUIREMENTS_EXTRACTION_AUDIT_2026-08-22.md`、
`WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`、および追補後の独立監査である。

## 総合判定

**FAIL / INCONCLUSIVE / HOLD**

- 文書整合性は FAIL。非スクロール正本と DESIGN の scroll 許容記述が未解決で、要求抽出表にも6件の明示的 `status: FAIL` がある。また U-03 の分類が表と後段説明で矛盾する。
- 証拠閉鎖は INCONCLUSIVE。226行の要求ID・構造は確認できるが、U-01〜U-05 は全て未閉鎖で、独立突合と同一fixture/raw証拠が未取得と記録されている。
- スレッド終了禁止・入力待ち終了禁止の方針文書は相互に整合し、HOLD/INCONCLUSIVE のまま継続する契約を明記している。

## 226行の行単位監査

対象 `docs/evidence/WINDOWS_REQUIREMENTS_EXTRACTION_AUDIT_2026-08-22.md` は物理ファイルとして430行だが、`WIN-A..M` の原子要求行は226件である。

| 検査 | 結果 |
|---|---|
| 原子要求行数 | 226 |
| ID重複 | 0件 |
| 範囲 | WIN-A-001..020、B-001..024、C-001..020、D-001..012、E-001..016、F-001..012、G-001..016、H-001..012、I-001..016、J-001..016、K-001..016、L-001..016、M-001..030 と分類表の合計226が一致 |
| 上位表セルの空欄/NF異常 | 0件 |
| grouped field markers（data/failure/persistence/security/performance/status）欠落 | 0件 |
| `WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md` の要約行数 | 226 |

上位の抽出表は各要求行に7つの表セルを持ち、契約グループを本文内に埋めているため、文字列構造としては欠落なし。ただし `WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md` が要求する11列の独立レコードに対し、別ファイルの row register は要約列のみである。要約表を11列台帳の代替とするか、詳細表を正本とするかの明示がなく、行固有の責務を機械的に追跡できることは文書だけでは確定しない。これは必須フィールドの形式面で INCONCLUSIVE とする。

## 明示的FAIL行

次の6行は、非スクロール契約と既存構造の衝突を明示的に FAIL と記録している。

- `WIN-M-003`: Main の主要値・状態・更新操作と `MainWindow.axaml:178` の ScrollViewer。
- `WIN-M-004`: Setup の同一viewport要求と `SetupWindow.axaml:32` の ScrollViewer。
- `WIN-M-005`: Settings の同一viewport要求と `SettingsWindow.axaml:24` の ScrollViewer。
- `WIN-M-007`: Threads の主要比較対象/更新/閉じる操作と `ThreadsWindow.axaml:57` の ScrollViewer。
- `WIN-M-008`: Legal の常時戻る導線と `LegalNoticesWindow.axaml:57` の ScrollViewer。
- `WIN-M-009`: 全Windowのページ全体非スクロール要求と複数WindowのScrollViewer、および `DESIGN.md:137,143,145-146` の許容記述。

抽出表の状態分布は FAIL 6件、INCONCLUSIVE 220件、PASS 0件である。FAIL/INCONCLUSIVEをPASSへ丸めていない点は正しいが、`EXTRACTION_COMPLETE` へ移行できる状態ではない。

## U-01〜U-05 証拠前提

`docs/evidence/WINDOWS_REQUIREMENTS_EXTRACTION_AUDIT_2026-08-22.md:382-395` に5項目が列挙され、現在はいずれも未閉鎖である。

| ID | 独立に必要な前提 | 現在の状態 |
|---|---|---|
| U-01 | X/Windows帰属、binary/artifact SHA、fixture/reset/timezone/metric/toggle manifest、capture/process記録 | 未閉鎖、画像名だけでは不十分 |
| U-02 | 同一3071点のcanonical raw、入力SHA、X/W normalized points、座標/geometry、境界セル | 未閉鎖 |
| U-03 | Remaining独立0–100軸のraw transform・axis tick・同一sample比較 | 同一fixture証拠未取得 |
| U-04 | locale/timezone label inventory、X/W文字列、clip/overlap、mappingと同一SHA証拠 | 同一SHA raw label/fresh/独立証拠未取得 |
| U-05 | cursor write なしのdefault gate、SKIP/INCONCLUSIVEログ、隔離VMでのopt-in drag trace | opt-in実測未実施 |

U-03は表（同文書386行）で `分類: 仕様曖昧` とされる一方、同文書392-395行では Decision Record により仕様曖昧は0件で、残りは証拠前提だけと説明される。baseline の U-03 も「方向確定/証拠前提」とするため、この分類は文書内の明示的矛盾であり FAIL とする。U-01/U-02/U-04/U-05は証拠未取得による INCONCLUSIVE とする。

## UX非スクロール整合性

`docs/WINDOWS_UX_SPEC.md:48-65,207-227` は Main、Setup、Settings、Graph、Threads、Legal の主要操作・情報をページスクロールなしで到達可能にし、内部ScrollViewerだけの解決をFAILとする。一方、同文書224-227行は `DESIGN.md` にThreads/Legal等の縦スクロール許容記述があると明示しており、正本選択と具体viewport設計が未決定である。

したがって、非スクロール原則は定義自体は明確だが、既存設計との矛盾が未解決である。M-003/M-004/M-005/M-007/M-008/M-009のFAIL記録は要求抽出文書として整合する。M-006等のグラフ固有・証拠依存行は別途INCONCLUSIVEであり、ここでPASSにはしない。

## スレッド終了禁止・入力待ち禁止

文書整合は PASS 相当（ただし全体状態はHOLD）。

- `REQUIREMENTS_INTAKE_POLICY.md:11-17` は抽出/検証完了まで主・監視・サブエージェントの自主終了を禁止し、タイムアウト等はHOLD/INCONCLUSIVEで再配置する。
- 同文書:15-21は、未完了状態でユーザーへの入力・確認・判断を返答待ちの終了条件にすることを禁止し、既存資料からの抽出・矛盾整理・証拠化を継続する。
- `COMPLETION_PROTOCOL.md:5-20` は同じ禁止を繰り返し、チャット基盤による停止も完了/PASSと扱わず、次回は未完了台帳から再開する。
- `AGENT_REQUIREMENTS_TRACKER.md:7` は `GOV-THREAD-END` と `GOV-NO-INPUT-END` を226件抽出作業の受入対象に含め、状態を `IN_PROGRESS / HOLD` と記録している。

確認範囲では、これらの禁止に反して「ユーザー入力を待って終了」「HOLDのまま完了」とする許可条項は見つからなかった。固定完了文言の記載は全要求PASS時だけという条件付きで、未完了時の終了許可ではない。

## 結論と最小修正課題

要求抽出は226行の件数・ID・上位フィールド構造まで進んでいるが、文書正本に明示的FAILがあり、U証拠前提も未閉鎖である。最小修正課題は次の通り。

1. U-03の分類を「証拠前提」へ統一し、Decision Recordで解消済みの仕様曖昧と未取得証拠を分離する。
2. 11列詳細台帳と7列要約row registerの正本/投影関係を明記し、226行の各列を行固有に追跡可能にする。
3. DESIGNのscroll許容とUX_SPECの非スクロール原則について正本を一つに決め、MのFAIL行を再抽出・再評価する。
4. U-01〜U-05の各必要証拠と独立突合結果を埋めるまで `EXTRACTION_INCOMPLETE / HOLD` を維持する。

今回、コード、テスト、ビルド、インストール、画像評価、生成物は変更・実行していない。

## 追補（2026-08-22、文書修正後）

- U-03の分類は `仕様決定済み・証拠前提` に統一した。Remaining独立軸の方向は `WINDOWS_UX_SPEC.md:140-146`、label差分は `UX-20260822-GRAPH-001` が正本であり、仕様曖昧は0件として扱う。ただしU-01〜U-05の同一SHA/raw/fresh/独立証拠は未取得のため、抽出ゲートは未閉鎖である。
- 11列の行固有契約を `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md` に追加し、226行すべてへID、actor/entry、precondition、observable、data/visual、failure/persistence、security/performance、implementation、oracle、evidence、independent reviewerを割り当てた。全行の判定は証拠未取得 `INCONCLUSIVE` のままなので、これはPASSへの変更ではない。
- 責務分離境界（WIN-Dのlive/Legal、WIN-Kの異常/ライフサイクル/DPI、WIN-Mの目的/導線/非スクロール/視覚）を `WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md` に追記した。
- DESIGN.mdのscroll許容とUX_SPECの非スクロール要求は、`UX_DECISION_NON_SCROLL_2026-08-22.md` を正本とする設計決定を記録したが、現行実装の解消と実画像・keyboard/DPI証拠は未実施であるため、M-003等のFAIL/HOLDは維持する。

追補後の総合判定も **EXTRACTION_INCOMPLETE / HOLD** であり、コード・テスト・ビルド・インストール・画面評価は開始していない。

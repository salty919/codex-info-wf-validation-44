# 要求抽出ゲート独立監査 V3（2026-08-22）

監査担当: Luna 独立評価

## 監査範囲と判定

対象は、親エージェントから指定された要求抽出文書と、その文書が参照する要求段階の
fixture/schema/oracle 契約だけである。対象は次のファイルに限定した。

- `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md`
- `docs/WINDOWS_UX_SPEC.md`
- `docs/UX_DECISION_NON_SCROLL_2026-08-22.md`
- `docs/evidence/GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md`
- `docs/evidence/UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md`
- `docs/evidence/ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md`

旧監査本文は historical snapshot として判定根拠にせず、上記の最新文書と今回の read-only
突合結果を優先した。コード、テスト、ビルド、インストール、X Window、画面キャプチャは
実行・閲覧していない。

総合判定: **INCONCLUSIVE（要求文書の構造は確認できたが、製品証拠と旧記述の整合は未閉鎖）**。

この監査は `EXTRACTION_COMPLETE`、製品受入 `PASS`、または完了状態へ変更する根拠ではない。
対象文書に記録された `EXTRACTION_INCOMPLETE` / `HOLD` を維持する。

## 確認結果

### 1. 226行と11列

構造確認は次のとおり。

| 対象 | 結果 | 根拠 |
| --- | --- | --- |
| baseline | **PASS（構造のみ）**: 要求行226 | `WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md` の WIN-A..M 行を `awk` で数えた結果 `226` |
| row register | **PASS（構造のみ）**: 要約行226 | `WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md` の `WIN-[A-M]-NNN` 行を数えた結果 `226`。状態は全行 `INCOMPLETE` |
| row contracts | **PASS（構造のみ）**: 行226、各行11列、空欄0 | `WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md` を pipe 区切りで検査し `rows=226`、`pipe_columns=11 rows=226`、列数不一致0、空欄0 |
| ID集合 | **PASS（構造のみ）** | baseline / register / contracts の表行を比較し、差分0 |

列が埋まっていることは証明したが、製品が各契約を満たすことは証明していない。row
contracts の各 `independent_reviewer` は `INCONCLUSIVE (証拠未取得)` のままである。

### 2. U-01〜U-05

契約定義と製品証拠を分離できている点は **PASS（契約文書の確認範囲）**。

- baseline §3（行397-413）は U-01〜U-05 をすべて「契約定義済み／製品証拠待ち」と記録する。
- `GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md` は U-01〜U-04 の入力、3071点、正規化、
  同一SHA、raw/fresh出力、未取得時 `INCONCLUSIVE` を定義する。
- `UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md` は U-04 の label mapping と U-05 の
  cursor write=0、static scan、隔離試験、未実施をPASSにしない条件を定義する。
- `ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md` は U-01 と全要求を同一
  `artifact_sha256` へ連結する必須列を定義する。

ただし、同一SHAの実プロセス、raw、fresh画像、manifest、独立判定は取得されていない。
したがって製品受入については U-01〜U-05 全件を **INCONCLUSIVE（製品証拠待ち）** とする。
この未取得を抽出段階の契約未定義や PASS に置き換えていない。

### 3. UX正本と DESIGN の旧記述

`UX_DECISION_NON_SCROLL_2026-08-22.md` の Decision ID `UX-20260822-UX-002` は、Main、
Setup、Settings、Graph、Threads、Legal で主要情報・主要操作・戻る・閉じるをページ全体の
スクロールなしで到達可能にする正本決定を記録している（同文書行7-11、22-37）。
`WINDOWS_UX_SPEC.md` §8（行222-229）もこの Decision を正本とし、ページング、章切替、
選択詳細、固定viewportを採用することを明記する。

一方、同 §8 は `DESIGN.md` に旧来の ScrollViewer / 縦スクロール許容記述が残り、実装時に
Decision へ整合させる対象であり、コードを変更せず解消済みとは扱わないと明記する。
よって正本の選択自体は **PASS**、旧記述の実際の整合・実装・fresh画面証拠は
**INCONCLUSIVE（未閉鎖）**。旧記述を理由に別の正本を推測していない。

### 4. WIN-D / WIN-K / WIN-M の責務境界

`WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md` §2（行31-54）は次を明示する。

- WIN-D は active snapshot / epoch / stale / `Failed rows=0` / last-good と Legal notice
  inventory / version / owner を分離する。
- WIN-K は abnormal input、window lifecycle、DPI/multi-monitor を別の行契約にする。
- WIN-M は UX目的、導線、非スクロール、視覚設計を分け、viewport、overflow、clip、focus、
  DPI、状態別セルを要求する。
- Legal の表示所有者は WIN-D、導線・navigation の所有者は WIN-M とし、同じ文言を二重に
  所有しない。

row contracts のカテゴリ付録（同文書行244-259）と依存DAG（行261-275）もこの境界を
  繰り返しているため、境界定義は **PASS（文書構造の確認範囲）**。実装上の責務分離は
  本監査範囲外で未検証である。

### 5. ライフサイクル条件

baseline §1.2（行67-78）は戻る、再入、二重クリック、更新中、子画面二重起動、閉じる、
再表示、更新、アンインストールを抽出対象に含める。row contracts は抽出ゲート（行238-242）
で `entry/re-entry/cancel/close/singleton/再開` を全該当行へ割り当てることを要求し、
カテゴリ付録（行258）は異常終了、再開、700x480、高DPI、複数モニタ、cursor API write=0
まで列挙する。

さらに K-010（購読解除）、K-011（child singleton）、K-012（main close時のchild終了）、
K-013〜K-015（monitor/DPI/drag）および K-016（ユーザーのマウスを動かさない）は、各行に
固有の `observable`、fixture、oracle、raw evidence、独立判定欄を持つ。
したがって、行契約／付録へのライフサイクル条件の記載は **PASS（契約構造の確認範囲）**。
leak、singleton、close、再開、DPI、jitter の実測は未取得で、各行の製品判定は
**INCONCLUSIVE** のままである。

### 6. 抽出状態とPASS丸め防止

baseline 行10、row register 行3、row contracts 行3 は `EXTRACTION_INCOMPLETE` を保持する。
row register の226行は `INCOMPLETE`、row contracts の226行は `INCONCLUSIVE (証拠未取得)`。
baseline §3（行402-403）と §4（行430-434）は、U契約と独立抽出突合が閉じるまで
`EXTRACTION_COMPLETE` に変更しないと明記する。したがって、抽出状態を PASS / verified へ
丸めていないことは **PASS**。

## 未達（重要度順）

1. **INCONCLUSIVE — U-01〜U-05 製品証拠未取得。** 同一artifact SHAへ結び付いた実プロセス、
   raw、fresh画像、manifest、独立再計算がなく、製品受入を閉じられない。要求段階の契約定義
   済みとは別の未達である。
2. **INCONCLUSIVE — DESIGN 旧記述の整合と実装受入未確認。** UX正本は決定済みだが、旧来の
   ScrollViewer / 縦スクロール記述は残存し、Decisionへ整合したコード・新画像・keyboard/DPI
   証拠がない。旧記述を解消済み、または現行UIが非スクロールPASSとは判定しない。
3. **INCONCLUSIVE — 226行の製品独立判定未取得。** row contracts の独立判定は全行が
   `INCONCLUSIVE (証拠未取得)`、row register は全行 `INCOMPLETE`。構造PASSを要求PASSへ
   すり替えない。

今回の限定文書突合で新たな明示的矛盾を確認した行はないため、FAIL件数は0件とする。ただし
上記INCONCLUSIVEが残るため、総合判定をPASSへ変更しない。

## 実施していないこと

- ソースコード変更、テスト、lint、型検査、ビルド、インストールは未実施。
- X Window、Windows実プロセス、DB、daemon、SSH、画面キャプチャ、目視レビューは未実施。
- 監査対象外の README、template、AGENTS、skill、session、生成物は判定に使用していない。

## 最小再開条件

- U-01〜U-05 の同一SHA製品証拠 bundle と独立再計算を揃える。
- `DESIGN.md` の旧記述を Decision `UX-20260822-UX-002` へ整合させ、実装後に全Window×状態×
  サイズ×locale×keyboard の fresh証拠を取得する。
- 226行の各 `independent_reviewer` を raw証拠に基づき再評価し、1行でも FAIL/INCONCLUSIVE が
  ある間は `EXTRACTION_INCOMPLETE / HOLD` とする。


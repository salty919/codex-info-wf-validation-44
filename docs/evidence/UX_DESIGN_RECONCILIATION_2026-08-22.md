# UX文書整合証拠（2026-08-22）

## 判定

- 正本Decision: `UX-20260822-UX-002`
- 正本ファイル: `docs/UX_DECISION_NON_SCROLL_2026-08-22.md`
- 文書整合: `PASS`
- 実装受入: `HOLD`（実装、画面評価、独立評価の証拠未取得）

この記録の対象は、`DESIGN.md` に残っていた旧スクロール導線と、Windows UX仕様・Decisionの
関係だけである。コード、テスト、ビルド、インストール、画面起動はこの作業で実施していない。

## 整合範囲

| 対象 | 旧記述の扱い | 維持する契約 |
| --- | --- | --- |
| `DESIGN.md` 情報所有権表のThreads全件 | 固定Window内の縦スクロールという旧記述を、`UX-20260822-UX-002` へ実装時に整合させる対象として明記 | canonical active snapshot、取得・dedup順、presentation意味論、role、親子関係 |
| `DESIGN.md` レイアウト規則のThreads | 4件目以降のスクロール導線を、ページング・選択詳細・折りたたみ等へ実装時に整合 | 900×480、本文840×440、viewport 384、row 128/384、文字サイズ、tree rail座標 |
| `DESIGN.md` Threads表示件数方針 | 旧ListViewスクロールを実装時整合対象として明記 | 0/1/2〜3件の表示方針、カード寸法、18px級本文、比較優先 |
| `DESIGN.md` Graph期間popup | popup内ListViewの旧スクロールを実装時整合対象として明記 | popupの前面位置、行領域128px＋外枠2px、plot/toggle位置・高さ、選択可能性 |
| `DESIGN.md` Threads受入基準 | `scroll clipping` をviewport端の部分row clippingへ明確化 | 指定x/y、rail、部分rowのcrop、完全可視rowの不変条件 |

旧記述は監査可能な形で残しているが、正本Decisionの規則として採用していない。データ意味論、
情報所有権、既存の寸法・座標・文字安全域・popup geometryを削除または弱める変更は行っていない。

## 文書間の正本関係

- `docs/WINDOWS_UX_SPEC.md` §2.1 は、主要情報・主要操作・戻る/閉じるをページスクロールなしで到達させ、長い一覧・本文をページング、章切替、選択詳細、折りたたみへ分割する規則を定義する。
- `docs/WINDOWS_UX_SPEC.md` §8 は、`DESIGN.md` のThreads/Graph/Legalに残る旧スクロール導線を実装時整合対象とし、データ意味論と既存レイアウト契約を変更根拠にしないこと、実装受入を `EXTRACTION_INCOMPLETE` / `HOLD` とすることを明記する。
- `docs/UX_DECISION_NON_SCROLL_2026-08-22.md` は `UX-20260822-UX-002` を正本とし、`DESIGN.md` の旧スクロール導線を実装時整合対象とすること、実装・画面評価・独立評価の証拠未取得により `HOLD` を維持することを明記する。

## 実施した文書検査

1. `rg -n -i 'スクロール|scroll|scrollviewer|viewport' DESIGN.md docs/WINDOWS_UX_SPEC.md docs/UX_DECISION_NON_SCROLL_2026-08-22.md`
   を実行した。残る `scroll` は、正本Decision/specの非スクロール規則、旧記述を明示的に「実装時整合対象」とした注記、またはviewport clippingの幾何契約だけで、採用中の旧スクロール導線はない。
2. `rg -n 'UX-20260822-UX-002|実装時に整合|実装時整合対象|HOLD' DESIGN.md docs/WINDOWS_UX_SPEC.md docs/UX_DECISION_NON_SCROLL_2026-08-22.md`
   を実行した。3文書すべてでDecision参照、実装時整合対象、HOLD状態を突合できる。
3. 実装受入ゲート（cargo、Windows実行、画面キャプチャ、独立評価）は未実行であり、HOLDを変更していない。


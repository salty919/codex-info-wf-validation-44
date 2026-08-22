# Graph parity evidence fixture contract

状態: `EXTRACTION_CONTRACT_DEFINED / PRODUCT_EVIDENCE_PENDING`

この文書はU-01〜U-04の「何を同一入力として比較するか」を固定する要求段階の契約である。
ここで実画像・実プロセスをPASSにしてはならない。製品証拠は実装後に同一SHAで取得する。

## 現行WIN-IDへの逆リンク

| U契約 | 所有する現行要求ID | 受入対象 |
| --- | --- | --- |
| U-01 同一入力 | WIN-B-001..024, WIN-L-006 | X/Windowsへ同じ3071点fixtureと同じperiod/reset/nowを入力する |
| U-02 geometry | WIN-B-004..024, WIN-C-017..019, WIN-L-008, WIN-M-006, WIN-M-027, WIN-M-029 | plot座標、軸、余白、最小幅、高DPIを同じ正規化式で比較する |
| U-03 graph意味論 | WIN-B-002..016, WIN-B-019..020, WIN-B-023..024 | 期間端、anchor、idle、欠測、bucket、系列、独立Remaining軸、epochを判定する |
| U-04 label/owner | WIN-B-017..018, WIN-B-021..024, WIN-G-006, WIN-G-010..012, WIN-M-019..024 | 系列順、色、単位、桁、文字列、表示所有者を突合する |

この表のIDは `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md` の同一行へ逆参照する。
未登録IDの証拠へ本fixtureを流用しない。

## 入力マニフェスト

`manifest.json` は次のキーを必須とし、JSON canonicalization（UTF-8、LF、キー辞書順、末尾改行なし）後の
SHA-256を `fixture_sha256` とする。

```json
{
  "fixture_id": "graph-parity-3071-v1",
  "artifact_role": {"x": "X", "windows": "Windows"},
  "source_sha256": {"x": "<sha256>", "windows": "<sha256>"},
  "reset_at": "RFC3339 with offset",
  "now": "RFC3339 with offset",
  "timezone": "IANA name",
  "metric": "dollar|token",
  "series": ["remaining", "luna", "terra", "sol"],
  "points_file": "points.jsonl",
  "points_sha256": "<sha256>"
}
```

`<sha256>`、日時、timezone、metric、series、入力ファイル名のいずれかが未確定なら比較は
`INCONCLUSIVE` とし、画像名や会話添付だけを代替にしない。

## canonical points（3071件）

`points.jsonl` はちょうど3071行。各行は `seq,timestamp,reset_at,remaining,model_cumulative,bucket,
observed,missing` を持ち、`seq` は0..3070の連番、timestampは単調非減少、remainingはnullまたは
0..100、model_cumulativeはnullまたは有限非負、bucketはUTC minute開始、observed/missingは真偽とする。
同一timestamp/同一bucket、欠測、初回観測前、idle、活動、終端、期間境界のセルをfixtureに含め、
セル一覧を `cells.json` として同じmanifestへ入れる。

## 正規化・合否式（実装後）

- X/Windowsが出力するraw pathは同じpoint列、同じreset/now/timezoneを参照する。
- plot geometry（940x640、700x480、実機サイズ）はmanifestへ記録し、各pointをplot矩形へ正規化する。
- Remainingは独立軸で `Y(0)=bottom, Y(50)=mid, Y(100)=top`。model dollar/tokenのmax変更でRemainingのYが変化したらFAIL。
- 対応pointごとに `abs(xX-xW) <= 1px` かつ `abs(yX-yW) <= 1px`。start anchor、first observation、idle hold、terminalを境界セルとして個別判定する。
- 余分な点、推測終端、欠測を0へ置換、同一SHAでない入力、別期間の混在はFAIL。

## 製品証拠の必須出力

raw normalized points、manifest、起動プロセスPID/command、source/payload SHA、fresh PNG、capture size、
画像SHA、判定ログを一つのevidence bundleへ保存し、X/Windows独立評価者が同じbundleを再計算する。
抽出段階ではこの出力が未取得でもよいが、製品受入段階で未取得なら `INCONCLUSIVE` とする。

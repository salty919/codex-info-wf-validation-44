# Domain contract 再監査 V6（2026-08-22）

## 判定

**FAIL（V4の残存テンプレートと依存ID不整合）**

**INCONCLUSIVE（製品証拠）**

最新のNormative Override V4を正本として再評価した。V4は要求別case_value、field、
expected式、fault/retention、dependency、oracle JSON expectedを大幅に具体化しているが、
カテゴリ内の共通negative・入力/観測の反復、未知依存ID、自己依存が残るため、構造ゲートは
FAILである。製品実証は未取得であり、別ゲートとしてINCONCLUSIVEに分離する。

## 1. 最新statとID・列完全性

### PASS

- 対象文書: 1221行。
- V4セクション: K–M、F–J、A–Eの3ブロック。
- V4行数: 226。
- V4 ID unique: 226。欠落・重複なし。
- baseline ID集合とのV4差分: 0。
- 全V4行に requirement、input、observable、exact_assert、negative、dependency、evidence
  が存在。
- 全226行に case_value が存在。
- oracle expectedの必須形状（requirement/value/field、row ID対応）失敗: 0。
- 抽出状態 EXTRACTION_INCOMPLETE は保持。

V1〜V3の履歴行は計数と判定から除外し、V4の3セクションだけを抽出した。

## 2. 正規化後のカテゴリ重複

ID、fixture、field/fault/oracleのID、要求文のcase/reason文字列を正規化して比較した。

| カテゴリ | 行数 | input unique | observable unique | negative unique | dependency unique | evidence unique |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A | 20 | 10 | 10 | 1 | 1 | 20 |
| B | 24 | 7 | 7 | 2 | 2 | 24 |
| C | 20 | 8 | 8 | 1 | 1 | 20 |
| D | 12 | 3 | 3 | 1 | 1 | 12 |
| E | 16 | 4 | 3 | 2 | 2 | 16 |
| F | 12 | 4 | 4 | 1 | 1 | 12 |
| G | 16 | 3 | 3 | 1 | 1 | 16 |
| H | 12 | 6 | 6 | 1 | 1 | 12 |
| I | 16 | 9 | 9 | 1 | 1 | 16 |
| J | 16 | 9 | 8 | 2 | 2 | 16 |
| K | 16 | 7 | 7 | 1 | 1 | 16 |
| L | 16 | 7 | 7 | 1 | 1 | 16 |
| M | 30 | 9 | 9 | 1 | 1 | 30 |

### 改善として確認できた点

- WIN-A-001のrelease diff、WIN-A-005の0/50/100 percent、WIN-A-007のUTC/local
  reset timestamp、WIN-K-002のDNS failure+recovery、WIN-M-001のdecision record complete
  など、case_valueとexpected_valueが要求語に応じて分化している。
- V4のoracle JSONは226件すべて requirement/value/field を持ち、raw trace、fresh artifact、
  same SHA、独立reviewer条件も列挙されている。

### 残存FAIL

1. negativeはA/C/D/F/G/H/I/K/M等で正規化後1テンプレートであり、fault IDと要求文を
   差し替えただけの「invalid counterpartを拒否してsafe stateを保持」に留まる。要求ごとの
   fault入力、保持対象、復旧境界を示していない。
2. input/observableはcase_valueの分化がある一方、同一case_valueを多数行で共有し、
   field_WIN-X-NNNのようなID由来の合成fieldを使う。これは実在の製品field、単位、順序、
   閾値を固定した証拠ではない。
3. dependencyは452参照中、未知ID WIN-D-014 と WIN-F-016 を含む。また138行が自身のIDを
   depends_onへ含む自己依存であり、前提契約として有効な依存グラフになっていない。
4. evidenceのexpected JSONは行ごとに異なるためV5より改善しているが、fieldが合成名であり、
   実際のAPI/DB/UI fieldやnegative oracleへの結び付きを確認できない。共通の取得 envelope
   だけで製品証拠を代替しない。

したがって、V4は「値を具体化した」点では改善しているが、全列が要求固有であるという
受入条件は満たさない。

## 3. 製品証拠

### INCONCLUSIVE

- 文書状態はEXTRACTION_INCOMPLETE。
- 文書自身が製品実証未取得・全行INCONCLUSIVEを示している。
- fresh image、process/host/DB/API trace、artifact SHA同一性、独立評価の実取得結果はない。
- コード、テスト、ビルド、インストール、実機確認は行っていない。

## 4. read-only検証

wc/rgで文書行数とV4セクションを確認し、V4のみを静的パースした。ID集合、キー存在、
case_value、oracle JSON形状、カテゴリ内正規化unique数、dependency参照の実在性・自己参照を
計算した。対象atomic文書、baseline、抽出状態は変更していない。

## 5. 最小修正タスク

要求ごとにnegativeの具体的fault・保持field・復旧境界を定義し、合成field_WIN-IDを実在の
対象fieldへ置き換える。dependencyは自身を除き、226要求台帳に存在する前提IDだけを関係理由
付きで参照し、未知IDを除去する。oracle JSONは実在field、単位、順序、閾値、negative traceへ
接続した後、新規独立Lunaで再監査する。

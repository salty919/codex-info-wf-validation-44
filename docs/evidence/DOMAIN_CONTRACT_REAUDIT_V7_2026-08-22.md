# Domain contract 再監査 V7（2026-08-22）

## 判定

**FAIL（V5の行固有性・意味field）**

**INCONCLUSIVE（製品証拠）**

最新のNormative Override V5を正本として再読込した。V5はcase_value、expected_value、
cross-domain dependency、fault、oracle JSONを前版より具体化し、依存IDの存在不整合を解消
している。しかしカテゴリ内の共通テンプレートと、real_semantic_fieldと称する合成名が
残るため、全226行の要求固有契約はPASSにできない。製品証拠は別ゲートのINCONCLUSIVEと
して分離する。

## 1. 最新stat・ID・列

### PASS（構造）

- atomic文書: 1462行。
- V5セクション: K–M、F–J、A–Eの3ブロック。
- V5行数: 226、ID unique=226。
- baseline IDとの差分: 0。
- 要求本文のID対応不一致: 0。
- 全行に requirement、input、observable、negative、dependency、evidence が存在。
- 全226行に requirement_case または case/value が存在。
- oracle expectedの要求・値・field形状の欠落: 0。
- dependency参照440件、未知ID=0、自己依存行=0。
- 抽出状態 EXTRACTION_INCOMPLETE は保持。

V1〜V4は監査履歴として計数から除外し、V5の3セクションだけを検査した。

## 2. 正規化後のカテゴリ重複

ID、fixture、fault/oracle ID、要求case/reason文字列を除いてカテゴリ内比較した。

| カテゴリ | 行数 | input unique | observable unique | negative unique | dependency unique | evidence unique |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A | 20 | 9 | 11 | 1 | 1 | 11 |
| B | 24 | 5 | 10 | 2 | 3 | 10 |
| C | 20 | 5 | 7 | 1 | 2 | 7 |
| D | 12 | 3 | 7 | 1 | 2 | 7 |
| E | 16 | 4 | 6 | 2 | 3 | 6 |
| F | 12 | 3 | 4 | 1 | 2 | 4 |
| G | 16 | 3 | 12 | 1 | 2 | 12 |
| H | 12 | 6 | 10 | 1 | 2 | 10 |
| I | 16 | 10 | 16 | 1 | 2 | 16 |
| J | 16 | 8 | 14 | 2 | 2 | 14 |
| K | 16 | 7 | 16 | 1 | 2 | 16 |
| L | 16 | 7 | 14 | 1 | 2 | 14 |
| M | 30 | 8 | 18 | 1 | 2 | 18 |

### 確認できた改善

- A-005はquota.remaining_percentと0/50/100%、A-006はquota.periodとweek/month、
  A-007はquota.reset_atとUTC時刻を結合している。
- F-001はsettings.atomic_file、F-002はi18n.catalog、K-002はssh.dns_errorなど、
  一部の行は意味fieldと具体値を持つ。
- dependencyは未知・自己参照がなく、V4のID不整合は解消されている。
- oracleは行ID、expected_field、expected_value、requirementを226件すべて持つ。

## 3. 残存FAIL

1. negativeはA/C/D/F/G/H/I/K/M等で正規化後1テンプレートである。fault IDと要求文を
   差し替えた「invalid caseをrejectしてlast-good/safe stateを保持」に留まり、要求別faultの
   入力値、失敗分類、保持対象、復旧境界が十分に分解されていない。
2. inputはカテゴリ内3〜10種類に留まるカテゴリが多く、同一のcase/valueと
   actor/entry/actionを要求文だけ変えて再利用している。observableもA=11/20、C=7/20、
   F=4/12、M=18/30など、同じ観測契約を複数行で共有する。
3. real_semantic_fieldの合成名を71件検出した。例として surface.semantic__、
   semantic_API_、semantic_SSH_、semantic_empty_details_、semantic__owner_ がある。
   これらは要求IDや要求語から機械生成した名前であり、既存のUI/API/DBの実在fieldを
   指していない。A-002/A-003/A-004でsurface.semantic__が反復することも確認した。
4. F-003のtimezone表示設定、F-004の接続状態など、異なる要求が同じ
   settings.atomic_fileへ写像されるサンプルがあり、fieldの意味対応が妥当か確認できない。
5. oracle JSONの形状は揃ったが、expected_fieldが合成fieldの場合は要求別oracleにならない。
   raw/fresh image/process/DB/host/SHA/独立reviewerという取得envelopeも共通で、実証結果は
   文書内に存在しない。

よって、V5は依存ID・値・oracle形式を改善したが、カテゴリ共通negative、重複する入力/観測、
合成semantic fieldが残り、要求固有性の受入条件はFAILである。

## 4. 製品証拠

### INCONCLUSIVE

- 文書状態はEXTRACTION_INCOMPLETE。
- 文書は製品実証未取得・全行INCONCLUSIVEを明記している。
- fresh image、process/host/DB/API trace、artifact SHA同一性、独立評価の実取得結果はない。
- コード、テスト、ビルド、インストール、実機確認は行っていない。

## 5. read-only検証と最小修正

wc/rgで文書行数とV5セクションを確認し、V5だけを静的パースした。ID集合、必須キー、
要求本文対応、カテゴリ内正規化unique数、dependencyの実在性・自己参照、合成field件数、
oracle形状を計算した。対象文書、baseline、抽出状態は変更していない。

negativeを要求別fault・保持field・復旧境界へ分解し、semantic_*等の合成名を実在fieldへ置換
する。重複するinput/observableは要求固有の数値・単位・時刻・状態・順序・所有者を追加し、
oracle expectedにも同じ実在fieldを結合した後、新規独立Lunaで再監査する。

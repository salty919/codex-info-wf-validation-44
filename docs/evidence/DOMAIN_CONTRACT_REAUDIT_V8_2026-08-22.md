# Domain contract 再監査 V8（2026-08-22）

## 判定

**FAIL（V7の要求固有性）**

**INCONCLUSIVE（製品証拠）**

最新のNormative Override V7を正本として再読込した。V7はfallback名を除いた意味field、
case/value、依存ID、要求別oracleの形を整え、未知依存・自己依存をなくしている。しかし
同一カテゴリ内で異なる要求が同じfield・値域・negativeを共有するため、要求固有契約の
受入条件はFAILである。製品証拠は未取得のためINCONCLUSIVEに分離する。

## 1. ID・列・依存の構造

### PASS

- atomic文書: 1934行。
- V7セクション: H–M、A–Gの2ブロック。
- V7行数: 226、ID unique=226。
- baseline IDとの差分: 0、要求本文不一致: 0。
- 全行に requirement、input、observable、negative、dependency、evidence が存在。
- 全226行にcase/valueが存在。
- oracleの要求・期待値・field形状欠落: 0。
- dependency参照440件、未知ID=0、自己依存行=0。
- 抽出状態 EXTRACTION_INCOMPLETE は保持。

## 2. 正規化後のカテゴリ重複

ID、fixture、fault/oracle ID、要求case/reason文字列を除いてカテゴリ内比較した。

| カテゴリ | 行数 | input unique | observable unique | negative unique | dependency unique | evidence unique |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A | 20 | 7 | 20 | 1 | 1 | 7 |
| B | 24 | 5 | 24 | 1 | 3 | 5 |
| C | 20 | 6 | 20 | 1 | 2 | 6 |
| D | 12 | 4 | 12 | 1 | 2 | 4 |
| E | 16 | 4 | 16 | 1 | 3 | 4 |
| F | 12 | 2 | 12 | 1 | 2 | 2 |
| G | 16 | 3 | 16 | 1 | 2 | 3 |
| H | 12 | 2 | 12 | 1 | 2 | 2 |
| I | 16 | 2 | 16 | 1 | 2 | 2 |
| J | 16 | 5 | 16 | 1 | 2 | 5 |
| K | 16 | 4 | 16 | 1 | 2 | 4 |
| L | 16 | 4 | 16 | 1 | 2 | 4 |
| M | 30 | 6 | 30 | 1 | 2 | 6 |

### 確認できた改善

- fallback形式のsemantic_*、field_*、要求本文そのものをfieldにした値は検出しなかった。
- dependencyは実在IDだけで構成され、未知・自己参照は0。
- 例としてA-005はQuota.remaining_percentと0/50/100、A-006はQuota.periodとweek/month、
  H-001はInstaller.payloadとrunnable self-contained、K-002はssh.dns_errorとDNS
  fail/recoveryを持つ。
- oracleは226行すべてに要求、expected field、expected value、raw/fresh/SHA/reviewerを持つ。

### 残存FAIL

1. negativeは全13カテゴリで正規化後1テンプレートである。
   「invalid input for exact requirement => reject; preserve last-good or explicit unavailable/
   safe hold; recovery is requirement-specific」という共通文を、要求文だけ差し替えている。
   要求別fault入力、保持field、部分結果、復旧境界が定義されていない。
2. inputの値域がカテゴリ共通である。A-001、A-002、A-004、A-007は同じ
   state/auth/quota fixtureと同じMain操作境界を共有し、H-001〜H-006は同じ
   install/update/uninstall/purge値域を共有する。case文字列だけでは要求固有の入力境界に
   ならない。
3. observableは行数上uniqueに見えるが、実質的な意味fieldの再利用がある。A-001/A-002/
   A-004/A-007は同じMain.surface、H-001〜H-006は同じInstaller.payloadであり、異なる要求の
   exact expectedを同じfieldへ投影している。要求によっては表示対象、検証対象、状態遷移が
   異なるため、field再利用だけではexact assertionを閉じられない。
4. dependencyはIDとしては有効だが、Aでは複数行が同じI-001/J-001、F/H/I等でも同一の
   cross-domain前提を共有する。依存field・成立順・失敗時の依存保持が要求別に記録されていない。
5. oracle JSONの形は行別でも、expected field/valueが上記の共有field・共有値域に依存する。
   共通のraw/fresh/SHA/reviewer envelopeは実証結果ではなく、製品証拠取得後に個別判定が必要。

したがって、V7はsynthetic fallbackと依存ID不整合を解消したが、全列を要求固有にする基準、
特にnegativeと入力境界・意味fieldの要求別分解を満たさない。

## 3. 製品証拠

### INCONCLUSIVE

- 文書状態はEXTRACTION_INCOMPLETE。
- 文書は製品実証未取得・全行INCONCLUSIVEを明記している。
- fresh image、process/host/DB/API trace、artifact SHA同一性、独立評価の実取得結果はない。
- コード、テスト、ビルド、インストール、実機確認は行っていない。

## 4. read-only検証と最小修正

wc/rgで文書行数とV7セクションを確認し、V7だけを静的パースした。ID集合、必須キー、
case/value、カテゴリ内正規化unique数、依存IDの実在性・自己参照、fallback field、oracle形状を
計算した。対象文書、baseline、抽出状態は変更していない。

要求別faultを具体的な入力・失敗分類・保持field・復旧境界へ分解し、同一fieldを共有する行は
表示対象・状態遷移・単位・値域を要求ごとに固定する。dependencyは依存fieldと成立順を追加し、
oracle expectedにも同じ要求固有値を結合した後、新規独立Lunaで再監査する。

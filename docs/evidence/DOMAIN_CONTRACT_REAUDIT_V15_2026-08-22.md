# Domain contract 再監査 V15（V14 A–M 226行、2026-08-22）

## 判定

- **構造判定: FAIL**。V14のID、field名、negative文字列、依存参照は機械的には揃うが、
  入力・期待値・否定条件・oracleの意味的な行固有性が閉じていない。
- **製品証拠: INCONCLUSIVE**。V14に記載されたraw/fresh artifactは参照名だけで、今回もコード、
  テスト、ビルド、実機、実プロセス、DB、画面証拠を実行・取得していない。
- **抽出ゲート: 閉じられない**。状態は `EXTRACTION_INCOMPLETE / HOLD` を維持する。
  機械的な集合PASSを構造・製品PASSへ昇格させない。

## 範囲・再現条件

比較対象は次の2文書のみとした。atomic assertionsの古い履歴行は無視し、見出し
`## 行固有Normative Override V14（A–M）`（2855行）直後の226行（2858–3083行）だけを監査した。

- `docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`

ID抽出、重複数、field/negative抽出、依存のknown/self判定は `sed`、`rg`、`awk`、`perl`、`comm`で
read-onlyに行った。コード、テスト、ビルド、インストール、実機確認は行っていない。

## 1. ID差分と行数

| 検査 | baseline | V14 | 判定 |
| --- | ---: | ---: | --- |
| ID raw行数 | 226 | 226 | PASS |
| ID unique数 | 226 | 226 | PASS |
| ID重複 | 0 | 0 | PASS |
| baselineとの差分 | — | 空集合 | PASS |

baselineの `WIN-A-001..020`、`WIN-B-001..024`、`WIN-C-001..020`、`WIN-D-001..012`、
`WIN-E-001..016`、`WIN-F-001..012`、`WIN-G-001..016`、`WIN-H-001..012`、
`WIN-I-001..016`、`WIN-J-001..016`、`WIN-K-001..016`、`WIN-L-001..016`、
`WIN-M-001..030`とV14のID集合に差分はない。これは集合突合だけのPASSである。

## 2. field / negative / dependency の機械突合

| 検査 | raw | unique / 状態 | 所見 |
| --- | ---: | ---: | --- |
| `observable=field` | 226 | 226、共有0 | **PASS（文字列一意性のみ）** |
| `negative`（`retention`前まで） | 226 | 226、重複0 | **PASS（文字列一意性のみ）**。field/要求文を埋め込んだ合成差分を含む |
| dependency edge | 441 | unknown 0、self 0 | **PASS（参照整合性）** |

fieldの共有は現行V14では検出しなかった。以前同じfieldを共有していたE-002/I-002/K-002も、
現行V14ではそれぞれ `Setup.connection_architecture`、`Api.ssh_local_tunnel_boundary`、
`Error.ssh_dns_resolution` として分離されている。

依存は全てbaseline IDへ解決でき、自己依存もない。ただし224行の依存表現は
`depends_on=[...] (valid,non-self; reason=<要求文>)`という同型であり、known/selfが0でも、
依存理由が実データ境界・所有者・consumerを示す独立証拠にはならない。

## 3. synthetic marker と意味的行固有性

V14はIDを差し替えた同一文型を多数残している。検出数は次のとおりである。

| marker | 行数 | 判定 |
| --- | ---: | --- |
| `case=case=<要求本文>` | 155 | **FAIL**。要求文の自己反復で、具体的な入力caseではない |
| `actor/entry/action bound` | 224 | **FAIL**。actor/entry/actionの実値がない |
| `explicit bounds` | 224 | **FAIL**。境界値が展開されていない |
| `negative ... invalid matrix` | 224 | **FAIL**。invalid matrixのcase、期待reject、保持対象がない |
| `last-good or explicit safe state` | 224 | **FAIL**。要求ごとの保持/消去/復旧がない |
| `exact_assert ... satisfies` | 224 | **FAIL**。fieldが要求文を満たすという同語反復で、独立期待値でない |
| `valid,non-self` | 224 | **FAIL**。依存検査の印字であり、依存の意味的根拠でない |
| `same SHA` + `independent reviewer` | 226 | **INCONCLUSIVE**。実SHA・実artifact・評価結果がない |

残り2行（WIN-E-008、WIN-J-001）は短い個別形式で、具体的なcommand/DB hashを持つ一方、V14の
actor/entry/bounds、invalid matrix、標準retention、expected_field等の共通構造を持たない。
V14の行スキーマが統一されていないため、構造ゲートは閉じない。

## 4. 入力・期待値・oracleの行固有性

### 入力

- fixture IDを含むraw文字列は226 uniqueだが、意味的なcaseは固有でない。
- 155行は `case=case=<要求本文>` で、要求を入力値として再掲しているだけである。
- 明示値を持つ行でも、同じ入力ベクトルを複数要求が共有する。代表例は次のとおり。

| 入力ベクトル | 共有行数 | 代表的なID群 |
| --- | ---: | --- |
| `percent=0\|50\|100` | 12 | A-005、B-005/B-008/B-009/B-010/B-011/B-012/B-014/B-019、C-004/C-005/C-010 |
| `auth=unauth\|auth\|expired` | 12 | A-003/A-017、C-002/C-008、D-011、E-003/E-013/E-014、F-005、K-004、L-007、M-014 |
| `argv/host/user/listening` | 11 | C-016、E-002/E-004/E-005/E-006/E-010/E-011/E-016、I-002、K-002/K-003 |
| `tokens=1000/250/50;dollars=24.33` | 9 | A-010、B-013/B-015/B-020/B-023、D-009、G-012、I-012/I-015 |
| `week\|month;restart=true` | 6 | A-006、B-001/B-002/B-003/B-004、M-006 |
| `dpi=100\|125\|150\|200;center<=2px;jitter<=1px` | 5 | C-019、K-014、L-008、M-027/M-029 |

例えばB-002「右端=min(reset_at,now)」はB-001/B-003/B-004等と同じ
`week|month;restart=true`だけで、reset_at/nowの具体値と境界別期待座標がない。

### 期待値・assert

- `expected`のraw文字列は169 unique、同一値の重複パターンは10組だが、多くは入力ベクトルの
  反復である。
- 155行は入力の自己反復に対応して `expected=case=<要求本文>` とし、値・単位・順序・座標・
  保持結果を指定しない。
- 224行のassertは `field satisfies <要求本文> under <case>` であり、実装から独立した期待値を
  計算する式ではない。E-008のcommand、J-001のDB hashだけが短い具体比較形式である。

### oracle

- 226行ともID付きoracle、raw ID、same SHA、independent reviewerを持つため、文字列上はID固有である。
- 224行は `expected_field`、`expected_case`、`requirement`を同じ要求文から生成し、
  `fresh artifact image/process/DB/host`という共通列挙を使う。実fixtureの入力hash、期待値ファイル、
  観測位置、合否式、実SHAがないため、独立oracleとして閉じていない。
- E-008/J-001は個別oracle形式だが、実artifact・raw trace・SHAがないため製品判定はINCONCLUSIVEである。

## 5. ゲート結論

| ゲート | 判定 | 根拠 |
| --- | --- | --- |
| baseline↔V14 ID集合 | PASS | 226/226、差分0 |
| field文字列共有 | PASS | 226 unique、共有0 |
| negative文字列unique | PASS（限定） | 226 uniqueだが224行は合成invalid-matrix文型 |
| dependency unknown/self | PASS | 441辺、unknown 0、self 0 |
| 入力の意味的行固有性 | **FAIL** | self-case 155行、共有ベクトル、実境界不足 |
| 期待値/assertの独立性 | **FAIL** | 224行の` satisfies`自己反復、具体値不足 |
| oracleの独立性・fresh証拠 | **INCONCLUSIVE** | 実raw/fresh/artifact SHA/独立判定未取得 |
| 抽出ゲート | **CLOSED不可** | FAILとINCONCLUSIVEが残る |

したがってV14は、旧版よりfield/negativeの文字列一意性とdependency参照整合性は改善しているが、
合成markerと自己参照expected/assertを含むため、226件の行固有契約を閉じたとは判定しない。
入力case・期待値・invalid matrix・保持結果・oracle合否式を実値へ展開し、実証証拠を同一artifact SHAへ
結び付けるまで、`EXTRACTION_INCOMPLETE / HOLD`を維持する。

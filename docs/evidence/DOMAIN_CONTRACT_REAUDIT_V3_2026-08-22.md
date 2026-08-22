# Domain contract 再監査 V3（2026-08-22）

## 判定

**FAIL（構造・行固有契約）**

製品実証については、対象文書自身が製品証拠未取得を示しているため、該当ゲートは
**INCONCLUSIVE** とする。構造上のFAILがあるため、文書監査全体をINCONCLUSIVEやPASSへ
丸めない。

抽出状態は変更していない。

## 監査範囲と制約

次の文書だけを読み取り、コード、テスト、ビルド、インストール、実機・画面確認は実施していない。

- `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md`
- `docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md`
- `docs/UX_DECISION_NON_SCROLL_2026-08-22.md`
- `docs/UX_DECISION_GRAPH_LABELS_2026-08-22.md`

## 1. ID・拡張の完全性

### PASS（集合としてのID突合）

対象テーブルの行数は次のとおりだった。

| 文書 | ID行数 |
| --- | ---: |
| extraction baseline | 226 |
| 主行契約台帳 | 226 |
| domain atomic assertions | 226 |
| lifecycle extension（D/K/M） | 58 |
| data-protection extension（I/J） | 32 |

baselineのカテゴリ範囲は A=20、B=24、C=20、D=12、E=16、F=12、G=16、H=12、
I=16、J=16、K=16、L=16、M=30 の合計226である。主台帳とatomic assertionの
ID集合に差分はなく、D/K/Mの58 IDはlifecycle extensionに、I/Jの32 IDは
data-protection extensionに存在した。

主台帳冒頭の同一ID行参照（atomic assertion、D/K/M extension、I/J extension）と、
`WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md` の同一参照設計は確認できた。従って、
ID集合の欠落・余剰は本監査では検出しなかった。

## 2. 行固有契約

### FAIL（atomic assertionの入力・否定・依存がテンプレートのまま）

226行が6列（ID、input/precondition、exact observable/assertion、negative/retention、
dependency、evidence oracle）を持つことは確認できた。しかし、列が埋まっていることと、
要求固有の契約になっていることは別であり、次の具体的なテンプレート残存を確認した。

- `WIN-A-001`〜`WIN-A-003`（および同カテゴリの行）の input は、
  `state=initial/auth/ready/warn/error; locale=ja/en; size=700x480/standard/highDPI; auth/API fixture`
  という同一の共通ベクトルで、要求ごとの入力値・境界・操作順・再入条件が追加されていない。
- 同じAカテゴリ行の negative/retention は通信・認証・空/null/invalid、last-good、秘密・重複を
  一括列挙する共通文で、要求固有の失敗条件、保持対象、復旧境界が分離されていない。
- 同じAカテゴリ行の dependency は `I/J→A` のカテゴリ依存であり、個別要求がどの契約・状態・
  前提に依存するかを示していない。
- exact observable は要求本文を文中に差し替え、fixture IDを付けた形だが、観測field、許容値、
  順序、境界値、失敗時の観測結果が要求ごとに固定されていない。
- evidence oracle は `raw=WIN-X-NNN.jsonl` などIDを含む一方、ID文字列を変えただけの共通
  process/DB/hash/fresh-image/or independent-reviewer条件が残る。fixture名だけでは、要求固有の
  oracle、期待値、否定証拠を定義したことにならない。

主台帳側にも、カテゴリ共通の entry/precondition/security-performance/implementation-target と
共通の失敗保持文が残り、atomic assertionの行固有化を補えていない。`visual owner=ID`、
`fixture=ID`、`raw=ID` は識別子であって、要求固有の入力・観測・否定・依存・証拠条件ではない。

### 拡張台帳の扱い

lifecycle 58行とdata-protection 32行はIDと列の存在が揃っているため、拡張の欠落は検出しなかった。
ただし、拡張行が存在することは、226行のatomic assertionに要求固有の5項目が埋まっていることの
代替証拠ではない。主台帳・atomic assertion・拡張の同一IDを結合した後も、上記の共通ベクトル、
共通否定文、カテゴリ依存を行固有値へ置き換える必要がある。

## 3. トレーサビリティとDecision

`WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md` はカテゴリ既定値の継承だけではverifiedにしない、
226行すべてにatomic判定式と拡張を登録する、と規定している。しかし同文書の状態は
`IN_PROGRESS / HOLD` であり、上記のテンプレート残存によりその設計ゲートを満たしていない。

Decision系として確認した次の2文書も実装・証拠待ちである。

- `UX-20260822-UX-002`: `EXTRACTION_DECISION_RECORDED / IMPLEMENTATION_PENDING`
- `UX-20260822-GRAPH-001`: `EXTRACTION_DECISION_RECORDED / EVIDENCE_PENDING`

Decisionの記録自体は存在するが、実装・製品証拠が無いためDecisionの受入PASSを示すものではない。

## 4. 製品証拠

### INCONCLUSIVE（対象外の実証を行っていない）

- baseline、主台帳、atomic assertionsはいずれも `EXTRACTION_INCOMPLETE`。
- atomic assertionsの冒頭は全行について製品実証未取得・実証判定`INCONCLUSIVE`と明記している。
- 主台帳の `independent_reviewer` も各行を `INCONCLUSIVE (証拠未取得)` としている。
- 本監査では要求どおりコード・テスト・ビルド・実機を実行していないため、製品のPASS/FAILを
  推測していない。

## 検証証拠

read-onlyで以下を実行した。

```text
wc -l <対象7文書およびDecision文書>
rg -c '^\\| WIN-[A-Z]-[0-9]{3} \\|' <baseline/main/atomic/lifecycle/data>
rg <baselineカテゴリ範囲>
ID集合をsort/commでbaseline↔main↔atomic、D/K/M↔lifecycle、I/J↔dataに突合
```

観測結果は 226/226/226、58、32 行で、集合差分は空だった。一方、行固有性は上記の
同一カテゴリ行の具体例と共通テンプレートでFAILとした。

## 最小修正タスク（実装担当へ返却）

1. 226行すべてについて、input/precondition、exact observable、negative/retention、dependency、
   evidence oracleを、fixture IDの置換ではなく要求固有の値・境界・操作・失敗保持・依存・期待値へ
   展開する。
2. 主台帳の各行からatomic assertionと該当extensionの同一IDへ到達できる行単位の参照を維持し、
   参照先と内容の不一致をゼロにする。
3. 文書構造がPASSになった後、別ゲートとして製品証拠を取得し、`INCONCLUSIVE`を解消する。


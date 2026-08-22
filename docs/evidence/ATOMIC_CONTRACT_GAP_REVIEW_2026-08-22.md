# Windows domain atomic assertions 行固有性ギャップ監査（2026-08-22）

## 判定（FAIL と INCONCLUSIVE を分離）

- **FAIL（文書構造・行固有契約）**: `docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md`
  の226行は、ID集合としては揃っているが、baselineが要求する行固有の入力・観測・失敗保持・依存・
  証拠条件を満たしていない。カテゴリ共通テンプレート、`fixture=ID`、`raw=ID`、`要求本文どおり`の
  置換は、行固有の契約値ではない。
- **INCONCLUSIVE（製品実証）**: atomic assertions 自身が製品実証未取得・全行の実証判定
  `INCONCLUSIVE` と明記している。今回もコード、テスト、ビルド、インストール、実プロセス、DB、
  画面キャプチャを実行していないため、実装のPASS/FAILは判定しない。
- **限定的な集合突合のみPASS**: baselineとatomicのID行数・ID集合は226件で一致する。しかし、
  これは要求契約や製品実証のPASSではない。

抽出状態は `EXTRACTION_INCOMPLETE` のままとする。FAILをINCONCLUSIVEへ、またはINCONCLUSIVEを
PASSへ丸めない。

## 監査範囲と方法

読み取った文書は次の2件だけである。

- `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`
- `docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md`

baselineの要求表とatomic assertionsの行をIDで突合し、次を読み取り比較した。

1. 226件のID集合・カテゴリ件数。
2. baseline §1.1 の必須フィールド、§1.2 の状態・イベント・データ直積、§1.3 の分割規則と、
   atomic assertionsの6列（`input / precondition vector`、`exact observable / assertion`、
   `negative / retention invariant`、`dependency`、`evidence oracle`）の対応。
3. 同一カテゴリ内の入力、否定条件、依存、証拠oracleの反復と、baseline行の要求本文との具体的な
   差分。

行数・文字列の読み取りには `wc`、`rg`、`awk`、`sed` のみを使用した。コード変更、テスト、
ビルド、インストール、実行環境の操作は行っていない。

## 1. ID集合の突合（集合だけのPASS）

baseline §0のカテゴリ件数と、baseline/atomic双方のID行数は次のとおりである。

| 区分 | 件数 | baseline範囲 | atomic範囲 | 集合判定 |
| --- | ---: | --- | --- | --- |
| A | 20 | WIN-A-001..020 | WIN-A-001..020 | PASS |
| B | 24 | WIN-B-001..024 | WIN-B-001..024 | PASS |
| C | 20 | WIN-C-001..020 | WIN-C-001..020 | PASS |
| D | 12 | WIN-D-001..012 | WIN-D-001..012 | PASS |
| E | 16 | WIN-E-001..016 | WIN-E-001..016 | PASS |
| F | 12 | WIN-F-001..012 | WIN-F-001..012 | PASS |
| G | 16 | WIN-G-001..016 | WIN-G-001..016 | PASS |
| H | 12 | WIN-H-001..012 | WIN-H-001..012 | PASS |
| I | 16 | WIN-I-001..016 | WIN-I-001..016 | PASS |
| J | 16 | WIN-J-001..016 | WIN-J-001..016 | PASS |
| K | 16 | WIN-K-001..016 | WIN-K-001..016 | PASS |
| L | 16 | WIN-L-001..016 | WIN-L-001..016 | PASS |
| M | 30 | WIN-M-001..030 | WIN-M-001..030 | PASS |
| **合計** | **226** | **226** | **226** | **PASS（集合のみ）** |

IDの一致は、atomic assertionsの各行がbaselineの最小観測可能な要求として固有化されたことを
示さない。以下の構造FAILを別途判定する。

## 2. 構造上のFAIL

### 2.1 baselineの必須フィールドを6列へ圧縮している

baseline §1.1 は、IDを含む次の12の契約フィールドを必須としている。

`requirement_id`、`actor / entry`、`precondition`、`action / observable`、`data_contract`、
`visual_contract`、`failure_contract`、`persistence_contract`、`security_contract`、
`performance_contract`、`evidence_oracle`、`owner_and_dependency`。

atomic assertionsは6列しかなく、次の情報が行内に存在しないか、異なる意味のまま一括されている。

| baseline必須情報 | atomic assertions上の状態 | 判定 |
| --- | --- | --- |
| actor、entry、操作開始条件 | `input / precondition vector`に状態名だけが並び、誰がどの入口で何を開始するかがない | FAIL |
| action、データ型・単位・範囲・丸め・順序 | `exact observable`はbaseline要求文と`fixture=ID`の差し替えで、期待値がない | FAIL |
| 座標、軸、色、文字、所有者、重複禁止 | `正本field/順序/単位/状態/所有者/操作結果を要求本文どおり`という未展開語 | FAIL |
| 失敗分類、保持/消去、再試行、復旧 | カテゴリ全行共通の否定文で、要求固有の保持対象がない | FAIL |
| 保存、再起動、更新、削除の境界 | `negative / retention invariant`へ保存・秘密・payload等が混在し、行固有の書込み集合がない | FAIL |
| security、performance | 一部は共通否定文へ混入し、trust boundary、上限、周期、再入、停止時挙動がない | FAIL |
| owner、依存の実体 | `I/J→A`等のカテゴリ関係だけで、フィールド・実装箇所・前後要求がない | FAIL |
| oracle、期待値、freshness、独立判定 | `raw=ID`と`expected=<ラベル>`、`when applicable`で、期待値と合否式がない | FAIL |

### 2.2 直積を列挙しているだけで、行ごとのセル判定がない

baseline §1.2 は、各原子要求について起動/接続、データ、グラフ、Window、入力・ライフサイクル、
永続化・競合の各セルを `applicable` または根拠付き `not-applicable` とするよう要求している。
atomic assertionsはカテゴリ単位のベクトルを各行へコピーしているだけで、どのセルをその行で閉じるか、
非該当ならなぜ非該当かを記録していない。

例えばBカテゴリ24行は全て
`fixture=graph-parity-3071-v1; reset/now/timezone/metric/series; boundary=...; geometry=...`
であり、B-002の右端境界とB-014の同一分bucket規則に同じ直積を割り当てている。セル名の存在は、
具体的入力値・期待出力・操作順が存在することを意味しない。

### 2.3 baselineが要求する分割規則を満たさない複合行が残る

baseline §1.3 は複数責務を別IDへ分割するよう指定している。atomic assertionsはbaseline要求文を
そのまま1つの`exact observable`へ埋め込んでいるため、例えば次が未分割である。

- WIN-A-001: 全監視機能の非削減、全画面差分、状態、操作、秘密・重複禁止を一つへ束ねる。
- WIN-B-024: 線、軸、凡例、ラベルの同一データ世代という複数の観測対象を一つへ束ねる。
- WIN-M-016: 原因、影響、復旧操作の分離とraw backend errorの秘匿を一つへ束ねる。
- WIN-L-007: 正常、認証、警告、危険、0、100、エラーという複数状態を列挙するだけで、状態ごとの
  観測・期待値・非該当理由を分離していない。

226件を維持するために条件を隠すのではなく、分割が必要なら件数を増やし旧ID対応を記録する必要が
ある。これはbaseline §1.4の下限規則にも反するため、構造FAILである。

## 3. 行固有性不足の具体例（18件）

以下はID、baseline要求、atomic assertionsの記載、行固有化に不足する値を突合した具体例である。
`fixture=ID`やoracle名がIDごとに違って見えても、期待値・否定条件・対象資源が未定義なら固有契約
とは数えない。

| ID | baseline要求（baseline行） | atomic assertionsの観測 | 行固有性の不足（FAIL理由） |
| --- | --- | --- | --- |
| WIN-A-002 | アカウント状態を表示する（baseline:119） | `state=initial/auth/ready/warn/error...`、`expected=状態fixture画像` | アカウント状態の正本field、許容状態値、画面上の所有者、状態遷移、未取得時の保持対象、画像内の期待文言/位置がない。A-003と入力・否定・依存が同一で、アカウントと認証を区別できない。 |
| WIN-A-003 | 認証状態を表示する（baseline:120） | A-002と同じ入力・否定・`I/J→A`、`expected=auth/ready画像` | 認証状態の値域、API/設定のどの値を表示するか、auth失敗とAPI失敗の分岐、auth/readyの具体的期待値がない。画像名は期待値ではない。 |
| WIN-B-002 | 現在期間の右端は `min(reset_at, now)`（baseline:144） | graph 3071 fixtureと複数境界名、`expected=単体+実画像` | `reset_at`と`now`の具体値、`now<reset`/`now=reset`/`now>reset`ごとの終端、x座標・許容誤差、単体と画像の照合式がない。境界名の列挙だけでは式の観測oracleにならない。 |
| WIN-B-014 | 同一分bucket内の残量値の採用規則を固定する（baseline:156） | Bカテゴリ共通ベクトル、`expected=raw point trace` | 同一分に複数残量がある場合の採用規則（最大、最新、同値時のtie-break等）、入力点、採用後の点列がない。raw traceというラベルだけで規則を固定していない。 |
| WIN-C-010 | 残量ゲージは左から右へ塗る（baseline:181） | Cカテゴリ共通状態、`expected=pixel audit` | barの左端/幅、fillのx起点・終点、0/中間/100の期待ピクセル、許容誤差がない。`pixel audit`は独立計算された期待値ではない。 |
| WIN-C-011 | 0/中間/100でゲージ方向が変わらない（baseline:182） | 同じCベクトル、`expected=three images` | 3枚それぞれの入力値、塗り幅、左端保持、画像の比較領域がない。C-010との差分も期待式として明示されていない。 |
| WIN-D-005 | 親→子の深さ優先順を維持する（baseline:201） | `snapshot epoch/root/child/orphan/cycle...`、`expected=order test` | 親子辺、同深度の並び、root/childのID、cycle/orphanの扱い、期待行列がない。`order test`はテストoracleの名前に過ぎない。 |
| WIN-E-010 | Windows側からSSH転送を開始できる（baseline:223） | alias/host/user/port/8787、成功/失敗、初回/再起動/menu、`expected=process trace` | 引数から実行プロセスへの対応、開始成功の観測（PID/listening/API到達）、終了・再試行・取消の状態、host/userを保存しない境界がない。process traceだけでは合否式がない。 |
| WIN-F-008 | 正常設定をatomicに保存する（baseline:242） | valid/empty/malformed/truncated/permission、atomic replace、`expected=file/hash test` | 書込み集合、temp→replaceの可視境界、再起動後の値、権限失敗時の旧ファイル保持、ハッシュの期待関係がない。失敗文の`旧payload`は設定行の契約値ではない。 |
| WIN-G-010 | 日付をlocale/timezoneに従わせる（baseline:261） | `locale=ja/en/de; timezone=local/UTC...`、`expected=timezone images` | 固定timestampに対するlocale別文字列、timezone変換、日付境界、文字列幅/位置、期待フォーマットがない。画像名では数値・文字列の正本を再計算できない。 |
| WIN-H-008 | 部分コピー時にshortcutを公開しない（baseline:280） | install/update/rollback/uninstall等の共通ベクトル、`expected=failure fixture` | どのコピー段階で失敗させるか、旧payload/shortcut/registryの前後集合、公開禁止の判定、rollback完了条件がない。`failure fixture`は失敗入力を特定しない。 |
| WIN-I-007 | JSON schemaを厳格検証する（baseline:296） | malformed/unknown/duplicate/NaN/oversize等、`expected=malformed fixture` | schema version、許可key/type/range、各不正入力のreject結果、last-good保持と応答境界がない。I-008/I-009と同一ベクトル・同一否定文で、厳格検証の意味が固定されていない。 |
| WIN-I-009 | duplicate keyを拒否する（baseline:298） | Iカテゴリ共通ベクトル、`expected=schema fixture` | duplicate keyの位置/値、パーサーの拒否結果、HTTP/API状態、snapshot置換禁止の具体値がない。I-007と異なるのは要求文とIDだけである。 |
| WIN-J-012 | 複数serverが同じDBへ二重登録しない（baseline:322） | 2+server/client/daemon、busy/full/corrupt等、`expected=concurrent writer` | 参加プロセスID、開始順、transaction/leaseの観測、期待行数、競合時に保持するDB状態がない。`concurrent writer`は同時実行の入力と合否を指定しない。 |
| WIN-K-009 | stale thread rowsを表示しない（baseline:340） | API/SSH/window、child id、singleton、DPI/座標/cursor等を同一入力へ列挙、`expected=stale fixture` | 旧epoch/新epochの到着順、除外対象と保持対象、画面更新時点、stale判定式がない。monitor/DPI/cursorはこの要求の固有入力として整理されていない。 |
| WIN-L-005 | 古い画像を現行証拠へ流用しない（baseline:357） | 全ID、artifact SHA、fresh process、`expected=freshness check` | 画像の生成時刻・対象process・artifact SHA・変更後の無効化関係、古い画像を検出する合否式がない。証拠管理行なのにUIのclip/overflow等の共通否定文も混入する。 |
| WIN-M-016 | 原因・影響・復旧操作を分離しraw backend errorを出さない（baseline:389） | UX共通ベクトル、`expected=redaction/text audit` | 失敗原因ごとの文言、影響、次操作、redaction対象、raw文字列禁止の検査対象が分離されていない。複数責務を1つのobservableに残している。 |
| WIN-M-025 | keyboardのみでmenu/主要操作/戻る/閉じるへ到達（baseline:398） | UX共通ベクトル、`expected=keyboard traversal log` | キー列、focus順、各controlのaccessibility name、disabled/busy時の遷移、到達後の操作結果がない。ログ名だけではkeyboard oracleにならない。 |

## 4. テンプレート反復の横断的証拠

atomic assertionsの`input / precondition vector`は、カテゴリ内の全行で同一である。行数は入力テンプレート
の反復回数であり、行固有値の数ではない。

| カテゴリ | 同一入力ベクトルの行数 | 同一否定/保持文の行数 | 同一カテゴリ依存の行数 |
| --- | ---: | ---: | ---: |
| A | 20 | 20 | 20 (`I/J→A`) |
| B | 24 | 24 | 24 (`I/J→B`) |
| C | 20 | 20 | 20 (`I/J→C`) |
| D | 12 | 12 | 12 (`I/J→D`) |
| E | 16 | 16 | 16 (`I→E→F`) |
| F | 12 | 12 | 12 (`E→F`) |
| G | 16 | 16 | 16 (`C/M→G`) |
| H | 12 | 12 | 12 (`A/F→H`) |
| I | 16 | 16 | 16 (`I boundary`) |
| J | 16 | 16 | 16 (`I/J data boundary`) |
| K | 16 | 16 | 16 (`I/J→K`) |
| L | 16 | 16 | 16 (`A-K/M→L`) |
| M | 30 | 30 | 30 (`D/K→M`) |
| **合計** | **226** | **226** | **226** |

否定文は4種類のカテゴリ共通文へ集約されている。

- A/C: 通信・認証・空/null/invalid、last-good/未取得、派生値・秘密・重複禁止（40行）。
- B/D/I/J/K: 不正・欠測・重複・世代不一致・部分結果、旧snapshot/DB/設定/last-good保持、
  0/空/推測終端/秘密/二重登録禁止（84行）。
- E/F/H: 失敗・取消・再起動・権限不足、部分保存・秘密保存・破壊的削除禁止、旧設定/旧payload/
  ユーザーデータ/復旧導線保持（40行）。
- G/L/M: clip/overflow/重複文言/未登録label/焦点奪取/古い画像/SHA不一致/未確認をPASSにしない、
  HOLD/INCONCLUSIVE伝播（62行）。

例えばAPI schema行、pixel行、証拠freshness行へ同じDB/snapshot/last-good保持を適用している。これは
共通安全既定値の参照ではなく、各要求の failure/persistence/security 契約を欠いたままのコピーである。

さらに全226行のoracleは `raw=WIN-<ID>.jsonl` と `fresh image/process/DB/host/hash when applicable; same artifact SHA; independent reviewer required`
を共有する。IDを変えたrawファイル名だけでは、入力ハッシュ、期待値、観測field、freshness判定、
独立評価の合否式を固定しない。`when applicable`も、非該当の理由を記録しない未確定語である。

## 5. 226行を行固有化する最小スキーマ案

baselineの必須契約を削らず、atomic assertionsの6列を次の**12契約フィールド＋2判定メタデータ**へ
展開する。1行は1つのaction/observableだけを持つ。baseline §1.3により複数責務が残る場合は、
新しい子IDを発行し、旧IDとの対応を記録する。

| フィールド | その行で必須の最小内容 | 行固有性の機械判定 |
| --- | --- | --- |
| `requirement_id` | baselineの1要求ID。分割時は`parent_id`/対応先 | ID重複・未対応・余剰が0 |
| `actor_entry` | actor、入口、開始イベント、操作主体 | カテゴリ名だけ、またはactorなしを拒否 |
| `precondition_input_cells` | 固定fixture/hash、認証/接続/locale/DPI等の具体値、操作順、境界セル。非該当は理由付き | `all`やカテゴリ共通ベクトルだけを拒否。各cellにcase IDを持つ |
| `action_observable` | 入力操作、観測場所（画面/API/file/process/DB）、結果field、状態遷移 | 「表示する」「適切に」「要求本文どおり」を単独値として拒否 |
| `data_contract` | 型、単位、値域、桁/丸め、時刻基準、並び、欠測、重複、世代 | fixtureから独立に期待値を再計算できる |
| `visual_contract` | 対象rect/軸/座標、塗り方向、色、文字、余白、所有者、重複禁止、許容誤差。非該当理由 | 画像名だけ、または`when applicable`だけを拒否 |
| `failure_contract` | 失敗入力/分類、表示・API・ログ結果、保持/消去、再試行、復旧操作 | カテゴリ共通のlast-good/旧payloadを未分解で継承しない |
| `persistence_contract` | 書込み対象、atomic境界、再起動/更新/削除/migration時の保持、backup | `N/A(reason)`以外の空欄を拒否 |
| `security_contract` | trust boundary、許可endpoint、秘密/redaction、入力サイズ・schema制限、権限 | 他要求の秘密/DB文言をコピーしない |
| `performance_contract` | 周期、上限、再入防止、停止・timeout時の挙動。非該当理由 | 数値なしの「bounded」「再入防止」を拒否 |
| `evidence_oracle` | 独立計算の期待値、fixture/raw入力、観測artifact、コマンド/プロセス/画像/DB、合否式 | `raw=ID`、`expected=<ラベル>`、`when applicable`だけを拒否 |
| `owner_dependency` | 実装owner、証拠owner、上流field/要求、下流consumer、同一世代境界 | `I/J→A`等のカテゴリ矢印だけを拒否 |
| `contract_status` | 行固有契約の構造判定 `PASS`/`FAIL`/`INCONCLUSIVE` | 欠落placeholderをPASSにしない |
| `evidence_status` | 製品証拠の判定 `PASS`/`FAIL`/`INCONCLUSIVE` と理由 | 構造FAILと証拠不足を混ぜない |

上記は見出しを増やすための提案ではなく、baseline §1.1 の12必須フィールドを失わずに、atomicの
カテゴリ共通6列を展開する最小単位である。`persistence/security/performance`を一つの共通否定文へ
畳み込むこと、または`data/visual`を一つの「正本field」に畳み込むことは許可しない。値が本当に
該当しない場合だけ、同じ行内に `N/A: <非該当理由>` を書く。

### 5.1 共通契約を参照する場合の制約

共通fixtureや共通安全契約は参照してよいが、各行に次をバインドする。

1. `contract_id@version` と、入力case ID・入力ハッシュ。
2. その行が読むfield、期待値、許容誤差、失敗時に保持するowner。
3. 適用セルと非適用セルの理由。
4. 画像/API/file/process/DBのどのartifactを観測し、どの独立計算で合否を出すか。

したがって `fixture=WIN-A-002`、`raw=WIN-A-002.jsonl`、`expected=状態fixture画像`だけでは不十分で、
少なくとも `expected.field/value/units/order/owner` と `negative.trigger/result/retained_state` が必要である。

### 5.2 再抽出後の受入ゲート

- 226件すべてで上記フィールドが値または根拠付きN/Aになっている。
- placeholder（`要求本文どおり`、`when applicable`、`fixture=ID`だけ、oracle名だけ）が0件。
- 直積セルが各行のcase IDへ割り当てられ、未割当セルが0件。
- 複合責務は分割され、旧IDとの対応がある。
- `contract_status`は構造を、`evidence_status`は実装証拠を別々に判定する。今回の2文書だけでは、
  行固有契約はFAIL、製品証拠はINCONCLUSIVEである。

## 結論と返却条件

IDの226件一致は確認できたが、atomic assertionsはbaselineの要求本文へfixture名と共通テンプレートを
付加した状態であり、226件を行固有の判定式へ変換できていない。したがって文書ゲートは**FAIL**、
製品実証は**INCONCLUSIVE**、抽出状態は**EXTRACTION_INCOMPLETE / HOLD**である。

再抽出では、まず上記スキーマでbaselineの各行を一対一に展開し、複合行は件数を増やして対応表を
残す。その後、各行の独立oracleと実証artifactを取得するまで、実装・テスト・ビルド・インストール・
画面評価へ進めない。

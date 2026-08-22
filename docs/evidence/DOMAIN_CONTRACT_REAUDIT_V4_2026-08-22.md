# Domain contract 再監査 V4（2026-08-22）

## 判定

**FAIL（行固有Normative Overrideの内容）**

**INCONCLUSIVE（製品証拠）**

最新の atomic assertion 文書には226件の Normative Override がある。しかし要求本文とIDを
差し替えただけで、カテゴリ共通の input、negative、dependency、evidence oracle が残る。
したがってV3で指摘した行固有性FAILは解消していない。製品コード・テスト・ビルド・実機は
実行していないため、製品証拠は別ゲートとしてINCONCLUSIVEに分離する。

## 構造確認

- 旧6列主表: 226 ID行、列数6。
- Normative Override: 226行、ID unique=226、重複・欠落なし。
- 各Override行に要求、input、observable、negative、dependency、oracleが存在する。
- baselineの要求ID集合との226件突合は一致。
- 抽出状態 EXTRACTION_INCOMPLETE は保持されている。

IDと列の存在はPASSだが、列が埋まっていることだけでは行固有契約のPASSにならない。

## 行固有性の正規化結果

ID、要求本文、fixture名、assert文字列などの識別子を除いてカテゴリ内で比較した。

| カテゴリ | 行数 | input unique | observable unique | negative unique | dependency unique | oracle unique |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A | 20 | 1 | 5 | 1 | 1 | 1 |
| B | 24 | 1 | 10 | 1 | 1 | 1 |
| C | 20 | 1 | 6 | 1 | 1 | 1 |
| D | 12 | 1 | 6 | 1 | 1 | 1 |
| E | 16 | 1 | 13 | 1 | 1 | 1 |
| F | 12 | 1 | 5 | 1 | 1 | 1 |
| G | 16 | 1 | 13 | 1 | 1 | 1 |
| H | 12 | 1 | 11 | 1 | 1 | 1 |
| I | 16 | 1 | 16 | 1 | 1 | 1 |
| J | 16 | 1 | 16 | 1 | 1 | 1 |
| K | 16 | 1 | 14 | 1 | 1 | 1 |
| L | 16 | 1 | 13 | 1 | 1 | 1 |
| M | 30 | 1 | 17 | 1 | 1 | 1 |

具体例:

- WIN-A-001〜020のinputは同一の actor/entry/action/boundary
  （Main/surface、named control、initial/auth/ready/error、700x480）。
- WIN-A-001〜020のnegativeは同一の missing/invalid upstream、last-good、派生値・秘密・
  duplicate禁止で、要求別の期間・認証・ゲージ・更新境界を持たない。
- WIN-A-001〜020のdependencyは同じ source=WIN-I/J、oracleも同じraw/expected/fresh
  image/process/SHA/reviewer構成である。
- WIN-B-001〜024は同じ reset_at<now、=now、>now、duplicate/missing minute 境界と同じ
  negative/dependency/oracleを使う。右端、左端、水平、段差、累積、系列順、軸書式の要求別
  操作順・閾値へ分解されていない。
- observableのassertは WIN-ID:target == expected_WIN-ID の形で、targetがA/B/C等のカテゴリ
  記号の行が多数ある。要求固有の対象field、単位、順序、境界値ではない。
- raw/expectedファイル名、fault-owner、surface-ownerは識別子であり、実際の期待値・失敗保持・
  依存関係・negative oracleを固有化しない。

よって、要求本文込みの文言は存在するが、要求本文をinput/observable/negative/dependency/
evidenceの具体的な境界・操作順・期待値へ分解したとは判定できない。残存FAILは226行の
5契約列に及ぶ。

## 製品証拠

製品証拠はINCONCLUSIVEである。

- 文書は全行の実証判定をINCONCLUSIVEとしている。
- fresh image、process/host/DB/API trace、artifact SHA、独立reviewerの取得結果はない。
- 本監査ではコード、テスト、ビルド、実機を実施していない。

これは文書構造FAILとは別の未検証ゲートであり、推測でPASSへ丸めない。

## 残存FAILと次の最小修正

カテゴリ共通のinput/negative/dependency/oracleを要求ごとの条件へ置き換え、observableの
カテゴリ記号を実対象fieldへ置き換える必要がある。各行に数値・時刻・viewport・状態・順序
などの境界、成功時のexact expected、失敗時の保持対象と復旧条件、実在依存ID、要求別oracle
を記録した後、新規独立Lunaで再監査する。

## 実行したread-only確認

wc -l、rgによる主表226行・Override226行の計数、Override ID unique計数、要求ID集合突合、
要求/input/observable/negative/dependency/oracle各キーの存在確認、静的パーサによる
カテゴリ内正規化unique数の計算を実行した。抽出状態および対象atomic文書は変更していない。


## J〜M具体契約の再監査（2026-08-22）

対象は [WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md](../atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md) の78行である。旧A〜Iの台帳・監査結果を置換せず、J〜Mの文書構造ゲートを独立に記録する。
固定値の照合元は [WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md](../WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md) と、そこから参照されるREST/DESIGN/UX/Data Protection正本である。

### ID集合・構造判定

- **ID集合 PASS**: `WIN-J-001..016`、`WIN-K-001..016`、`WIN-L-001..016`、`WIN-M-001..030` を抽出し、行数78、unique=78、欠落=0、余計なID=0。baseline全226 IDとの依存突合も一致。
- **10列 PASS**: 全78行が `ID + actor + entry + precondition + input + action + exact_expected + negative_retention + depends_on + independent_oracle` の10列（Markdownの`|`区切りではNF=12）で、空セル=0。
- **依存ID PASS**: `depends_on`から抽出したIDはbaselineに存在し、自己依存=0、未定義ID=0。
- **禁止語 PASS**: `case=case`、`satisfies`、`generic invalid matrix`、`fixture=` は0件。`fixture_only`はoracle入力であることを明示するための語としてのみ許可した。
- **固定値突合 PASS（静的）**: REST endpointは`127.0.0.1:8787`のみ、Main/Threadsは900×480 logical、Graphは940×640初期・700×480最小、daemonは既定60秒（設定5〜3600秒）、DB busy timeoutは2秒、backupは3世代、pruneは検証済みbackup成功後の3暦月境界、leaseはPID/process identityとlock file identityで検証する形に限定した。座標・monitor/DPIのfixture値は`fixture_only`またはDESIGN式参照とした。
- **横展開スキャン PASS（静的）**: `48123`、`8765`、`8000`、24時間だけのstale判定、発明したREST epoch/Failed row、token/expiry/401公開、Linux DB path/inode直接アクセス、Main/Threadsの700×480、固定Back/Close座標、HTTP pollへのdaemon 60秒混入は各0件。K-008の32 MiBと100,000 samplesは別上限として明記した。

### 行固有性の検査方法

次のread-only Python検査を行った。対象行をMarkdownの`|`で分割し、10列数、ID集合、各セル非空を確認した後、`depends_on`を正規表現で抽出してbaseline集合へjoinした。actor〜oracle各列についてraw unique数を算出し、さらにWIN ID、evidence名、`fixture_only`ラベルだけを正規化から除外して、要求ごとの入力・操作・期待値・保持・oracleの差分が残ることを確認した。entry/input/action/exact_expected/negative/independent_oracleは各78 unique、preconditionは77 unique（共通のsurface式を含むが、行内の状態・境界・保持条件は固有）、actorは65 uniqueである。依存列は既存IDの組み合わせとして全行の実在性・自己依存なしを確認した。

### 残存FAIL / INCONCLUSIVE

- **J〜M文書構造の残存FAIL: 0**（上記静的ゲートの範囲）。
- **製品証拠: 78/78 INCONCLUSIVE**。現行artifact SHAに結合したfresh image、Windows実プロセス、HTTP raw、SQLite/daemon/lease/backup/migration trace、host registry/install証拠、独立reviewer verdictは未取得であり、文書PASSを製品PASSへ昇格しない。
- **実装フェーズ: 未開始**。コード変更、テスト、ビルド、インストール、実機操作は行っていない。
- A〜Iを含む全226要求の既存監査判定（FAIL/HOLD/INCONCLUSIVE）はこの追補で解消したとは扱わず、元監査の残存状態を保持する。

### 製品証拠未取得の明記

この再監査は契約行のID・列・値根拠・行固有性を対象とする静的検査だけであり、製品の実証結果を含まない。実環境の証拠が追加されるまで、J〜Mの各行は`INCONCLUSIVE`のまま保持する。

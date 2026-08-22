# データ保護契約・Jカテゴリ独立監査（2026-08-22）

## 判定

**FAIL（契約の行固有トレーサビリティ未達）**。

実装が動作するという主張や、別文書のテスト結果をもって欠落した行契約を補完してはいない。
`WIN-J-001`〜`WIN-J-016` は、行の `independent_reviewer` 欄が全件
`INCONCLUSIVE (証拠未取得)` である（`docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md:159-174`）。
したがって、個別行を PASS と扱う根拠はない。以下で `FAIL` とした項目は契約の欠落、
`INCONCLUSIVE` とした項目は証拠または独立判定が不足していることを表す。

## 監査範囲と方法

read-only で次の文書を突合した。コード変更、テスト、ビルド、daemon 起動は実施していない。

- `docs/DATA_PROTECTION_POLICY.md`（特に §2、§3、§4、§5）
- `docs/REQUIREMENTS_LEDGER.md`（DP-001〜010）
- `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`（WIN-J-001〜016）
- `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md`
- `docs/TRACEABILITY_MATRIX.md`
- 証拠の同一性確認のため `docs/evidence/DATA_PROTECTION_RUNTIME.md` も参照した

J 行を機械的に数えた結果は `J_rows=16`、`independent_INCONCLUSIVE=16`、
`generic_precondition=16`、`generic_performance=16` であった。これは、全 J 行が
同じ「該当セルを明示」「周期・再入・CPU/メモリ上限を測定」というテンプレートを
持ち、具体的な値・境界・否定条件を行内に持っていないことを示す。

## 要求の突合結果

| 論点 | 該当J行 | 正本／台帳 | 判定 | 欠落と必要な修正 |
| --- | --- | --- | --- | --- |
| API read-only | J-001 | Policy §2.3, §2.4, §5（`DATA_PROTECTION_POLICY.md:37-38,74-79`） | **FAIL** | `/v1/details` とだけ書かれ、許可HTTPメソッド、拒否する書込みメソッド、入力・認証境界、DB row/file hash の前後不変条件がない。`oracle=concurrent-writer-recovery`（J-001）はread-only検査と対応しない。GET等の許可リスト、全拒否メソッドの期待status、DB/バックアップ不変条件をJ-001のfixture/oracle/evidenceへ記録する。 |
| `(reset_at,timestamp)` の一意性・二重登録 | J-004, J-005, J-012 | DP-005（`REQUIREMENTS_LEDGER.md:39`）、Policy §2.2（`DATA_PROTECTION_POLICY.md:36`） | **FAIL** | J-012は「複数server」「concurrent writer」としか書かず、J-004/005もキー、row count=1、remaining の COALESCE、cost/token の MAX、競合時の全batch rollbackを期待値にしていない。2つ以上のserver/client/daemonが同一キーを同時送信するfixtureと、重複0・各最大値・失敗時旧世代保持を行固有に記録する。 |
| daemonのUI独立性・複数daemon | J-010, J-011, J-013 | DP-008（`REQUIREMENTS_LEDGER.md:42`）、Policy §2.8（`DATA_PROTECTION_POLICY.md:42`） | **FAIL** | J-010は独立記録、J-013はsingleton leaseを宣言するが、別PID、同一DB、同時起動、stale lock、SIGKILL後再開、TERM時解放、二つ目が書込みを行わない結果がない。J-013にlock path/owner、同時2起動の1成功・1拒否、異常終了後の再取得、DB重複0を明記し、J-010/J-011とDP-008をIDで相互参照する。 |
| SQLite `busy` / `full` / `corrupt` / read-only | なし（カテゴリ既定値のみ） | Policy §2.3、§3（`DATA_PROTECTION_POLICY.md:37,55`）、§4（`DATA_PROTECTION_POLICY.md:63-68`）、DP-005/007（`REQUIREMENTS_LEDGER.md:39,41`） | **FAIL** | 行固有の入力・拒否動作・保持対象・bounded retry が存在しない。ROW_CONTRACTS末尾の I/J 行カテゴリ記述（`...:257`）は行固有契約の代替にならない。少なくとも既存J行を責務別に分割／拡張し、busy lock、容量不足、破損DB、filesystem read-only、schema mismatch を個別fixtureとして、transaction rollback、旧DB・旧memory・旧backup保持、次回要求への境界を記録する。 |
| transaction atomicity / 完全snapshot公開 | J-005, J-007, J-015 | Policy §2.3-4（`DATA_PROTECTION_POLICY.md:37-38`）、§3（`...:54,58`） | **FAIL** | J-005の `upsert test`、J-007の `restart trace`、J-015の `failure injection` は、途中commit、部分batch、publish失敗、旧完全snapshot復帰、未commit破棄を期待値としていない。入力全体を成功公開する前後のrow/hashと、失敗時に表示・REST・memoryが旧完全世代のままであることを独立fixtureにする。 |
| backup 3世代・prune順序 | J-006, J-014 | DP-006（`REQUIREMENTS_LEDGER.md:40`）、Policy §2.9、§4（`DATA_PROTECTION_POLICY.md:43,65-66`） | **FAIL** | J-014の `expected=backup inventory` は3世代の固定名、SQLite quick_check/reload、0600 file・0700 directory、各世代のrow/hash、backup失敗時 prune 禁止・原本不変を指定していない。J-006のDB hashもprune前後比較のタイミングと許可削除を定義していない。これらをfixtureと否定条件へ具体化する。 |
| migration / atomic switch / recovery | J-015, J-016 | DP-007（`REQUIREMENTS_LEDGER.md:41`）、Policy §2.10、§4（`DATA_PROTECTION_POLICY.md:44,67-68`） | **FAIL** | J-015は失敗時の旧DB保持だけで、旧schema拒否、candidate別名作成、全行の型・値・一意キー・quick_check・row count・fingerprint・reset-period境界、検証後だけのatomic switch、lock/switch失敗を列挙していない。J-016の「再生成しない」だけでも不十分。成功・検証不合格・switch失敗・復元失敗を別fixtureにし、旧DB/旧backup/candidateの保持と自動復元禁止を期待値にする。 |
| recovery / app-server・REST停止・daemon gap | J-007〜011 | DP-001/003/004/008（`REQUIREMENTS_LEDGER.md:35-38,42`）、Policy §2.1、§2.6-8、§3（`DATA_PROTECTION_POLICY.md:35,40-42,51-59`） | **FAIL** | J-008のlast-good、J-009のlocal history error、J-011のgapは部分的に該当するが、認証境界、persisted reset hint による bounded one-shot backfill、失敗cycleの部分結果拒否、collector停止中に未取得値を捏造しない条件が行固有でない。各J行のfixtureにDP ID、前世代、公開可否、再試行回数、次cycle条件を記録する。 |
| 負荷上限・再走査・再起動のboundedness | J-010, J-011, J-013（全J行の共通欄にも記載） | DP-004/008（`REQUIREMENTS_LEDGER.md:38,42`）、Policy §2.7-8（`DATA_PROTECTION_POLICY.md:41-42`）、TRACEABILITY_DESIGN §4（`...:80`） | **FAIL** | 「測定し無断入力なし」だけで、周期、one-shot latch、最大走査回数、retry/restart上限、CPU/メモリ上限、gap処理が数値化されていない。`DATA_PROTECTION_RUNTIME.md:89` の60秒・5〜3600秒clampは行契約へ逆引きされていないため代替証拠にならない。J-010/J-011/J-013へ数値・トリガー・上限・超過時の保持を追加する。 |
| 同一SHA証拠契約 | J-001〜016、WIN-L依存 | TRACEABILITY_DESIGN §5（`...:88-94`）、ROW_CONTRACTS evidence欄、TRACEABILITY_MATRIXのDB行 | **FAIL / INCONCLUSIVE** | J行は `process/db/hash manifest; artifact SHA同一` とだけ書き、manifest ID/path、source commit→published payload→installer→installed executable→raw log→独立監査のSHA連鎖、fresh process、取得時刻を指定していない。`DATA_PROTECTION_RUNTIME.md:55-62,98-103,121-151` はDB世代のSHAを示すが、release binary/source/監査のartifact SHAを結び付けていない。各J行のevidenceへ同一manifestを明記し、未連結の実動作証拠はINCONCLUSIVEのままとする。 |

## WIN-I（API/security）との境界

WIN-I は J のDB保護実装そのものではなく、loopback/SSH・REST入力・redactionの入口契約である。
したがって、I の契約欠落を J の実装失敗とは数えず、次の2層を分離した。

### I行固有契約の判定（FAIL）

`WIN-I-001`〜`WIN-I-016`（`WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md:143-158`）は、
要件名とfixture名はあるものの、全行の precondition が共通の「該当セルを明示」、
security/performance が共通の「入力サイズ/finite/range検証」「周期・再入・CPU/メモリ上限を測定」
である。TRACEABILITY_DESIGN §4 は、`安全に` には攻撃/破損入力・上限・拒否動作・last-good・
redaction、`確認する` にはfresh process・SHA・raw log・独立判定を必須としている
（`WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md:72-83`）。そのため次の欠落は契約FAILである。

| I行 | 契約上の不足 | 必要な修正 |
| --- | --- | --- |
| I-001〜I-005 | 固定loopbackの許可endpoint/port、SSH alias/host/user/port、redirect/cookie/proxy/decompressionの拒否結果、秘密情報を送受信しない境界が具体値・status・否定fixtureになっていない。 | 接続先、HTTP method/redirect方針、header/cookie/proxy制御、SSH tunnelの入力・失敗・終了状態をfixtureへ固定し、秘密情報の送信ログ・保存ログが0である期待値を記録する。J-001（API read-only）のmethod allowlistと同じ endpoint manifest を参照させる。 |
| I-006〜I-009 | response size上限の数値、超過時status・切断・last-good保持、schema version/required field/nullの扱い、unknown/duplicate keyの拒否statusとDB無変更がない。 | 最大byte数、許可schema、未知/重複時の拒否結果、入力前後のDB row/file hash、公開snapshotの世代を行固有oracleにする。I-009のAPI duplicate keyとJ-012のDB duplicate keyを別事実としてcrosswalkする。 |
| I-010〜I-013 | timestamp/percent/dollar/tokenの許容範囲、単位・境界、NaN/Infinity/overflow/null時の拒否または表示値、許可model集合の完全な期待値がない。 | 具体的な範囲・型・単位・境界値（min/mid/max/NaN/Infinity/overflow/unknown）とHTTP status、last-good保持をfixtureへ記録する。 |
| I-014〜I-015 | backend error、token/password/email/pathの具体的な入力fixture、表示・debug log・raw log・DBの各漏えい面、redaction後の期待文字列がない。 | 各ログ・レスポンス・画面面を列挙し、秘密文字列が0件、安定したredacted codeのみ、という否定oracleと保持境界を記録する。 |
| I-016 | `before/after hash` の語はあるが、invalid snapshotの種類、拒否status、公開しない完全世代、memory/REST/DBの比較対象がない。 | malformed/partial/schema-invalid snapshotごとに、旧完全snapshotのrow/hash・REST出力・表示世代が不変であることを明記する。 |

### Iの実装後証拠待ち（INCONCLUSIVE）

I-001〜I-016 の `independent_reviewer` は全件 `INCONCLUSIVE (証拠未取得)` であり、
Iカテゴリの実装が正しい／誤っているとは判定していない。現時点で参照できる
`DATA_PROTECTION_RUNTIME.md:82-90` は health endpoint と daemon のPID/lock/周期の記録であり、
Iのendpoint固定、redirect、cookie、proxy、schema、size、numeric domain、redactionの
個別raw traceではない。さらに同文書のDB SHA（`...:55-62,98-103,121-151`）だけでは、
I行のclient binary・source/payload・rawログ・独立監査を同一artifact SHAへ連結できない。
行固有契約を埋めた後、I各行について fresh process、raw request/response（秘密redaction済み）、
拒否status、DB/hash前後、同一manifest SHA、独立判定を取得するまで `INCONCLUSIVE` を維持する。

## 重要所見

### F-01（Blocker）行固有契約の不足

TRACEABILITY_DESIGN は、カテゴリ既定値の継承だけでは受入不可であり、`precondition`、
`failure_persistence_contract`、`security_performance_contract`、`test_oracle`、
`evidence` を観測値へ具体化するよう要求している（`WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md:67-86`）。
しかしJ-001〜016の全行で、precondition は「該当セルを明示」、性能欄は「測定」となっている。
SQLiteの4失敗分類、atomicity、retry上限などはカテゴリに書かれたままで、行のfixtureへ
降りていない。これはテスト結果以前に契約FAILである。

### F-02（Blocker）台帳状態とJ独立判定の橋がない

DP-005〜008は台帳上 `verified` だが、J-001〜016の独立判定は全て
`INCONCLUSIVE` である。一方 DP-001、DP-009、LIVE-001 は `HOLD` で、台帳の終了前チェック
も全ID verified・独立PASS・最新gateを未達としている（`REQUIREMENTS_LEDGER.md:35-58`）。
TRACEABILITY_DESIGN §3 は依存元が `HOLD`/`INCONCLUSIVE` の場合、依存先を verified に
変更してはならない（`...:56-65`）。DPとJの行をつなぐ crosswalk（DP ID、J ID、fixture、
evidence manifest、独立判定）がないため、DPの `verified` はJカテゴリの受入根拠にならない。

### F-03（High）証拠の名称だけでは逆引きできない

J行の `raw=WIN-J-NNN.jsonl` 等は予約名に留まり、保存場所、SHA、生成binary、取得日時、
独立評価者の判定記録がない。TRACEABILITY_DESIGN §5が要求する同一artifact SHAを満たす
manifestを行ごとに参照できるまで、実装者のruntime記述やDB file SHAだけをcanonical
acceptance evidenceにしてはならない。

## 最小修正タスク（親／Luna実装担当へ返す内容）

コードをこの監査で変更することはしない。契約を閉じるための最小作業は次の通り。

1. J-001〜016を台帳DP-001〜010へ行IDで crosswalk し、各行のfixture manifestに対象DP ID、
   preconditionの具体値、許可／拒否入力、期待status、保持する旧世代、retry回数、負荷上限、
   `not-applicable`理由を記録する。
2. J-012/J-004/J-005へ、複数server/client/daemonの同時同一キー、row count=1、
   `(reset_at,timestamp)`、remaining COALESCE、cost/token MAX、busy時の全batch rollbackを
   明記する。J-013へ singleton lock競合、stale lock、異常終了、再取得、二重書込み0を明記する。
3. SQLite busy/full/corrupt/read-only、transaction途中失敗、完全snapshot publish拒否を、
   既存行の責務分割または新しい原子要求IDとして追加し、Policy §2.3/§3/§4へ逆引きできる
   fixture・oracle・保持契約を持たせる。226行制約に触れる場合は、無根拠統合をせず旧IDとの
   分割／統合履歴を残す（TRACEABILITY_DESIGN §4）。
4. J-014/J-006へ3世代固定名、quick_check/reload、row/file hash、0600/0700、backup失敗時
   prune禁止を追加する。J-015/J-016へ旧schema拒否、candidate全行検証、fingerprint・期間境界、
   atomic switch、switch/restore失敗時のsource保持を追加する。
5. J-010/J-011へdaemonの周期・one-shot・最大走査／再起動回数・CPU/メモリ上限と、
   未取得gapを捏造しない期待値を数値で追加する。J-008/J-009へapp-server/REST停止、認証境界、
   backfill一回、失敗cycleの部分公開拒否を追加する。
6. J-001へHTTP method allowlistと全拒否メソッド、DB row/file hash前後不変を追加する。
7. `source commit → payload → installer → installed executable → raw log/DB → independent audit`
   を1つのmanifest ID／artifact SHAで連結し、各J行のevidenceへそのmanifestを記録する。値、
   取得日時、fresh process、独立判定を欠くものは `INCONCLUSIVE` として残す。
8. I-001〜016にも、I行の表で列挙したendpoint/tunnel、size/schema/domain、redaction、
   invalid snapshotの具体fixture・status・hash・secret=0 oracleを追加し、J-001/J-012との
   API/DB責務境界をcrosswalkする。
9. 上記修正後にのみ、独立した新規Luna評価で各I/J行を再判定する。現在の `verified`／`PASS` を
   欠落契約の代用にしない。

## 未検証事項

- 本監査では実装コード、テスト、build、data-protection gate、daemon、RESTを実行していない。
- DB backup/migration/runtime文書に記載されたPASS文字列は、行固有契約の欠落を埋める証拠として
  扱っていない。
- 全J行の独立判定が `INCONCLUSIVE` のため、契約修正後の同一SHAによる再評価が必要である。

## 追補（文書修正後）

上記FAILを受け、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md` にI/Jの32行を追加し、
endpoint/schema/domain、transaction、複数writer、daemon singleton、backup/migration、数値負荷上限、
保持境界、同一SHA証拠の行固有契約を割り当てた。これは要求契約の修正であり、runtime実証ではない。
新しい独立監査で32行を再突合するまで、総合状態は `FAIL / INCONCLUSIVE / HOLD` として維持する。

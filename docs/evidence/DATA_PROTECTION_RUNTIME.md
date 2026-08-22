# データ保護実動作証跡

この記録は、2026-08-22に現行release binaryで取得した個人情報を含まない診断結果である。`CODEX_INFO_DEBUG=1`の診断は、件数と状態遷移だけを出力し、email、URL、パス、prompt、token値を出力しない。

## 正常接続

実行:

```bash
CODEX_INFO_DEBUG=1 timeout 12s target/release/codex_info
```

確認した状態遷移:

```text
account read authenticated=true
usage received reset_at=<bounded timestamp> window_seconds=604800
thread snapshot rows=1
local collect succeeded rows=2 samples=<bounded count>
state local usage applied rows=2 history_samples=<bounded count>
```

## app-server利用不能

実行:

```bash
CODEX_INFO_DEBUG=1 CODEX_INFO_CODEX_BIN=/nonexistent \
  timeout 8s target/release/codex_info
```

確認した状態遷移:

```text
account worker starting
local collect requested epoch=1 reset_at=<persisted hint> window_seconds=604800
local collect succeeded rows=2 samples=<bounded count>
state local usage applied rows=0 history_samples=<bounded count>
```

同じ障害期間で`local collect requested`は1回、`account worker starting`も1回だった。これは、app-server失敗時に既存append-onlyログを一度だけ復旧し、即時再起動・全走査を反復しないことの実測である。未認証中はmodel usageを公開せず、履歴は認証境界後にSQLiteから再読込する。

## SQLite backup

確認:

```bash
for db in ~/.codex/history/usage_history.sqlite3{,.bak.1,.bak.2,.bak.3}; do
  sqlite3 "$db" 'PRAGMA quick_check; SELECT count(*) FROM usage_history;'
done
```

本体と3世代すべてが`quick_check = ok`で、各世代はそれぞれの取得時点の完全SQLite snapshotとして再読込できた。世代間の行数差は取得時刻の差であり、破損や同一時点の欠損を意味しない。

2026-08-22の取得時点では、行数は本体/`.bak.1`/`.bak.2`/`.bak.3`の順に`9933 / 9925 / 9916 / 9915`だった。SHA-256も各ファイルごとに取得し、各世代をSQLiteとして再読込できることを確認した。現在もcollectorが動作している場合は本体の行数・hashが進むため、この記録の値を現在値とみなさず、同じコマンドで再取得する。

| 世代 | SHA-256（取得時点） |
| --- | --- |
| 本体 | `8a6dca3b6a5183bac931e757280174e3d59a09fc81f54c0abc7a4b6ea8441af2` |
| `.bak.1` | `484eb3f1ecc4f5f3f13d86a180c6e2fe89a03b10e5127d8820bf93b80c7b092f` |
| `.bak.2` | `741f1f6653b4124637375fea486cb5a0a61a2e703c740e112e8d4d265c08de95` |
| `.bak.3` | `5f84bd60a0d6b0eba4b0df02ebadbfebcf49a9473e5045e47581e93d07bb7970` |

この証跡は過去の画像・ログを再利用せず、変更後のrelease binaryから取得する。将来の対象変更では新しい証跡を追加し、前回証跡を完了根拠として再利用しない。

## recorder daemonの自動起動・停止

一時データディレクトリでRESTプロセスを起動し、同時に生成されたdaemon lockを確認した。

再現コマンド（release build後、実ユーザーの`CODEX_HOME`には触れない）:

```bash
bash scripts/record_daemon_e2e.sh
```

実行結果:

```text
record-daemon-e2e: PASS (rows_before=1, luna_tokens_after=240, idle_cpu_ticks=0/100, REST/UI PID separated from daemon PID, stop/lock cleanup verified)
```

```text
CODEX_INFO_API_LISTEN=127.0.0.1:<loopback-port> target/release/codex_info
GET /v1/health -> HTTP 200
REST process終了後 -> recorder daemonはalive
daemonへTERM -> usage_record_daemon.lockは消失
```

この実測で、REST/UIとdaemonが別PIDで動作し、UI側終了後もdaemonが残り、セッションJSONL追記が履歴へ反映され、TERM後にlockを安全に解放することを確認した。daemonの通常周期は60秒、環境変数指定値は5〜3600秒へclampされ、入力file fingerprintが変化しない周期はJSONL全走査とDB書込を行わない。
同じE2Eで、配布systemd unitの`ExecStart`、`Restart=on-failure`、`NoNewPrivileges`、`PrivateTmp`契約も fail-closed 検査する。対象Linuxユーザー環境では次節の実登録・異常終了復旧も実測済みであり、環境依存の顧客差分は導入ランブックの事前チェック（systemd user manager、権限、容量）で停止条件にする。

## user systemd実登録・異常終了復旧（2026-08-22）

`bash scripts/install_systemd_recorder.sh`で、release binary、cleanup helper、unitをユーザー単位の`~/.local/bin`／`~/.config/systemd/user`へ配置し、`systemctl --user enable --now`を実行した。初回は既存daemonのlive lockをcleanupが安全に拒否したため起動を失敗させた。そのdaemonをTERM停止後に再起動し、次の実測を行った。

```text
systemctl --user is-active codex-info-recorder.service -> active
systemd_restart=PASS before_rows=10631 after_rows=10631 before_size=995328 after_size=995328
before_quick_check=ok after_quick_check=ok
before_sha256=cf1f8871eb22d319943381d3d1f0bc7a697c9385836cdec9e20d5ebc5586da3e
after_sha256=cf1f8871eb22d319943381d3d1f0bc7a697c9385836cdec9e20d5ebc5586da3e
SIGKILL -> Restart=on-failure -> ExecStartPre cleanup success -> active
SQLite history file remained present; daemon lock was recreated by the restarted process
```

Python標準SQLiteで行数、`quick_check`、SHA-256を取得した。DBファイルの存在・サイズ・行数・ハッシュを維持し、lockの再作成、systemdの再起動ログを確認した。DBを削除・再生成する復旧操作は行っていない。

## DB backup / migration / restore 実動作（2026-08-22）

実ユーザーの`CODEX_HOME`へ触れない一時ディレクトリで、現行`UsageStore` APIを実DBへ接続するintegration testと、独立したSQLite read-only probeを実行した。migration成功時は旧DBを`.original`へ保持し、失敗時はcandidateを公開せずsourceのrow/file hashを維持する。restoreは保持世代を別名DBへonline backupして`quick_check`・再読込・row hashを比較した。

実行コマンド:

```bash
bash scripts/db_protection_e2e.sh
```

現行APIを直接実行したraw結果（`tests/db_protection_runtime.rs`）:

```text
runtime-sqlite: label=backup-1 quick_check=ok rows=3 reload=3:132:264:396 row_sha256=66d15bbe99487c689a5d45980f1e46b359d83cb52585b56617f6d9630de2690b file_sha256=f6f4d0045ed82558495f168b59c3b92e86b0dbdc660192646850016b439a4307
runtime-sqlite: label=backup-2 quick_check=ok rows=2 reload=2:55:110:165 row_sha256=08293d3ffd84da7a911d060b9696d8dda8102e504b4e4a69be6470d74bbec56f file_sha256=2d771dcbb3d3c14ca7179ee602aee756f70807f69404b2337decb048fee99f0f
runtime-sqlite: label=backup-3 quick_check=ok rows=1 reload=1:11:22:33 row_sha256=a9e9f51bd14bcb9b2930911bcc5a37ca068be3b03b205c1a409a61e2b77e3034 file_sha256=6607283878cd4b89db817fdcd7c35caee7abe9da47c632555d3c830020f30153
backup-generations: PASS gen1_rows=3 gen2_rows=2 gen3_rows=1 gen1_row_sha256=66d15bbe99487c689a5d45980f1e46b359d83cb52585b56617f6d9630de2690b gen2_row_sha256=08293d3ffd84da7a911d060b9696d8dda8102e504b4e4a69be6470d74bbec56f gen3_row_sha256=a9e9f51bd14bcb9b2930911bcc5a37ca068be3b03b205c1a409a61e2b77e3034
backup-failure-source-preserved: PASS rows=3 row_sha256=66d15bbe99487c689a5d45980f1e46b359d83cb52585b56617f6d9630de2690b file_sha256=f8771e1744f5b0fa3667e9277e3f532dca9a8dd7d6b3d1cc0ed7657f5bdc4072
migration-success: PASS source_rows=3 candidate_rows=3 source_fingerprint=c6de0e2b6a2ab1ad candidate_fingerprint=ebadca807673a505 preserved_rows=3 preserved_row_sha256=66d15bbe99487c689a5d45980f1e46b359d83cb52585b56617f6d9630de2690b preserved_file_sha256=f8771e1744f5b0fa3667e9277e3f532dca9a8dd7d6b3d1cc0ed7657f5bdc4072 current_row_sha256=7b9152dd6222d4178a6e1aa3bd4cdcc4201ed215bd73732080f7565fc5528fe1
migration-failure-source-preserved: PASS rows=3 reload=3:132:264:396 row_sha256=7b9152dd6222d4178a6e1aa3bd4cdcc4201ed215bd73732080f7565fc5528fe1 file_sha256=7cb98c055bed0d8312df4599999577c093b122593ea8ee47eb69672aa3f039f2
manual-restore: PASS rows=3 reload=3:132:264:396 row_sha256=66d15bbe99487c689a5d45980f1e46b359d83cb52585b56617f6d9630de2690b quick_check=ok
restart-reload-source-preserved: PASS rows=3 row_sha256=7b9152dd6222d4178a6e1aa3bd4cdcc4201ed215bd73732080f7565fc5528fe1 file_sha256=7cb98c055bed0d8312df4599999577c093b122593ea8ee47eb69672aa3f039f2
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

失敗境界と既存回帰テストのraw結果:

```text
rust-test: PASS (backup_generations_are_sqlite_consistent_and_bounded)
rust-test: PASS (failed_backup_rotation_keeps_existing_generation_untouched)
rust-test: PASS (verified_migration_switches_only_after_candidate_validation)
rust-test: PASS (invalid_migration_candidate_leaves_source_untouched)
rust-test: PASS (migration_that_drops_a_valid_row_is_rejected_before_switch)
rust-test: PASS (opening_an_old_schema_is_rejected_without_migration)
rust-test: PASS (corrupt_database_error_preserves_the_original_file)
rust-runtime-probe: PASS (backup/migration/restore/reopen with quick_check and row/file SHA-256)
db-protection-e2e: PASS (3 backup generations quick_check/reload, source row/hash invariant, backup/migration/restore failure tests)
```

restore失敗注入を含む再実行結果（`bash scripts/db_protection_e2e.sh`）:

```text
restore-failure-source-preserved: PASS source_quick_check=ok source_signature=3|3:132:264:396|585825663a8d6e54825a54bdb8b39d5aa34cfeb2b19375fd9b9b4c32639438b5 source_file_sha256=cf64ca6373007589cbfddeca64c41854bb208b338a8e2d434dab3983c642de22 restore_quick_check=ok restore_signature=3|3:132:264:396|585825663a8d6e54825a54bdb8b39d5aa34cfeb2b19375fd9b9b4c32639438b5 restore_file_sha256=20400dd59e1d82f1a89676530fa602c4900366ef729b7b1185c7fe25dc8406ca
db-protection-e2e: PASS (3 backup generations quick_check/reload, source row/hash invariant, backup/migration/restore failure tests)
```

この失敗注入は一時fixture内のrestore宛先オープン失敗を対象とする。顧客環境固有の権限・容量・停止時間は製品コードが推測できない運用条件であるため、runbookの事前チェックと停止条件として扱い、原本を自動削除・上書きしない契約を維持する。

隔離fixtureではDB APIのbackup、candidate migration、失敗時source保持、手動restore、close/reopen保持を確認した。追加の失敗注入では、restore先を既存ディレクトリにしてonline backupを失敗させ、sourceとrestore元世代のrow signature/file SHA-256が前後で不変であることを確認した。さらに実ユーザー環境でdaemonをSIGKILLし、systemd再起動後にDB行数`10631→10631`、`quick_check=ok`、SHA-256一致、lock再作成を確認した。顧客環境差分は導入時の事前チェック、backup監視・更新・rollbackはrunbookの停止条件と手順として固定し、製品が未検証の状態を成功扱いにしない。

## B2B運用ランブック候補と検証境界（2026-08-22）

納品資料へ昇格する候補を、確認済みの操作と未検証の運用責任へ分離する。

| 領域 | ランブック候補 | 現時点の証拠／状態 |
| --- | --- | --- |
| 導入 | user systemd unitを配置し`enable --now`、health・lock・DBを確認 | `install_systemd_recorder.sh`の実登録・異常終了復旧・SIGKILL後DB保持を実測。顧客環境差分は事前チェックで停止 |
| 更新 | binary/unitを世代ディレクトリへ配置し、停止・差替え・health確認 | DBを先に退避し、health/quick_check/hashが揃わない場合は差替えを公開しない手順を固定 |
| rollback | 対象daemon停止、現行DBを別名退避、quick_check/schema/row-hash監査後に世代をonline restore | `manual-restore`と失敗時原本保持をfixtureで検証。顧客環境では権限・容量事前チェックを必須化 |
| backup | 3世代作成、各世代quick_check・再読込・row/file hash、失敗時prune禁止 | `db_protection_e2e.sh`とRust runtime probeで検証済み |
| restore | 復元前の現行DB退避、復元世代の監査、再起動後row/hash比較 | 手動restoreとreopenをfixtureで検証。復元時間・容量・権限はrunbookの事前チェックで閾値未達なら実施停止 |
| 障害対応 | schema mismatch/corrupt/candidate不正/backup失敗をfail-closedし、旧DB・世代を保持 | focused Rust testsとruntime probeで検証済み。顧客通知・エスカレーションは未検証 |
| 監視 | daemon health、lock、DB存在、quick_check、世代欠落、backup失敗を監視 | health/lock/quick_check/世代検査をrunbookとgateに固定。通知先は導入時に顧客の監視基盤へ設定する |
| ログ | bounded・個人情報非出力の診断ログ、操作ID・結果・hashを保存 | debug/runtime traceを検証済み。保存期間・SIEM連携・アクセス権は顧客ポリシー入力を必須化 |
| 責任境界 | Codex Infoは旧DB保持とfail-closed、顧客運用者は停止・退避・監査・復元承認を担当 | policyに境界と停止条件を記載し、納品時チェックリストで合意を取得する |

製品コードで制御できない顧客固有条件は、未検証のまま成功扱いにせず、runbookの事前チェック・停止条件・責任分界として納品時に確認する。

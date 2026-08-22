# Codex Info データ保護規約

この文書は、利用履歴・ローカルセッションログ・thread情報・SQLiteデータベースを変更する全実装の正本である。`DESIGN.md`の補足ではなく、変更を許可するための拘束条件として扱う。

## 1. 目的と適用範囲

対象は次のデータフローである。

```text
Codex app-server / session JSONL / thread rollout
  -> bounded validation
  -> immutable cycle snapshot
  -> transactional SQLite upsert
  -> reload/graph/REST/Windows presentation
```

対象ファイルを変更する場合は、実装だけでなく[要求台帳](REQUIREMENTS_LEDGER.md)と受入証拠を同じ変更で更新する。

対象ファイル（直接変更・間接変更を含む）:

- `src/main.rs`
- `src/thread_contract.rs`
- `src/usage_store.rs`
- `src/server.rs`
- `DESIGN.md`
- `docs/REST_API_V1.md`
- `docs/WINDOWS_CLIENT.md`
- `windows-client/`
- `run.sh`

## 2. 絶対不変条件

以下を満たせない変更は不合格とし、fallback値・ゼロ値・空DBで通過させない。

1. 既存の有効なDB行を、収集失敗・認証失敗・通信切断・UI終了・migration失敗で削除、上書き、推測変換しない。
2. canonical DBのusage rowは`(partition_id, reset_at, timestamp)`で一意である。`partition_id`は
   `ProfileScopeId`、`AccountScopeId`、`StorageEpoch`に結合し、`timestamp`は有効なUTC event秒を
   `floor(event_epoch / 60) * 60`へ変換したminute-startであり、同一キーの再計測は行を増やさず、
   残量はcanonical順序の最後の有効値、累積cost/tokenは列ごとの最大既知値を保持する。元event秒は
   同一minuteのcanonical順序を決めるためだけに使い、REST/DBのtimestampへ書き戻さない。
3. DB書き込みはtransaction内だけで行う。busy、I/O、full、corrupt、schema不一致、migration中断はrollbackし、旧DBと旧メモリ世代を保持する。
4. 有効な完全snapshotだけを公開する。`SnapshotPublisher`のpublish admissionは現行の
   `(ProfileScopeId, AccountScopeId, StorageEpoch, SupervisorLeaseIdentity, CollectorEpoch, CycleSeq)` tupleだけを正本とし、candidateの6要素が全て現行値と一致する場合だけDB、memory、REST、UIへ進める。
   stale lease/epoch/cycleまたはtuple欠落・不一致はcandidateを破棄し、DB、memory、REST、UIを0変更とする。部分的な履歴、thread、model usage、REST応答を成功値として公開しない。
5. local usage JSONLとlive rolloutを同じrecord隔離規則へ丸めない。live rolloutではUTF-8、JSON、
   envelope、event kind、task-stateへの非影響を完全検証できない改行済みrecordを含むcycleはfail-closedにする。
   oversize recordを隔離できるのは、bounded streaming parserがduplicate/unknown envelope key 0の正規eventを
   完全検証し、livenessを変更しないtool payloadであると証明した場合だけである。証明不能、known state eventの
   型不正、invalid UTF-8/JSONは旧完全thread snapshot＋未確認を保持し、古い`task_started`をrunningへ流用しない。
   live rolloutのEOF直前の未改行tail（同一inodeへの追記待ち）だけは次cycleへ保留し、途中状態を公開しない。
   local usage側の改行済み不正record隔離は、後続のvalidated cumulative snapshotで対象列の欠落を覆えるcaseだけを
   許可し、後続snapshotなし・usage eventか判定不能・EOF以外の部分行・I/O・file差替え・資源上限はfile/candidate
   単位でrollbackする。local readerがEOFで検出した未完了recordはvalid/invalid UTF-8・oversizeを問わずrollbackする。
6. app-server停止中でも、Codex Infoプロセスが動作し、`history/usage_reset_hint.json`とcanonical sessions root配下のappend-onlyログが存在する場合だけ、
   outage epochにつき1回のbounded one-shot backfillを許可する。hintはschema `reset-hint-v1`、UTF-8 JSON、最大4KiBとし、
   `state`（`active`/`expired`/`tombstoned`）、`reset_at`、`window_seconds`、`observed_at`、file cursor、
   個人識別値を含まないopaqueな`auth_epoch_nonce`を保持する。
   現在の認証epoch・nonceに束縛されたauthenticated hintだけを受理し、未認証中の復旧値は公開しない。
7. 同一障害中にlocal JSONL全走査やapp-server再起動を無限反復しない。通常の全走査はquota cycle、明示更新、または一度だけの障害復旧に限定する。
   fingerprint不変ならscan/writeは0、変化時も1 cycleにつき1 scan・1 transactionだけとする。
8. collector全停止中の未取得データは後から捏造しない。常駐daemonは、Slint/X11/Wayland runtime dependencyを持たない
   headless release binary `codex-info-server`の`record --interval 60`を`codex-info-recorder.service`が所有する
   UI/REST独立の記録プロセスであり、同じcanonical DB profileの`UsageStore`、singleton lease、停止・更新・監視・再起動手順を実装してから有効化する。
   `codex-info-server.target`が`codex-info-recorder.service`と`codex-info-api.service`を束ね、systemd unitsがinstalledなprofileでは
   systemdだけがsupervisor/MaintenanceOwnerであり、UI/RESTがrecorderをspawnしない。units未使用時のfallbackは明示的な
   `codex-info-server record --interval 60`だけがleaseを取得し、後続起動はno-opとなる。
   daemon実装前の状態を実装済みと扱わない。binary installer、unit installation、update、rollback、uninstallの
   要求順序・保持境界は`docs/CUSTOMER_OPERATIONS_RUNBOOK.md`を正本とするが、製品実装・同一release証拠・独立判定が
   未取得の間は`PRODUCT_PENDING`であり、この規約から別commandを推測して補わない。
9. canonical DB profileごとに`MaintenanceOwner`を1つだけ許可する。起動時pruneの前に、writer admissionを止めた同一排他境界で
   SQLite online backupを3世代作成・検証する。backup失敗、検証失敗、writer競合時はpruneを実行しない。バックアップは`0600`、DBディレクトリは`0700`とする。
10. 現行版は旧schemaを暗黙migrationしない。schema mismatchはread/writeを拒否する。将来migrationは別名DB、全行validate、件数/hash/期間境界比較、
   3世代backup保持、検証後のatomic switchの順序だけを許可する。candidate失敗時はDB、backup、memory、公開rootを旧世代のまま保持する。
11. スレッドのライブ状態はDB履歴を根拠に再生しない。root/childとも、同一cycleで前後identityを検証した
    eligible Codex workloadの`canonical path -> nonempty ProcessIdentity set`にrolloutが存在し、最後のtask状態が
    runningである場合だけnative candidateにできる。`ProcessIdentity=(pid,starttime_ticks,exe_device,exe_inode)`とし、
    exact Codex Info artifactを祖先に持つ観測用app-server childは全Codex Info process分を除外する。
    identity変化、祖先不明、FD scan部分失敗、native DBのduplicate/root非到達/cycle/dangling/partialはcycle全体を
    fail-closedにし、旧完全snapshot＋未確認を保持する。完全受理済みREST PublicThread集合内のmissing parentだけは
    presentation orphanでありnative DB danglingの救済ではない。詳細は`docs/LIVE_STATE_DECISION_MATRIX.md`を正本とする。
12. Windows clientの設定永続化は`language`、`setupCompleted`、`connectionConfigured`、`timeZoneId`、
    `connectionProfile`、`connectionSelector`の6 keyだけを許可する。`connectionProfile`は`none|wsl|sshConfigAlias`のexact enum、
    `connectionSelector`はWSLのexact distribution tokenまたはliteral OpenSSH Host alias grammarだけを許可し、秘密、展開済み値、raw host/user/pathを0件とする。
    saved selectorによるauto reconnectはこの6 keyを再検証して行い、remote自動起動は`ArgumentList`と`BatchMode=yes`を使う。
    auth argvもsaved profileから構築するが、起動成功とstatus再確認を別stateに分ける。4-key recoveryはMain disconnectedとSettingsだけに残し、
    設定不正・接続失敗時は保存値とDBを破壊せずSettings recoveryへ戻す。この設定経路の製品判定は`PRODUCT_PENDING`であり、RESTへselector/secretを送らない。

## 3. 失敗時の保持契約

| 障害 | 保持するもの | 破棄するもの | 再試行 |
| --- | --- | --- | --- |
| app-server/REST停止 | 既存のquota、履歴、thread、DB | 未取得の新規値 | 次の明示/周期要求。local backfillは障害期間1回 |
| daemon unexpected exit / restart budget超過 | 直前の完全snapshot、DB、hint、source cursor、確定gap ledger | 停止区間の推測値、未検証の再起動candidate | supervisor検知後2秒以内に最大1回、以後はFailed latch。明示startまたはsystemd再activation |
| oversized/不正な1レコード | 同一ファイル内の前後の有効レコード | そのレコード | 次のcycleで再読込可能 |
| ローカル履歴のEOF未完了レコード | 直前の完全snapshot、DB | 不完全レコードを含むローカル入力 | 次のcycle |
| I/O、差替え、EOF以外の部分行、資源上限 | 直前の完全snapshot、DB | 失敗cycleの部分結果 | 次のcycle |
| SQLite busy | 旧DB、旧backup、旧メモリ/root | 2.000秒でrollbackした未commit transaction | same-callback/polling retry 0。次の通常scheduled cycleまたは明示操作で最大1 attempt、同じ2.000秒deadline |
| SQLite full/corrupt | 旧DB、旧backup、旧メモリ/root | 未commit transaction/candidate | same-callback retry 0。容量・明示restore等の原因解消後だけ新cycle |
| SQLite open/read/write I/O | 旧DB、旧backup、旧memory/root | 失敗batch、partial candidate | 同一callbackでは再試行せず次cycleまたは明示操作 |
| schema mismatch/migration失敗 | 旧DB、旧backup世代 | 新DB候補 | migrationを修正して別途再実行 |
| backup/rotation/restore失敗 | 現行DB、検証済み3世代、旧memory/root | partial backup/candidate、未検証世代 | 次のmaintenanceまたは明示restore。自動復元0 |
| confirmed daemon stop gap | 前後のvalid sample、gap ledger、旧完全root | gap区間の補間値・複製値 | source cursor確認後にのみ確定。確定後は次の実sampleで再開 |
| status/details pair不一致・stale admission tuple | 直前の完全status/details pair、現行`(ProfileScopeId, AccountScopeId, StorageEpoch, SupervisorLeaseIdentity, CollectorEpoch, CycleSeq)` | 片側だけ進んだcandidate、stale lease/epoch/cycle candidate | 次cycleで現行tupleを再取得・検証し、DB/memory/REST/UIは不一致中0変更 |
| 認証喪失・アカウント切替 | 非認証root（旧account表示は消去）と非破壊DB | 旧accountの画面公開値 | 認証成功後に再読込 |
| reset hint expired / auth epoch切替 | source log、旧DB、旧root、tombstone hint | `now >= reset_at`の旧期間scan/row、次期間への誤帰属、旧epochの公開値 | 旧hintをexpiredまたはtombstonedとして無効化し、source logを保持。current epochのfresh authenticated hint後だけ次のbounded one-shot |
| UI/REST publish失敗 | DBのcommit済み世代、旧表示snapshot | 失敗した公開試行 | 次のroot更新または明示操作 |
| thread DB履歴とlive pathの不一致 | 旧完全thread snapshot、quota、履歴、DB | DBだけから復元した停止済みroot/child | 次のcycleでactive pathとterminal状態を再検証 |

## 4. 同時実行とバックアップ

- すべてのcollectorは`UsageStore`を通してSQLiteへ書く。直接JSON、直接SQL、別形式の履歴DBは禁止する。
- SQLite transaction lockとbounded busy timeoutを正本とする。ロックを無視した上書き、DB削除、DB再生成は禁止する。
- `usage_history.sqlite3.bak.1`〜`.bak.3`は時系列の完全SQLite snapshotであり、同じ件数である必要はない。各世代は`PRAGMA quick_check`と再読込で検証する。
- backup、prune、migrationの失敗は元DBを変更しない。pruneはbackup成功後だけ許可する。
- migrationは`UsageStore::migrate_verified`を入口とし、候補DBを別名で作成して全行の型・値・一意キー、`quick_check`、row count、決定的fingerprint、reset-period境界を検証する。検証後だけ元DBを退避してcandidateをatomic switchし、旧DBと3世代backupを残す。candidate検証失敗・switch失敗・lock競合は元DBをそのまま保持する。
- backup世代の復元は、対象プロセスを停止し、現在DBを別名退避してから、quick check・schema check・row/hash監査を通した世代だけで行う。通常起動が自動復元を試みてはならない。

### 4.1 RecorderSupervisor、lease、backfill、gap

- supervisorの状態は`Absent → Starting → Running → StopRequested → Stopped`または`Failed`だけを進む。
  unexpected exitはsupervisorが2秒以内に検知し、同一supervisor epochでは5秒backoff後の自動restartを1回だけ許可する。
  2回目のunexpected exitまたはrestart失敗後は`Failed`へlatchし、無限restartを行わない。明示startまたはsystemdの新activationだけが新epochを開始する。
- `codex-info-server.target`、`codex-info-recorder.service`、`codex-info-api.service`がinstalledならsystemdが唯一のsupervisor/MaintenanceOwnerであり、
  UI、REST、`run.sh`、個別のrecorder起動はrecorderをspawnせず既存leaseを検出してno-opになる。API serviceの起動はheadless
  `codex-info-server serve --listen 127.0.0.1:8787`、recorder serviceの起動は`codex-info-server record --interval 60`である。
  systemd未使用時は明示的なrecord commandだけがcanonical DB profileのleaseを取得する。UI/REST workerはrecorder ownerではなく、
  UI/REST終了はrecorderを停止しない。explicit stopだけがTERM、drain、lease解放を順に行う。
- singletonのscopeは正規化canonical DB pathとprofileの組である。lease schemaは最大4KiBのUTF-8 JSON
  `recorder-lease-v1`（`pid`、`process_start`、`owner_nonce`、`canonical_db_path`、`device_or_volume_serial`、`file_index_or_inode`）とし、
  writer processは同じ`UsageStore`のtransaction/upsert契約を使う。通常のrecorder二重起動はlease前にno-op、競合試験だけが別の許可済みwriter processを使い、
  製品経路でleaseを無効化しない。
- stale lockはPID不存在またはprocess-start不一致を必要とし、削除直前に同じpathをreopenして取得時のfile identityと再比較する。
  24時間は診断用の経過時間であり削除条件ではない。年齢だけの削除、別ownerの削除、path/identity不一致の削除は常に0件とする。
- fingerprintはcanonical sessions root配下のregular・non-symlink JSONLだけを相対path辞書順で並べ、各fileのdevice/inode、size、mtime_ns、
  最後の完全行offset、最後の完全行SHA-256をLF区切りcanonical bytesへ連結してSHA-256化する。appendはcursor以降、rotation/truncationは同一fileのcursorを捨てて1回だけ再検査する。
  fingerprint不変のcycleはJSONL全走査・DB write・retryを0とする。
- app-server outage epochでpersisted `history/usage_reset_hint.json`とappend-only sourceが両方有効なときだけ、backfill latchを1回消費する。
  hintは`active → expired`または`active → tombstoned`へ一方向に進む。`now >= reset_at`になった旧hintはexpiredとして扱い、
  新規source変更を理由に旧期間のscan/DB row作成をせず、source logだけを保持する。quota cycleまたは明示更新が別に起動しても、
  current authenticated reset periodを検証できない限り旧期間へ帰属させない。current AuthEpochに束縛された
  fresh authenticated hint（`reset_at > now`、同じcurrent source identity、旧hintでない）が受理された後だけ、新期間へ帰属する
  bounded one-shot backfillを開始する。hint、cursor、source identity、AuthEpoch/nonceが不一致ならcandidate全体を破棄し、gapを埋めず旧rootを保持する。
  logout、token失効、AccountKey変更ではAuthEpochを先に増やし、persisted hintを`state=tombstoned`へatomicに更新してcursorと公開候補を無効化する。
  hint/DBへemail、account ID、その他の個人識別値を保存せず、opaque nonceとprocess内AuthEpochだけでepoch境界を検証する。
- daemon停止区間は`RecorderGapLedger`がsource cursorと停止・再開monotonic時刻から回収不能を確定した場合だけgapとする。
  v1 wire schemaへgap fieldを追加せず、history sampleのminute-start timestampの不連続をpresentation-owned markerとして表示する。
  確定gapは補間、残量推測、旧値複製の対象外であり、backfillが成功した区間だけ実sampleで置換する。

### 4.2 Maintenance、backup、restore

- canonical DB profileごとに`MaintenanceOwner`を1つだけ許可する。maintenanceはwriter admissionを止めた同じ排他境界で
  `online backup candidate → flush → quick_check/schema/row count/deterministic fingerprint/reset-period境界検証 → verified rotation → prune transaction`
  の順に進む。検証前のcandidate、writer競合、検証失敗ではpruneは0件で、現行DB・旧memory・既存backupを変更しない。
- backup名の世代順は`.bak.1=最新verified`、`.bak.2=次に新しいverified`、`.bak.3=最古verified`とする。
  1 maintenance activationで追加できる検証済み新世代は最大1件で、0→1→2→3 activationと実時間順に蓄積する。
  初回に同一snapshotを複製して不足数を埋めず、未検証・欠損・破損世代を3世代へ数えない。
  candidate fileをflush/fsyncし、parent directoryをfsyncした後、owner-only `backup-rotation-v1` journalへ
  old rank/path/inode/hash、candidate hash、各rename phaseをflush/fsyncしてからrenameする。各rename後もdirectoryを
  fsyncし、全rankとhashを再検証してjournalをcommit/removeする。crash/restart時はjournalとhashから完全rollbackまたは
  roll-forwardを一意に行い、回復完了前のwriter、prune、publishは0とする。
- restoreは通常起動から自動実行しない。明示restoreでは全writer/API/UIを停止して確認し、現DBを削除せず別名退避し、最新の完全verified世代からquick_check/schema/row/hash/period監査を通したものだけを同一filesystemへatomic replaceする。
  reloadとREST/UIのpair検証まで成功する前に旧DB・全backup・old memoryを破棄せず、どの段階の失敗でも旧世代を復元可能なまま保持する。

### 4.3 Migration 3経路とSQLite fault matrix

- **old-schema startup reject**: 現行schemaでないDBはread/writeを拒否し、旧DB、旧backup、old memory/rootをそのまま保持する。暗黙変換や空DB置換はしない。
- **candidate migration success**: `UsageStore::migrate_verified`が別名candidateをtransactionで作り、全rowの型・値・`(partition_id,reset_at,timestamp)`一意性、quick_check、row count、deterministic fingerprint、partition/reset-period境界を比較する。writer/API/UI停止後、旧DB/candidateをflush/fsyncしparent directoryをfsyncする。owner-only `migration-switch-v1` journalへold/candidate/current path・inode・hash・phaseをflush/fsyncし、各renameとdirectory fsyncを記録する。再読込/pair検証後だけjournalを`committed`へ進め、DataGenerationを1回だけ増やす。terminal journalの削除は別のretention処理で行い、commit成立条件へ混ぜない。
- **candidate validation/switch/crash failure**: candidate、lock競合、backup、rename、fsync、再読込、pair検証のいずれかが失敗した場合、または再起動時に未完了journalがある場合は、journalと実path/inode/hashから完全rollbackまたはroll-forwardを一意に選ぶ。current path不在、current DB二重、空DB自動作成を許さず、回復完了までwriter/publish=0、旧DB/backup/memory/rootを保持し、同callbackで再試行しない。

`migration-switch-v1`はowner-only 0600、UTF-8 JSON、64 KiB以下とし、exact key集合を
`schema_version,operation_id,operation_generation,owner_identity,phase,current_identity,current_sha256,
candidate_identity,candidate_sha256,quarantine_identity,quarantine_sha256,parent_data_generation,
result_data_generation_or_null,created_at_utc,updated_at_utc`に固定する。phaseは
`admission_closed,backup_verified,candidate_validated,switch_intent,current_quarantined,candidate_published,
pair_checked,committed,rollback_required,rolled_back`だけである。`pre_switch_crash,source_lock,candidate_lock,
rename_failure,post_intent_pre_commit_crash,validation_failure`を独立interruptとして扱い、verified `committed`前は
旧DBを唯一のcurrentとして保持する。同じoperation ID/generationの再入はresumeまたはno-opであり、新backup、rename、
DataGeneration、pair publicationを二重化しない。foreign/第二operationはBusyかつmutation 0である。

| fault | bounded action | retention / next state |
| --- | --- | --- |
| SQLite busy | 1 attemptのbusy deadline=2.000秒。deadline到達時はbatch全体rollbackし、same-callback/application polling retry=0。次の通常scheduled cycleまたは明示操作だけが最大1 attemptを開始し、そのattemptも2.000秒 | 1 cycle attempt=1、partial row/duplicate=0、旧DB/backup/memory/root保持。再BUSYは全rollbackし新cycleまで待つ |
| full / open-read-write I/O | transactionまたはcandidate単位でrollback。same callback retry=0 | 旧DB/backup/memory/root保持、次cycleまたは明示操作 |
| read-only filesystem / permission | writer admissionまたはcandidateを拒否し、same callback retry=0 | 旧DB/backup/memory/root保持、明示操作または権限回復後の次cycle |
| corrupt / quick_check failure | corrupt DBを読取成功扱いせず、candidate公開・自動再生成をしない | 旧DB/backup/memory/root保持、明示restore/migrationへ |
| schema mismatch / migration lock | old-schema rejectまたはcandidate switch=0 | 旧DB/backup/memory/root保持、migration修正後に別epoch |
| backup validation/rotation failure | partial candidateをpublishせずprune=0 | 現行DBと検証済みbackup 3世代保持 |

### 4.4 有限入力・snapshot境界

| resource | canonical bound / counting point | failure boundary |
| --- | --- | --- |
| reset hint | 4KiB、UTF-8 JSON bytes、hint path=`history/usage_reset_hint.json` | schema/size超過、expired/tombstoned hint、current AuthEpoch/nonce不一致はhint scan・backfill writeを拒否。source logは保持 |
| recorder lease | 4KiB、UTF-8 JSON bytes、`recorder-lease-v1` | schema/size超過、PID/process-start/file identity不一致はlease取得・stale reclaimを拒否 |
| local JSONL record / session file / session total | line 4MiB / file 256MiB / aggregate 2GiB、decode前の受信bytes | oversize/invalid record隔離は後続validated cumulative snapshotで欠落を覆えるcaseだけ。その他とfile/total超過はfile/candidate rollback |
| live rollout record / file / active paths | line payload 4MiB / file 256MiB / active path 1024、stream受信bytesとProcessIdentity前後値 | oversizeはstreaming envelopeでliveness非変更を完全証明した場合だけpayload隔離。invalid UTF-8/JSON/envelope/state event、identity/ancestry/FD partialはlive cycle全rollback |
| internal validated snapshot | canonical JSON 1MiB | candidate全体をrejectし旧snapshot保持。REST transfer bodyとは別resource |
| REST response headers / status body / details body | 8KiB / 64KiB / 32MiB、transfer後・decode前 | Content-Lengthは事前拒否、streamは最初の超過byteで停止 |
| transaction batch / retry | 最大1024 rowsかつ1MiB、backfill latch=1、scan/restart retry=1 | 上限到達はpartial公開せず次cycleまたは明示操作 |

## 5. 変更管理の制約

データ保護対象ファイルの変更者は、次を満たさない限り完了を宣言してはならない。

1. [要求台帳](REQUIREMENTS_LEDGER.md)の該当IDを更新し、要求・失敗境界・証拠を逆引きできる。
2. 既存DB行のread-only row count/hashを変更前後で比較する。通常の3か月prune以外の減少はFAILとする。
3. malformed、empty、multiple writer、app-server停止、再起動、認証境界、migration/schema mismatchを検査する。
4. `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo build --release --locked`を実行する。
5. `scripts/data_protection_gate.sh`と独立評価を実行する。
6. 変更後に新しいruntime traceを取り、前回の画像・ログを再利用しない。

### 禁止事項

- `rm`、DB削除、DB再生成、無検証の上書きで障害を隠すこと
- 有効値がないのに0%、0 token、0 dollar、空履歴を成功値として作ること
- migrationで旧行を推測変換すること
- 一時的な手動実行だけで、恒久的なテスト・台帳・CIゲートを追加せず終了すること
- 独立評価を自己評価で代用すること

## 6. 既知の回帰と再発防止メモ

過去の回帰では、巨大なtool出力1行がstrict file parserを失敗させ、ローカル履歴とactive threadが同時に「取得失敗」になった。またlocal収集がquota応答に結びついていたため、app-server停止中の変動が未収集になった。

再発防止は次の3層で固定する。

- 実装制約: record isolation、persisted reset hint backfill、auth epoch、transaction/upsert、backup-before-prune、live path + rollout terminal stateの二重判定
- 自動検査: regression/SQLite consistency/static policy/CIに加え、root/child/mixed/empty、DB欠落・重複・cycle・dangling、terminal/partial/invalid、process停止/再起動、複数server、RPC/stale epochの表形式live-state matrix
- 完了手順: freeze済み同一releaseで要求台帳の全IDが`current release PASS`、独立評価PASS、未確認事項ゼロ。抽出中の`contract authored`や旧`verified`を現行PASSへ昇格しない

「対応したつもり」「今回の実行で通った」は完了条件ではない。

## 7. DATA/REST 横断 OPEN 境界（RC-139..149 / DP-REST-001..011）

現行conflict集合はRC-001..174である。この節が直接所有するのはRC-139..149であり、RC-001..138と
RC-150..171は参照入力として扱う。
RC-150..171の意味を再登録しない。RC-139..149を、データ保護とREST境界の同じ状態・入力・保持・証拠へ1:1で結ぶための
未決authority conflictを原子化する。全項目の状態は
`OPEN_AUTHORITY_CONFLICT`であり、具体的な数値、HTTP status、schema key、generation順、
許可side effect、負荷profileを推測して確定しない。`docs/REST_API_V1.md`にも同じIDと用語を
記載し、IDの意味を文書ごとに分岐させない。

本書はDP-REST-001..011のdata state、last-good、retention、writer/maintenance identityを所有し、
`docs/REST_API_V1.md`はwire path/method/status/header/body/request limitを所有する。同じ事実を両書で
別値に決めず、cross-document joinが不一致なら`OPEN_AUTHORITY_CONFLICT`を保持する。

対応は`RC-139↔DP-REST-001`、`RC-140↔DP-REST-002`、`RC-141↔DP-REST-003`、
`RC-142↔DP-REST-004`、`RC-143↔DP-REST-005`、`RC-144↔DP-REST-006`、
`RC-145↔DP-REST-007`、`RC-146↔DP-REST-008`、`RC-147↔DP-REST-009`、
`RC-148↔DP-REST-010`、`RC-149↔DP-REST-011`に固定する。既存RC overlap欄は
`RC-064..080`、`RC-095..097`、`RC-106..107`、`RC-129`を監査した結果であり、
隣接するだけで意味が同一でないRCは重複扱いにしない。

共通語は次のとおりとする。

- `DR-AdmissionTuple`は既存の`(ProfileScopeId, AccountScopeId, StorageEpoch, SupervisorLeaseIdentity, CollectorEpoch, CycleSeq)`である。
- `DR-LastGoodPair`は、同じ現行admissionで検証済みのstatus/details `PublishedPair`である。wireへ新しい世代fieldを追加する意味ではない。
- `DR-SourceCheckpoint`はsource fingerprint、file identity、cursor、AuthEpoch/nonce、DB commitとの関係を表す未決境界である。
- `DR-GenerationNamespace`は`DataGeneration`、backup generation、`CollectorEpoch`、service/bootstrap generation、`CycleSeq`を混同しないためのnamespace・parent・順序規則である。
- `DR-DataRestLineage`はsource→DB transaction→publisher pair→HTTP response→Windows表示を同一操作として再結合する証拠identityである。encodingと全fieldは未決である。
- `DR-ReadOnlyEffectSet`はRESTがSQLite以外へ及ぼし得るlog、metric、cache、temp、filesystem、process、network副作用の許可・禁止集合である。

### RC-139 / DP-REST-001 — health body schema / limit

- **根拠**: `docs/REST_API_V1.md:80-91,93-108,138-147`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:22,27-33,83`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:74,79`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:13`。
- **状態/入力**: `HealthCandidate→HealthValid/HealthRejected`を固定し、exact path/method、listener owner、body key/type/size、healthが認証・ready・snapshot更新を意味するかを決定する。health固有のschemaと上限値はOPENである。
- **失敗/保持/idempotence**: malformed、oversize、owner不一致、non-JSONはhealth成功を0にし、health表示・connection stateと`DR-LastGoodPair`の保持関係を決める。status/detailsをhealth失敗で更新せず、同一candidateの再入はpublish/writeを増やさない。
- **identity/oracle/targets**: `DR-AdmissionTuple`、health schema generation、request identityを結び、canonical body/status/header/byte counter、listener owner、DB副作用0を再計算する。対象=`WIN-E-012`, `WIN-I-001`, `WIN-I-006..007`, `WIN-J-001`, `WIN-K-001`, `GLOBAL:AUD-011`。RC overlap=`RC-076,079`（共通limit/headerは部分定義だが、health body schemaは未定義）。

### RC-140 / DP-REST-002 — server non-2xx / error envelope

- **根拠**: `docs/REST_API_V1.md:78-108,138-140`、`docs/DATA_PROTECTION_POLICY.md:169-176`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:20-23`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:87`。
- **状態/入力**: publisher欠落、DB read fault、serialization fault、stale owner、内部timeoutを、`PublisherAvailable→Success`または`PublisherFault→ErrorResponse/Retained`へ分類する。200 `state=error`、non-2xx、切断の選択値はOPENである。
- **失敗/保持/idempotence**: error responseのschema/header/raw body・statusを固定し、raw error/秘密を出さず、失敗時は`DR-LastGoodPair`とDB rootを保持する。error再入でretry loop、DB再生成、二重publishを起こさない。
- **identity/oracle/targets**: request、`DR-AdmissionTuple`、publisher generationを結び、wire status/body/header、pair hash、DB trace、error classificationを再計算する。対象=`WIN-J-001`, `WIN-J-009`, `WIN-J-016`, `WIN-I-014`, `WIN-I-016`, `WIN-E-012`, `GLOBAL:AUD-011`。RC overlap=`RC-079,080,107`（route/status、DB副作用、ready predicateは部分定義だがserver error envelopeは未定義）。

### RC-141 / DP-REST-003 — request line/header/body/connection boundary

- **根拠**: `docs/REST_API_V1.md:69-76,88-108`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:83,86`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:79`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:13,29`。
- **状態/入力**: `ConnectionAccepted→RequestBounded→RouteDecision→Response/Rejected`を固定し、request line、request header aggregate、GET body、同時接続、keep-alive/idle、cancel、shutdownの境界値と適用時点を決める。値はOPENである。
- **失敗/保持/idempotence**: bound超過・malformed・途中切断はmaterialize前に拒否し、DB、memory、pair、設定を変更しない。拒否の再入は同じrequestを二重処理せず、shutdown中は新規admissionを決められた状態へ遷移させる。
- **identity/oracle/targets**: connection/request identity、listener generation、`DR-AdmissionTuple`を結び、raw wire byte、同時connection数、timeout/keep-alive、CPU/RSS、pair/DB before-afterを採取する。対象=`WIN-I-001`, `WIN-I-006`, `WIN-J-001`, `WIN-J-016`, `WIN-K-001`, `GLOBAL:AUD-021`, `GLOBAL:DP-008`。RC overlap=`RC-076,079,080`（response/local inputとDB副作用でありrequest envelopeは未定義）。

### RC-142 / DP-REST-004 — read-only non-SQLite effect set

- **根拠**: `docs/REST_API_V1.md:11-14,88-91,93-108`、`DESIGN.md:130-134`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:22,32-33`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:28`。
- **状態/入力**: `ReadOnlyAdmission→EffectAudit→Response/Retained`を固定し、成功・404・405・errorごとのSQLite外副作用（access/error log、metric、cache/temp、atime、filesystem journal、process、network）を`DR-ReadOnlyEffectSet`へ分類する。allow/deny値はOPENである。
- **失敗/保持/idempotence**: allowlist外の副作用を黙って許可せず、決定されたreject/holdへ進め、`DR-LastGoodPair`とDB/rootを保持する。同一requestの再入で未登録副作用や二重publishを発生させない。
- **identity/oracle/targets**: request、listener/publisher identity、effect category、generationを結び、filesystem/process/network/SQLite/WAL/SHM/row/hashの全before-afterを再計算する。対象=`WIN-I-001`, `WIN-J-001`, `WIN-J-016`, `WIN-E-012`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-080`（DB側read-onlyのみで、非SQLite effect setは未定義）。

### RC-143 / DP-REST-005 — DB profile/account/AuthEpoch storage identity

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:35-43,69,103,127-143`、`docs/REST_API_V1.md:11-14,215-219,235-242`、`docs/LIVE_STATE_DECISION_MATRIX.md:34-37`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:15-17`。
- **状態/入力（検出履歴）**: `StorageIdentityCandidate→PartitionAdmitted`または`IdentityConflict→Rejected/Quarantined`を固定し、canonical DB path、profile、account、AuthEpoch/nonceとusage keyのscopeが未決だった。採用値は§8.6のlogical partitionと`(partition_id,reset_at,timestamp)`である。
- **失敗/保持/idempotence**: identity不一致、同一keyの異なるaccount、profile alias collisionではwrite/upsert/publishを0にし、旧DB、旧root、旧accountの非表示境界を保持する。同一storage identity・同一operationの再入は1回を超えるrow/publishを作らない。
- **identity/oracle/targets**: storage-binding generation、profile/account/AuthEpoch、lease ownerを結び、同一path・同一keyを用いたcross-profile/account fixtureでpartition、row count、column MAX/COALESCE、visible occurrencesを再計算する。対象=`WIN-J-003..005`, `WIN-J-012..013`, `WIN-J-016`, `WIN-I-016`, `GLOBAL:DP-008`, `GLOBAL:LIVE-001`。RC overlap=`RC-068,069,096,106`（writer/lease/publisher identityでありstorage partitionは未定義）。

### RC-144 / DP-REST-006 — cursor + SQLite atomic commit

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:53-59,96-105,133-146,188`、`DESIGN.md:79,123`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:16-17,85,89`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:23-24`。
- **状態/入力**: `SourceRead→Candidate→DBAndCheckpointCommit→PairCandidate→Published`または`Rollback/Hold`を固定し、source fingerprint/file identity/cursor/AuthEpoch/nonce、row batch、DB transactionのcommit関係を決める。atomic join方式と値はOPENである。
- **失敗/保持/idempotence**: DB commit後のcursor失敗、cursor先行後のDB rollback、crash、busy/full/I/Oではcursor、DB、rootを同じ旧世代へ保持し、重複・欠損・推測gapを作らない。同一source checkpointとoperationの再入はidempotentにする。
- **identity/oracle/targets**: `DR-SourceCheckpoint`、operation generation、`DR-AdmissionTuple`を結び、cursor before/after、transaction id、row/hash、restart trace、publish countを同じevidenceへ入れる。対象=`WIN-J-010..012`, `WIN-J-016`, `WIN-I-016`, `GLOBAL:DP-008`。RC overlap=`RC-065,067,068,074`（cursor/gap/writer/faultを個別に扱うがcommit joinは未定義）。

### RC-145 / DP-REST-007 — generation namespace / parent / monotonicity

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:41-43,112-115,153-159,166-167`、`docs/REST_API_V1.md:11-14,235-242`、`DESIGN.md:132-133`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:19,23`。
- **状態/入力**: `GenerationCandidate→NamespaceValidated→Published`または`Stale/Regression/Collision→Retained`を固定し、`DataGeneration`、backup generation、`CollectorEpoch`、service/bootstrap generation、`CycleSeq`のnamespace、parent、順序、restore/rollback後の関係を決める。値はOPENである。
- **失敗/保持/idempotence**: namespace不明、parent不一致、generation回帰・衝突・stale candidateでは新pairを公開せず`DR-LastGoodPair`を保持する。同一operation/同一candidateの再入はgeneration/publicationを二重化しない。
- **identity/oracle/targets**: `DR-GenerationNamespace`、base DB hash、operation id、`DR-AdmissionTuple`、RootHashを結び、generation ledgerとDB/backup/pair hashを時系列で再計算する。対象=`WIN-I-016`, `WIN-J-014..016`, `WIN-L-016`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-071,073,078,096,106`（各世代・pair・ownerは部分定義で、namespace/parent関係は未定義）。

### RC-146 / DP-REST-008 — explicit restore crash journal / re-entry

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:150-161,165-167`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:18-20,88`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:26-27`。
- **状態/入力**: 明示restoreを`RestoreIdle→AdmissionClosed→CandidateAudited→CurrentQuarantined→Replaced→PairChecked`または`RestoreFailed→Reconcile`として固定し、operation id、owner、phase、current/candidate path・inode・hash、crash/re-entryを入力にする。restore journal schemaはOPENである。
- **失敗/保持/idempotence**: 退避後・replace前・reload/pair検証中のcrashや第二restoreではwriter/publishを0にし、現DB、全verified backup、old memory/rootを復元可能に保持する。同一operationのresumeは一回だけ、foreign operationはtakeover/rewriteしない。
- **identity/oracle/targets**: MaintenanceOwner、operation generation、journal identity、DB generationを結び、各crash boundaryのjournal replay、current DB count、candidate/current hash、switch/publish count、pair検証を再計算する。対象=`WIN-J-006`, `WIN-J-014..015`, `WIN-I-016`, `WIN-L-016`, `GLOBAL:DP-008`。RC overlap=`RC-071,072,073`（backup/migrationまたはrestore順序はあるが、restore固有journal/re-entryは未定義）。

### RC-147 / DP-REST-009 — host reboot daemon re-entry

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:119-142`、`docs/REST_API_V1.md:35-56`、`DESIGN.md:111`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:14-16,83-86`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:19,22-23`。
- **状態/入力**: `HostBoot→RecoveryAdmissionClosed→LeaseJournalCursorRevalidated→NewCollectorEpoch→Starting→Running`または`Hold`を固定し、boot/power-loss identity、persisted lease/journal/cursor/hint、DB/root/generationを入力にする。systemd boot orderingとre-entry authorityはOPENである。
- **失敗/保持/idempotence**: stale lease、未回収journal、cursor/hint不一致、旧publisherの再入は採用せず、旧DB/root/pairを保持する。同一boot/operationの再入はactivation、lease取得、backfill、publishを二重化しない。
- **identity/oracle/targets**: boot identity、new supervisor/collector epoch、owner nonce、`DR-SourceCheckpoint`、`DR-AdmissionTuple`を結び、reboot/power-loss前後のprocess/lease/journal/cursor/DB/pair traceを再計算する。対象=`WIN-J-010`, `WIN-J-011`, `WIN-J-013`, `WIN-J-016`, `WIN-I-016`, `GLOBAL:DP-008`, `GLOBAL:LIVE-001`。RC overlap=`RC-064,066,071,073`（通常crash/ownerとmaintenance recoveryでありhost reboot re-entryは未定義）。

### RC-148 / DP-REST-010 — source→DB→pair→HTTP→Windows lineage

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:7-15,110-115,133-146,156-167`、`docs/REST_API_V1.md:11-14,235-242`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:20-23,89`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:89`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:26-28`。
- **状態/入力**: `SourceCandidate→DBTransaction→DataGeneration/RootHash→PublishedPair→HTTPResponse→WindowsAcceptedRoot`を一つのlineage stateとして固定し、source fingerprint/cursor、AuthEpoch、transaction id、DB generation、request id、artifact SHA、表示rootのjoin fieldsを決める。schemaとencodingはOPENである。
- **失敗/保持/idempotence**: 任意edgeの欠落、foreign generation、時刻逆転、別artifactでは受理・表示・顧客証拠を0にし、`DR-LastGoodPair`、旧DB/root、失敗evidenceを保持する。同じlineageの再入はrow/transaction/publish/displayを二重化しない。
- **identity/oracle/targets**: `DR-DataRestLineage`、`DR-GenerationNamespace`、`DR-SourceCheckpoint`、`DR-AdmissionTuple`を全edgeへ付与し、全hash・timestamp・before/after・request/response・Windows acceptanceを一つのDAGとして再計算する。対象=`WIN-J-010..016`, `WIN-I-007`, `WIN-I-016`, `WIN-L-016`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-078,096,106`（pair atomicityとowner identityは部分定義だが、source→DB→HTTP→Windowsの全lineageは未定義）。

### RC-149 / DP-REST-011 — combined low-load scope

- **根拠**: `docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:83,85-86`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:22`、`docs/DATA_PROTECTION_POLICY.md:58-59,133-135,188`、`docs/REST_API_V1.md:69-108`。
- **状態/入力**: `LoadProfileDeclared→Measured→InScope/Unsupported`を固定し、daemon-only idleか、REST polling、複数collector/server、maintenance、同時requestを含むcombined profileかを決める。scope、同時数、測定時間、CPU/RSS/work counterのauthority値はOPENである。
- **失敗/保持/idempotence**: 宣言profile外の実測を製品負荷保証へ昇格せず、超過・測定不能時はデータ/root/pairを変更しない。同じprofileの再測定は追加writer、scan、publishを発生させない。
- **identity/oracle/targets**: host、artifact SHA、各process owner、`DR-AdmissionTuple`、operation generationを結び、daemon/API/maintenance/collectorのCPU/RSS、request/connection、scan/write/retry、DB/pair hashをcombined traceで再計算する。対象=`WIN-J-010`, `WIN-J-013`, `WIN-J-016`, `WIN-I-001`, `WIN-J-001`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-075`（daemon finite boundsのみで、combined scopeは未定義）。

## 8. DP-REST-001..011 採用authority値

この節は§7のOPEN値を要求として一意化する。§7は検出履歴と対象mappingとして保持するが、値の正本は
本節である。これは実装済み、実機確認済み、または製品PASSを意味しない。

```text
decision_id = DP-REST-AUTHORITY-20260823-001
decision_version = dp-rest-authority-v1
authority_status = REQUIREMENTS_SELECTED
product_status = PRODUCT_PENDING
release_status = HOLD
```

### 8.1 共通採用規則

- REST workerはread-only consumerであり、health/status/details、404、405、server errorのいずれでも
  SQLite transaction、DB/WAL/SHM、backup、migration、checkpoint、PublishedPairを変更しない。
- statusとdetailsは同じ`DR-AdmissionTuple`に属する一つの`DR-LastGoodPair`としてだけ採用する。
  status単独またはdetails単独のcommit、表示、cache更新は0件とする。
- wire producerが返す全JSON responseのContent-Typeは正確に
  `application/json; charset=utf-8`である。consumerもmedia type=`application/json`かつ
  charset=`utf-8`の両方を要求し、charset欠落・別charset・parameter追加をrejectする。
- authority不一致・schema不正・resource超過・stale ownerではcandidate側のwrite、publish、表示を0件とし、
  直前の完全DB、checkpoint、PublishedPair、Windows accepted rootを保持する。

### 8.2 DP-REST-001 / RC-139 — healthとpair保持

healthはAPI listenerの到達性だけを所有し、認証、ready、DB健全性、snapshot鮮度を意味しない。
health候補がmalformed、oversize、foreign listener、schema/header不一致ならconnection stateを
`HealthUnavailable`へ遷移させるが、`DR-LastGoodPair`、DB、collector stateは変更しない。同じrequestを
再送してもdata generation、pair publication、DB writeは0件である。wire schemaと1 KiB上限は
`REST_API_V1.md`の§「DP-REST wire authority」を唯一のownerとする。

### 8.3 DP-REST-002 / RC-140 — server faultとatomic pair

publisher欠落、DB/root read fault、stale owner、内部read timeoutはREST側のcanonical 503、response生成失敗は
canonical 500またはresponse未commit時のconnection abortへ写像する。200 error bodyは使用しない。どのfaultも
status/detailsの片側だけを進めず、clientはS1+D0やS0+D1を作らない。特に`WIN-I-016`の採用規則は
`both-valid-and-same-pair→commit both`、それ以外は`retain S0+D0`であり、status-only commitを禁止する。

### 8.4 DP-REST-003 / RC-141 — request resource owner

request line、header、body、connection、timeout、keep-alive、shutdown drainの値はREST wire ownerへ委譲し、
data ownerは全rejectでDB、memory pair、settings、checkpointのbefore/after hash一致を要求する。connection/request
identityはlistener generationへbindし、shutdown開始後の新規request admissionは0件とする。

### 8.5 DP-REST-004 / RC-142 — read-only effect分類

許可する副作用は、当該request lifetime内のheap buffer、bounded in-memory counter、loopback socketのread/write、
read-only file open/statだけである。禁止する副作用は、persistent access/error log、Event Log、persistent metric、
cache/temp file、registry、child process、非loopback socket/DNS、file create/write/rename/delete/fsync、
SQLite transactionとDB/WAL/SHM mutationである。OSがread-only openに伴って更新し得るatimeはproduct data mutation
の成功条件に使わず、DB content/inode/WAL/SHMとproduct syscall traceを判定ownerにする。allowlist外のeffectを
検出したresponseは成功証拠にせずreleaseをHOLDにする。

### 8.6 DP-REST-005 / RC-143 — storage partition identity

一つのcanonical DB file内にlogical partitionを持つ。partition keyは
`(ProfileScopeId, AccountScopeId, StorageEpoch)`であり、usage rowの一意keyは
`(partition_id, reset_at, timestamp)`である。

- `ProfileScopeId`: 保存profileを作成した時に生成する128-bit random opaque ID。raw WSL distro、SSH alias、pathを
  DBへ保存しない。
- `AccountScopeId`: authenticated app-server ownerがcanonical AccountKeyから、owner-only 256-bit install keyを使い
  `HMAC-SHA-256("codex-info-account-scope-v1" + NUL + AccountKey)`で生成する32-byte値。raw AccountKey、email、tokenを
  DB・hint・logへ保存しない。
- `StorageEpoch`: partition作成時のmonotonic unsigned 64-bit値。account/profile不一致やHMAC key欠落時は新規writeと
  publishを0件にし、自動的な空partitionや推測mergeを作らない。

同一account/profileの再認証は同じpartitionを再利用し、別account/profileは別partitionにする。画面は現在認証済み
partitionだけを公開し、旧partitionを削除・混合しない。HMAC install keyは0600 owner-only fileへatomic保存し、
欠損時は既存AccountScopeIdを再生成せずrecovery-requiredとする。

### 8.7 DP-REST-006 / RC-144 — cursorとDB transaction

authoritative checkpointは外部cursor fileではなく同じSQLite DBの`collector_checkpoint` tableに置き、usage row batch、
dedupe record、`DataGeneration`、RootHashと同じtransactionでcommitする。checkpoint keyは
`(partition_id,source_file_device,source_file_inode)`、値はfingerprint、durable byte offset、last complete record hash、
AuthEpoch nonce、operation IDである。外部hintはscan開始候補でありcommit authorityではない。

transaction commit前はrow/checkpoint/generationの全てが旧値、commit後は全てが新値である。commit後publish前のcrashは
再起動時にDB checkpoint/rootから同じpairを一度だけ再生成する。source record identity
`(partition_id,file identity,start offset,end offset,record hash)`はuniqueで、再読込はno-op、partial rowとcursor先行を
0件にする。

### 8.8 DP-REST-007 / RC-145 — typed generation namespace

bare integerを異なるnamespace間で比較しない。採用型は次のとおりである。

- `BootId`: Linux `/proc/sys/kernel/random/boot_id`のUUID。
- `SupervisorLeaseIdentity`: canonical DB profile、BootId、PID、process start ticks、128-bit owner nonceのtuple。
- `CollectorEpoch`: lease取得ごとに生成する128-bit random ID。
- `CycleSeq`: CollectorEpoch内で1から始まり、admitted cycleごとに1増えるu64。
- `DataGeneration`: partition内で0から始まり、usage rowとcheckpointの同一transaction commitごとに1増えるu64。
- `BackupGeneration`: DB profile内のu64と128-bit backup ID。parent DataGenerationとDB SHAを必須にし、一activationで
  最大1世代だけ増える。0/1/2から同一snapshot複製で3世代を穴埋めしない。
- `MigrationGeneration`と`RestoreGeneration`: 128-bit operation ID、parent DataGeneration、source DB SHA、result
  DataGenerationのtuple。成功switch後だけresult DataGeneration=`current+1`、失敗時は未発行である。
- `ServiceGeneration`と`BootstrapGeneration`: service manager activation IDとartifact SHAのtuple。

全ledger entryはnamespace tag、value、parent、operation kind、scope、source/result hashを持つ。unknown parent、回帰、
同namespace同値別hash、stale CollectorEpochはpublish 0である。

### 8.9 DP-REST-008 / RC-146 — restore journal

restore journalはowner-only 0600、UTF-8 JSON、64 KiB以下、schema=`codex-info-restore-journal-v1`とし、exact keyを
`schema_version,operation_id,operation_generation,owner_identity,phase,current_identity,current_sha256,
candidate_identity,candidate_sha256,quarantine_identity,quarantine_sha256,source_backup_generation,
parent_data_generation,result_data_generation_or_null,created_at_utc,updated_at_utc`に固定する。phaseは
`admission_closed,candidate_audited,current_quarantined,candidate_replaced,pair_checked,committed,rollback_required,
rolled_back`だけである。

各rename前後にfileとparent directoryをfsyncしてjournal phaseをatomic更新する。未terminal journalがある起動はwriter/API
admissionを閉じ、same operationだけをresumeする。第二・foreign restoreはBusyでmutation 0。current candidateの双方が
validでもjournal identity/hash/phaseだけで一意にrollbackまたはroll-forwardし、mtimeやfilename推測を使わない。
`pair_checked`完了前はold pairを保持し、current DBを削除しない。

### 8.10 DP-REST-009 / RC-147 — boot recovery order

systemd installed profileでは`codex-info-recorder.service`がDB/source mountを要求し、
`Before=codex-info-api.service`でrecovery gateを所有する。API serviceはrecorderのjournal/lease/checkpoint検査が
`RecoveryReady`になった後だけpublisher admissionを開く。BootIdを毎activationで取得し、旧BootIdのlease、未terminal
maintenance journal、file identity不一致checkpointがあればwrite/publishを0にして`RecoveryRequired`へ置く。

同一BootId・service activation IDのStartLimit外再入は新CollectorEpochを作らずno-op、別BootIdでは旧callbackを全破棄して
新SupervisorLeaseIdentity/CollectorEpochを各1件だけ発行する。systemd外の明示record commandも同じrecovery gateを通る。

### 8.11 DP-REST-010 / RC-148 — lineage schema

`DR-DataRestLineage`はcanonical UTF-8 JSON objectでschema=`codex-info-data-lineage-v1`とする。必須fieldは
`schema_version,source_release_id,server_artifact_sha256,windows_artifact_sha256_or_null,profile_scope_id,
account_scope_id,storage_epoch,source_file_identity,source_fingerprint,cursor_start,cursor_end,source_record_set_hash,
collector_epoch,cycle_seq,transaction_id,data_generation,db_sha256,root_hash,published_pair_hash,listener_generation,
request_id,route,response_status,response_body_sha256,client_snapshot_hash_or_null,render_generation_or_null,
operation_started_at_utc,operation_committed_at_utc`である。

product常時運転でこの全objectを別logへ毎回保存せず、DBにはcheckpoint/generation/rootだけを保持する。test/evidence modeは
同じ内部IDsからobjectをbounded生成し、raw path、account、token、bodyを含めない。全edgeのhash/parent/time順を再結合できない
candidate、別artifact、同じlineage IDの別hashは受理0である。

### 8.12 DP-REST-011 / RC-149 — supported low-load profile

製品保証scopeは、1 recorder、1 loopback API、1 Windows client、10秒completion-based poll、in-flight 1、source不変、
maintenance停止中の`steady_idle`である。warm-up 2分後の30分窓で次を同時に満たす。

- recorder: CPU平均0.5%以下、1秒sample p95 2%以下、RSS増加16 MiB以下、full source scan/DB write/retry 0。
- API: request外CPU平均0.2%以下、1秒sample p95 1%以下、RSS増加8 MiB以下。各pollはhealth/status/detailsを各1回以下。
- Windows client: CPU平均1.0%以下、1秒sample p95 5%以下、RSS増加32 MiB以下、poll queue 0。
- Linux recorder+APIとWindows clientを各hostの1 logical CPUへ正規化した平均の合計は2.0%以下。source不変時の
  product DB/write bytesは0、networkはloopback/SSH tunnelのpoll bytesだけである。

`changed,backfill,maintenance,recovery`はidle保証へ混ぜず別profileとし、各generationでscan 1、DB transaction 1、
publish 1以下、batch 1024 rowまたは1 MiB、REST request境界は§8.4、backfill total inputは既存2 GiB上限を守る。
測定不能または超過は要求PASSへ丸めずPRODUCT_FAIL/HOLDにし、負荷低減のためデータを捨てたりpollを重ねたりしない。

### 8.13 RC-067 — gap ledgerとREST projection

通常欠測と回収不能gapをtimestamp間隔だけから推測する設計は禁止する。DB ownerは
`recorder_gap_ledger`を持ち、schema=`recorder-gap-ledger-v1`、exact fieldを
`gap_id,partition_id,source_identity_before,source_identity_after,cursor_before,cursor_after,
stopped_at_monotonic_ns,resumed_at_monotonic_ns,start_at,end_at,reset_at,reason,state,
owner_collector_epoch,confirmation_cycle_seq`へ固定する。reasonは
`daemon_stop_unrecoverable,reset_hint_expired,auth_epoch_tombstoned`、stateは
`pending,confirmed,recovered,rejected`だけである。

stop/restartを検出した時点では`pending`であり、bounded source rescan/backfillが完了するまでREST/UIへgapを
公開せずlast-good Graph rootとbounded statusを保持する。全missing minuteがvalid source recordで回収できた場合だけ
`recovered`としgap projection 0、回収不能な閉区間をsource cursorと前後identityで証明した場合だけ`confirmed`とする。
invalid/foreign owner、時刻逆転、reset period外、overlap contradictionは`rejected`でcandidate rootを変更しない。

REST detailsへ公開するのはconfirmed rowだけで、raw path/cursor/process/ownerを除外した
`history_gaps` projectionとする。projectionは`gap_id,reset_at,start_at,end_at,reason`の5 exact field、
同一period内で`start_at<=end_at`、重複/交差なし、canonical `(reset_at,start_at,end_at,gap_id)`順、最大4096件である。
この追加は未出荷のREST v1 contract revision
`rest-v1-details-gap-20260823`として明示し、server/client/release manifestのrevision一致を必須にする。
旧12-key details clientは新13-key bodyをrejectしてfail closedし、旧artifactと新serverを混在させて成功扱いにしない。
API family/pathと`api_version="v1"`は維持するが、同一releaseに旧/new schemaを混在させない。

Graphはnormal unbracketed terminal missingでlast measured remainingを水平保持する。confirmed gapのstart直前でsubpathを
終了し、gap区間へremaining/model valueをcarry/interpolateせず、gap markerだけを描く。pending/rejected ledger、
timestamp間隔だけ、transport errorからgapを推測しない。gapが後の明示repairでrecoveredへ変わる場合は、完全な新details
rootの実sampleとgap集合を一括採用した時だけmarkerを除去する。

## 9. RC-167〜169 データ保護 fault / source checkpoint closure

この節は `WIN-J-007..016` に対する要件抽出の正本であり、製品実装済み・実機確認済みを意味しない。
既存行の `depends_on` と B2B projection header は変更せず、ここで定義する RC-167〜169 は
既存行の independent oracle へ join する。実行前提でない oracle join を新しい hard edgeへ昇格させない。

### RC-167 — source rotation / truncation checkpoint

`source_event` は `append`、`rotate`、`truncate`、`replace` の4値だけとする。source identity は
`(device,inode,size,prefix_generation)` とし、Windowsでの実体名はそれぞれ
`device_or_volume_serial`、`file_index_or_inode` と記録する。`prefix_generation` は canonical pathごとの
monotonic `u64` で、`[0,last_complete_lf_offset)` の完全prefix hashが変わった場合または
identity replacementが検出された場合だけ増加する。mtime、filename、現在時刻から推測しない。

event分類は、再openした同一canonical pathの before/after identity と prefix hashを同じoracleで比較し、
`rotation_marker=1` かつ identity変更なら `rotate`、identity変更で markerがなければ `replace`、
identity不変かつ size減少なら `truncate`、identityとprefixが不変で size増加なら `append` とする。
それ以外は `replace` として fail-closed にする。

- `append` は同じ device/inode/prefix_generation の durable cursor
  `(last_complete_lf_offset,last_complete_row_sha256)` 以降だけを読む。
- `rotate` と `replace` は新しい file identity の cursor を必ず `0` に reset する。
- `truncate` は新size以下の最後の durable LF boundaryへ cursorを clamp し、その直前の完全recordを
  最大1件だけ bounded overlap として再検査する。boundaryが存在しない場合だけ file offset `0` から1回読む。
- 旧cursorで有効recordを `skip` する数は `skip_count=0`、dedupe keyによる重複insert数は `dedupe_insert_count=0` とする。
  dedupe key は `(partition_id,file_device,file_inode,start_offset,end_offset,record_sha256)` である。
- 1 eventにつき scan は最大1回、DB transaction は最大1回、同じcallback内のretryは `0`、次の通常cycleまたは
  明示操作でのretryは最大1回とする。1 transaction は最大1024 rowsかつ1 MiB、1 JSONL recordは4 MiB、
  1 source fileは256 MiB、session aggregateは2 GiBを超えない。入力fingerprint不変時は全走査、DB write、retryを各 `0` とする。

RC-167 oracle は `source_identity_before/after`、`prefix_generation_before/after`、cursor before/after、
`scan_event`、`scan_count`、`scan_bytes`、`scan_records`、`transaction_count`、`skip_count`、
`dedupe_insert_count=0`、前後の完全record hash、DB row/file SHA、publisher generation、restart traceを
同一case markerへ結合する。old cursor継続、無制限再走査、4 MiB超record受理、前後valid recordの削除、
lease/generation不一致のpublishは FAIL とし、failure時は old checkpoint、DB、confirmed gap ledger を保持する。

### RC-168 — exact database fault matrix

fault enum は次の11値に固定する。各caseは `RC-168:<fault_enum>:<injection_point>:v1` の専用markerを持ち、
同じfault名を別注入点の結果へ流用しない。

| fault_enum | injection_point | exact SQLite result / exact OS result | required transition | retention and retry |
| --- | --- | --- | --- | --- |
| `BUSY` | usage transaction begin or write batch | `SQLITE_BUSY` / `NONE` | full transaction rollback | old DB, verified backups, history, root保持; same-callback retry=0; next cycle max1 |
| `LOCKED` | shared-cache transaction or backup read while competing writer owns lock | `SQLITE_LOCKED` / `NONE` | candidate and transaction mutation=0 | old DB/backup/history保持; lock解消後の別cycleだけmax1 |
| `IOERR` | source open/read, DB write, candidate fsync, or directory fsync | `SQLITE_IOERR` / `EIO` | batch or candidate rollback | partial row/candidate保持なし; same-callback retry=0 |
| `FULL` | DB page write or backup candidate fsync | `SQLITE_FULL` / `ENOSPC` | transaction rollback and prune=0 | current DB, backups, history保持; 容量解消後の別cycleだけ |
| `READONLY` | writer admission or DB/candidate open with read-only filesystem | `SQLITE_READONLY` / `EROFS` | writer/candidate admission=0 | old DB/backup/history保持; automatic repair=0 |
| `PERMISSION` | DB/backup open, rename, or prune delete permission check | `SQLITE_CANTOPEN` / `EACCES` | operation mutation=0 | old DB/backup/history保持; permission回復後だけ明示/次cycle |
| `CORRUPT` | DB open/read or `PRAGMA quick_check` | `SQLITE_CORRUPT` / `NONE` | corrupt source/candidate publish=0 | old readable DB/verified backups/history保持; empty DB再生成=0 |
| `BACKUP_VALIDATION` | candidate quick_check/schema/row/hash/period validation before rotation | `SQLITE_OK` plus `BACKUP_VALIDATION_FAILED` / `NONE` | candidate publish=0 and prune=0 | current DB and verified backup set保持; unverified generation採用=0 |
| `BACKUP_ROTATION` | backup journaled rename or parent-directory fsync | `SQLITE_OK` / `EIO` | rotation switch=0 and prune=0 | pre-fault current DB and verified generations保持; journal reconcileまでpublish=0 |
| `PRUNE_CONTENTION` | prune transaction after verified backup and before delete commit | `SQLITE_BUSY` / `NONE` | prune delete=0 and transaction rollback | current DB, all verified backups, history保持; next maintenanceだけ |
| `MIGRATION_LOCK` | migration lease/candidate switch admission | `SQLITE_BUSY` / `NONE` | migration switch/delete/publish=0 | old DB唯一current、candidate/journal保持; foreign operation takeover=0 |

各 fault は `operation_id`、injection point、SQLite/OS result、transaction id、canonical row SHA before/after、
current DB file SHA before/after、各 backup file SHA before/after、`quick_check`、candidate/backup/prune state、
restart後の open/read result を同じ raw recordへ入れる。fault cycleの `success_commit`、partial row、partial switch、
delete、publish、synthetic recovery は全て `0` とし、old DB・verified backup・historyを保持する。
restart後は old DBを open/read でき、検証済み世代は `quick_check=ok` でなければならない。faultの原因解消前に
同じcallbackで再試行せず、復旧後の新generationだけを1回 publishする。fault結果の流用、corrupt DBの上書き、
未検証backup採用、prune先行、空DB成功化は FAIL とする。

### RC-169 — migration atomic switch / J015-J016 re-entry

RC-169 は既存 `WIN-J-015` の「migration失敗時に旧DBを保持する」意味と、`WIN-J-016` の「clientはDBを
破壊的再生成しない」意味を変更しない。専用case markerは
`RC-169:<interrupt>:<operation_id>:<operation_generation>:migration-switch-v1` とする。
required interrupt は `pre_switch_crash`、`source_lock`、`candidate_lock`、`rename_failure`、
`post_intent_pre_commit_crash` の5値であり、既存J015の `validation_failure` は追加の候補検証失敗caseとして残す。

各caseは `owner_identity`、migration lease、old/candidate/intent/backupの path・device/inode・SHA、
exact journal key/phase、rename count、switch/delete/publish countを記録し、
`admission_closed → backup_verified → candidate_validated → switch_intent → current_quarantined →
candidate_published → pair_checked → committed` または rollback pathを一度だけ進める。割込み後のrestartは
journalと再取得したfile identity/hash/phaseだけから rollback または roll-forward を一意に選ぶ。

verified `committed` 前は old DBだけを logical current とし、lock・validation・rename・crashの全経路で
`switch=0`、`delete=0`、`publication=0`、新DataGeneration発行=0とする。成功経路だけが candidate current=1、
old DB retained=1、rename=1、publication=1、DataGeneration delta=1となる。missing/double/empty current、
foreign owner、stale journal、未検証candidate、old DB削除、synthetic commitは FAIL とする。

同一 `operation_id` と `operation_generation` の再入は journal の同じphaseから resume または no-op とし、
追加 backup、rename、switch、delete、generation、pair publicationを各 `0` にする。foreign/second operationは
Busyで mutation `0` とする。J016 client/REST consumerはこのmigration journalやLinux DB pathへopen/write/deleteせず、
invalid/partial/foreign pairは直前のaccepted rootを保持する。RC-169 oracle は5 interrupt＋validation_failureの
restart trace、old/candidate/current count、path identity/hash、journal phase、rename/publication、DB/backup/history SHAを
独立再計算し、J015/J016の専用marker、oracle、re-entry結果を同じ artifact lineageへ結合する。

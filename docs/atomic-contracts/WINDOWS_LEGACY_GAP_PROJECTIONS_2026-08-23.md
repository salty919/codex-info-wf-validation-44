# Windows legacy-gap atomic projections

状態: `REQUIREMENTS_SELECTED / PRODUCT_PENDING / FRESH_AUDIT_REQUIRED`

対象: `RC-164..171`、legacy source `TG-SET-02,TG-INST-01,TG-THREAD-01,TG-DAEMON-01,TG-DB-01,TG-DB-02,TG-INST-02,TG-CI-01`

## 目的と不変条件

旧96件の意味が現行226件の複数行へ分散したため、単独行が部分条件だけを満たして旧要求全体をPASSに
できる欠落を防ぐ。本書はcross-row state/evidenceを所有し、各base 10-field行の意味を置き換えない。
conflict台帳からcurrent targetを毎回range展開し、下記53 ID/RC集合と完全一致させる。

各契約は製品証拠の計画であり、文書の存在、fixture名、旧画像、別artifact、自己判定を製品PASSへ使わない。
FAIL/HOLD/INCONCLUSIVE、欠落case、stale PID/image/hash、別generationが1件でもあれば旧要求、要求抽出、
実装再開、release publicationをPASSにしない。

## 10-field atomic contracts

| ID | actor | trigger / precondition | exact input / state | action / transition | exact expected / acceptance | negative / failure | retention / idempotence | target / dependency owner | independent oracle / status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| LEGACY-GAP-RC-164 | settings restart evidence owner | valid six-key settingsをatomic commit済み、同一Windows artifact SHAとconfig generationを固定 | restart_count=2、launch sequence=exit→launch1→exit→launch2、各launchに新PID/start token、settings SHA、route、surface occurrenceを記録 | launch1とlaunch2を順に行い、各起動でrouteとsettings bytesを再読する | 両起動でSetup/Welcome occurrence=0、profile/selector/marker/settings bytes不変、readyは別REST predicateだけ、forbidden secret persisted=0 | 1回だけの再起動、corrupt caseの2回で代用、別artifact/PID再利用、設定修復・秘密保存はFAIL | 同一operation/capture IDの再入はno-op、last-good settings/profile/historyを保持 | TG-SET-02、RC-164、WIN-E-015/F-008/F-011/L-004 | 2 PID、2 settings SHA、2 route/UIA traceをsequence順に再計算。要件status=REQUIREMENTS_SELECTED、製品status=PRODUCT_PENDING |
| LEGACY-GAP-RC-165 | installer update lifecycle owner | verified update candidate、owner lease、旧payload/config/history/rollback sourceが存在 | client_state=[running,graceful_shutdown_in_progress,abnormal_exit_residue]、PID/start/image、HWND、handle、lock、15秒deadline | stateごとにupdate entry→shutdown/waitまたはpreflight reject→stage→commit/rollback→restart reconcileを独立実行 | runningは明示確認後だけowned client停止、graceful中は15秒内の同一owner終了を待つ、residueはidentity/lockを再検証。各caseでcommit/publication<=1、拒否時mutation=0、成功/失敗とも旧config/history保持 | foreign PID/handle/lock、timeout、残留不明を強制終了・上書き、部分publish、別state成功の流用はFAIL | operation_id+journal_epoch+client_state generationで一度だけ収束し、旧payload/rollback source/journalをreconcile完了まで保持 | TG-INST-01、RC-165、WIN-H-009/H-011/H-012/L-015、installer lifecycle | state別process/handle/lock/journal/filesystem/registry/shortcut before-after、shutdown consent、15秒、commit/publication countを独立比較 |
| LEGACY-GAP-RC-166 | thread end-to-end evidence owner | 同一artifact SHA、caseごとにclean process start、fixture generation固定 | cases=[empty,single,multi,partial,malformed,duplicate,rpc_failure,stale]、各caseにunique capture ID、新PID/start token、input/root generation、thread set、last-good hash、UIA tree、fresh PNG | 各caseを別processで起動し、native→REST→presentation validation後に安定描画を1回captureする | 8 caseすべて1:1 manifest。empty/single/multiだけcomplete success、partial/malformed/duplicateはcandidate全reject、rpc_failure/staleはlast-good保持と正しいStatus、case固有visible text/UIA/imageが一致 | 旧PID/旧画像/別case画像再利用、invalidをempty/success表示、partial row公開、capture前後generation差はFAIL | case ID+artifact SHA+PID/start+capture tokenを一度だけ受理し、invalid/failure時last-good root/thread setを保持 | TG-THREAD-01、RC-166、WIN-D-002..010/I-016/K-009/L-009 | 8 input/root/raw/UIA/PNG manifestの集合・hash・PID時系列・visible textを独立突合。fresh_image_count=8、reused_image_count=0 |
| LEGACY-GAP-RC-167 | daemon source checkpoint owner | owner lease、durable checkpoint、line max=4MiB、file max=256MiB、aggregate max=2GiB | event=[append,rotate,truncate,replace]、identity=(device,inode,size,prefix generation)、cursor=(last complete LF offset,last complete row hash) | appendは同一identity/prefixならcursor以降、rotate/replaceはnew file cursor=0、truncateはsize以下の最後のdurable LF boundaryへclampし直前complete record 1件だけをoverlap再検査。境界なしは0からfile bound内で1回scan | old cursor skip=0、dedupe key duplicate insert=0、前後valid record保持、1 eventにつきscan transaction<=1、bytes<=256MiB/fileかつaggregate<=2GiB、同一fingerprint cycle scan/write=0 | old offset継続、全履歴無制限scan、4MiB超record受理、重複登録、前後valid削除、lease不一致publishはFAIL | checkpointはDB transactionとatomic commit、同一source generation再入no-op、failure時old checkpoint/DB/confirmed gap ledger保持 | TG-DAEMON-01、RC-167、WIN-J-007..013/K-007..008、GLOBAL:DP-002/003/008 | 4 eventのfile identity/cursor/scan bytes/record count/dedupe/DB hash/publisher generationをbefore-after再計算 |
| LEGACY-GAP-RC-168 | DB fault-matrix owner | source DB、verified backup、transaction/journal generation、fault injection pointを固定 | faults=[BUSY,LOCKED,IOERR,FULL,READONLY,PERMISSION,CORRUPT,BACKUP_VALIDATION,BACKUP_ROTATION,PRUNE_CONTENTION,MIGRATION_LOCK]、各faultにSQLite resultまたはOS error、before row/file SHA | 各faultをwrite/backup/prune/migrationの所有phaseへ1件ずつ注入しrollback→process restart→open/read/quick_checkを行う | success commit=0、partial row/switch/delete/synthetic recovery=0、old DBとverified backup/history保持、restart後old DB readable、復旧を検証した次generationだけpublish | faultをempty DBやsuccessへ変換、corrupt sourceを上書き、未検証backup採用、prune先行、別fault結果流用はFAIL | fault operation ID再入は同じjournalからno-opまたは一度だけbounded retry、old/current/backup generationをreconcileまで保持 | TG-DB-01、RC-168、WIN-J-004..008/J-012/J-014..016 | 全faultのinjection point/result/transaction/row/file SHA/quick_check/candidate/backup/prune/restart traceを独立比較 |
| LEGACY-GAP-RC-169 | migration atomic-switch owner | old DBが唯一current、candidate、migration-switch-v1 journal、exclusive leaseを固定 | interrupts=[pre_switch_crash,source_lock,candidate_lock,rename_failure,post_intent_pre_commit_crash]、old/candidate/intent/backup path identity/hash | caseごとにprepare→verify→intent→switch→commitへ進め、割込み後再起動でjournal/file identityを再検証してrollbackまたはroll-forwardを一度だけ選ぶ | verified commit前はold DBだけcurrent、lock時switch/delete=0、rename/publication<=1、同一operation再入で二重migration=0、commit後だけnew generation publish | missing/double/empty current、foreign lease、stale journal、未検証candidate、旧DB削除、synthetic commitはFAIL | old DB/candidate/backup/journalをterminal reconcileまで保持し、same operation/phase再入no-op | TG-DB-02、RC-169、WIN-J-015/J-016 | 5 interruptのlease/journal/path identity/hash/rename/current count/publication/restart traceを独立再計算 |
| LEGACY-GAP-RC-170 | installer resource-recovery owner | operation=[install,update,rollback,uninstall]、owner lease、durable journal、resource manifest固定 | faults=[registry_delete_denied,shortcut_delete_denied,process_interrupt_each_stage,resume_after_reboot]、resources=[filesystem,registry,shortcut,journal,staging,rollback,tombstone] | operation×faultを別caseで注入し、失敗をjournalへdurable化→rollbackまたはreboot後same journal resume→全resource再照合 | 部分install/orphan Apps entry/wrong shortcut/success code=0、旧payload/config/history保持、terminal時だけexit一回、re-executionは同一journalから一度だけ収束 | cmd shell cleanup、失敗resourceを無視、journalなし再開、別operationへresume、foreign resource削除はFAIL | resource before-after manifestとjournalをterminalまで保持、same operation/epoch side effect各1以下 | TG-INST-02、RC-170、WIN-H-003/H-007..012/L-015 | operation×fault filesystem/registry/shortcut/journal hash、exit/publication、reboot/resume generationを独立比較 |
| LEGACY-GAP-RC-171 | clean provenance and release evidence owner | release evidence開始時にclean checkoutとworkspace identityを固定 | checkpoints=[evidence_start,artifact_finalized]、source commit、tracked_diff_count=0、build-input untracked_count=0、submodule digest、lockfile digest、toolchain digest、workspace identity、source archive SHA | startで全値capture→同じworkspaceからbuild→artifact確定直後に再capture→source/archive/artifact/manifest/fresh PID evidenceをjoin | 両checkpointでcommit/workspace/digest一致、tracked diff=0、build input untracked=0、build後source change=0。同一source releaseとartifact固有SHAだけがfresh PID/image/independent verdictへjoin | dirty/unknown workspace、別worktree/commit artifact混入、untracked input、submodule/lock/toolchain差、build後変更ではpublication/PASS=0 | generated outputはbuild input外の明示allowlistだけ。source/archive/raw statusをimmutable evidenceとして保持し同一capture再入no-op | TG-CI-01、RC-171、WIN-A-020/L-001..016、freeze/release lineage | 2 checkpoint raw status、source archive hash、toolchain/lock/submodule、artifact manifest、PID/image reviewer identityを独立再計算 |

## Current row projection

| concrete row ID | `legacy_gap_projection` exact set |
| --- | --- |
| `WIN-A-020` | `RC-171` |
| `WIN-D-002` | `RC-166` |
| `WIN-D-003` | `RC-166` |
| `WIN-D-004` | `RC-166` |
| `WIN-D-005` | `RC-166` |
| `WIN-D-006` | `RC-166` |
| `WIN-D-007` | `RC-166` |
| `WIN-D-008` | `RC-166` |
| `WIN-D-009` | `RC-166` |
| `WIN-D-010` | `RC-166` |
| `WIN-E-015` | `RC-164` |
| `WIN-F-008` | `RC-164` |
| `WIN-F-011` | `RC-164` |
| `WIN-H-003` | `RC-170` |
| `WIN-H-007` | `RC-170` |
| `WIN-H-008` | `RC-170` |
| `WIN-H-009` | `RC-165,RC-170` |
| `WIN-H-010` | `RC-170` |
| `WIN-H-011` | `RC-165,RC-170` |
| `WIN-H-012` | `RC-165,RC-170` |
| `WIN-I-016` | `RC-166` |
| `WIN-J-004` | `RC-168` |
| `WIN-J-005` | `RC-168` |
| `WIN-J-006` | `RC-168` |
| `WIN-J-007` | `RC-167,RC-168` |
| `WIN-J-008` | `RC-167,RC-168` |
| `WIN-J-009` | `RC-167` |
| `WIN-J-010` | `RC-167` |
| `WIN-J-011` | `RC-167` |
| `WIN-J-012` | `RC-167,RC-168` |
| `WIN-J-013` | `RC-167` |
| `WIN-J-014` | `RC-168` |
| `WIN-J-015` | `RC-168,RC-169` |
| `WIN-J-016` | `RC-168,RC-169` |
| `WIN-K-007` | `RC-167` |
| `WIN-K-008` | `RC-167` |
| `WIN-K-009` | `RC-166` |
| `WIN-L-001` | `RC-171` |
| `WIN-L-002` | `RC-171` |
| `WIN-L-003` | `RC-171` |
| `WIN-L-004` | `RC-164,RC-171` |
| `WIN-L-005` | `RC-171` |
| `WIN-L-006` | `RC-171` |
| `WIN-L-007` | `RC-171` |
| `WIN-L-008` | `RC-171` |
| `WIN-L-009` | `RC-166,RC-171` |
| `WIN-L-010` | `RC-171` |
| `WIN-L-011` | `RC-171` |
| `WIN-L-012` | `RC-171` |
| `WIN-L-013` | `RC-171` |
| `WIN-L-014` | `RC-171` |
| `WIN-L-015` | `RC-165,RC-170,RC-171` |
| `WIN-L-016` | `RC-171` |

`total_projection_target_count=53`。A-D=10、E-I=11、J-M=32。

## Gate

1. RC-164..171とlegacy source 8件を一意に1:1 joinする。
2. conflict台帳のcurrent target rangeを展開し、53 ID/RC集合と完全一致させる。
3. 8 atomic行が10 field、空欄0、target実在、case enum・failure・retention・idempotence・oracleを持つ。
4. 3 concrete contract、legacy crosswalk、canonical、freeze、tracker、machine gateが本書へjoinする。
5. fresh evaluatorが8契約と旧96全件を最新freeze bytesから再抽出し、FAIL/INCONCLUSIVE=0にする。

一項目でも不成立なら `legacy_gap_projection_pass=0`、`legacy96_pass=0`、
`implementation_resume=0`、`release_publication=0`。

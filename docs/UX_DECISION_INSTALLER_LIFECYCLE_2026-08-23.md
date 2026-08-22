# UX Decision: Windows installer lifecycle

Decision ID: `UX-20260823-INSTALLER-001`

状態: `EXTRACTION_DECISION_RECORDED / PRODUCT_PENDING`

## 利用者の課題

初回install、update、rollback、uninstallを内部file操作だけで定義すると、利用者はどのprogramを
起動すべきか、実行中clientをどう扱うか、失敗後に旧版を使えるか、設定・履歴が残るかを判断できない。
一部fileだけが入れ替わった状態や、実際には失敗したのに成功表示する状態はB2B配布では許容しない。

## 目的

同じ`CodexInfo.WindowsClient.Setup.exe`を初回installとupdateの唯一の入口にし、Appsの登録uninstallerを
uninstallの唯一の通常入口にする。各操作をtransaction、公開状態、利用者画面、失敗保持、証拠へ分離し、
旧版・settings・Linux側historyを破壊せず、成功条件を利用者が一画面で確認できるようにする。

## 代替案と棄却理由

1. zipを展開して利用者にexe/shortcutを手作業で置かせる案は、部分導入とpath誤りを防げないため棄却する。
2. 実行中clientを強制killして上書きする案は、未完了処理とlast-goodを失うため棄却する。
3. fileを順番に削除し、途中失敗でもuninstall成功とする案は、壊れた公開状態と残留物を作るため棄却する。
4. 同一setupをmanifest検証、graceful shutdown、staging、atomic switch、公開metadata、rollbackのownerにし、
   operation journalから再開可能にする案を採用する。

## 採用案

### 共通入口と表示

- 初回install/update: releaseの`CodexInfo.WindowsClient.Setup.exe`を利用者が起動する。
- uninstall: Windows Appsの`Codex Info Monitor` entryを利用者が起動する。Start Menuからuninstall commandを
  重複追加しない。
- すべての操作画面はoperation名、current/new version、現在step、主操作、Cancel、保持対象を同一viewportに
  表示し、page/root scrollを要求しない。
- 初回installとupdateを別state machineにし、同じ`published=false`へ丸めない。

### 初回install transaction

1. setup自身、embedded payload、manifest、全notice、architecture、disk space、per-user pathをwrite前に検証する。
2. `%LOCALAPPDATA%\Programs\Codex Info Monitor.staging.<nonce>`へ展開し、全file SHA/version/entry pointを再検証する。
3. stagingをfinal rootへ同一volume atomic renameする。
4. installed executableの再検証後だけStart Menu shortcutとHKCU uninstall entryを公開する。
5. shortcut target/cwd、DisplayVersion、installed file version/hashを再読込し、全て一致した場合だけ成功を表示する。

検証・展開・rename・公開のいずれかが失敗した場合、新shortcut/HKCU entryは0、final rootは不存在、stagingを
隔離して`INSTALL_OR_UPDATE_FAILED`のRetry/Cancelを表示する。既存settings、Linux server/historyへのwriteは0。

### update transaction

1. HKCU entry、shortcut、installed path/file identity/version/hashを旧公開世代`V_old`として検証する。
2. setupはpath＋process image identityが一致するinstalled clientへgraceful shutdown requestを一回送り15秒待つ。
   応答しない場合はkill・上書きをせず、`V_old`を起動可能なままRetry/Cancelを表示する。
3. `V_new`を別stagingへ展開・全file検証し、旧rootを同一volume rollback pathへrename、new stagingを旧rootへ
   atomic renameする。shortcut/HKCUはこの時点まで`V_old`の公開値を保持する。
4. new executable/hash/version/起動smokeを検証後、同じshortcut target/cwdとHKCU DisplayVersionを`V_new`へcommitする。
5. switchまたは検証に失敗した場合はnew rootを隔離し、rollback pathを旧rootへatomic復元し、shortcut/HKCU/
   version/hashを`V_old`へ再検証してから失敗を表示する。

成功は`manifest.version == DisplayVersion == installed file version`、installer→payload→installed SHA lineage、
Start Menu起動、settings hash不変、Linux history非接触のANDだけである。利用者が明示的に選んだ場合だけupdate後の
clientを起動し、backgroundでforeground/cursorを奪わない。

### uninstall transactionと途中失敗

1. Apps entryから起動し、「Windows client、shortcut、HKCU登録を削除し、client settingsとLinux historyは保持」
   を表示する。通常導線にpurgeを置かない。
2. verified installed PIDへgraceful shutdownを一回要求して15秒待つ。timeoutはwrite 0でRetry/Cancelとする。
3. installed manifestから全targetを列挙し、operation journalとrollback copyをinstall root外のowner限定stagingへ
   作成して全hashを検証する。
4. installed rootを同一volume tombstoneへatomic renameし、shortcut/HKCUを除去し、tombstone、rollback copy、
   installer journalを削除する。
5. binary/root/shortcut/HKCU/journal/stagingの不存在とsettings hash不変を再検証した場合だけ成功を表示する。

commit途中でfile/shortcut/registry cleanupに失敗した場合は成功を表示しない。rollback copyから旧rootを復元し、
旧shortcut/HKCU/version/hashを再公開して`UNINSTALL_FAILED`のRetry/Cancelを表示する。OS lock等により完全復元も
できない場合はoperation journalを保持して次回Apps entryから同じoperationを再開し、欠損した旧版を正常と表示しない。
残留targetが一つでもあればuninstall証拠はFAILである。

### crash、power loss、reboot、resumeとowner contract（RC-102..106）

#### 共通journalと状態

install/update/uninstallは同じinstall rootに対して同時に一つだけ実行する。永続journalの必須fieldは
`operation_id`、`operation_kind`、`owner=(installer PID,PID start token,Setup HWND,WindowInstanceGeneration,owner generation)`、`journal_epoch`、現在phase、
旧/新manifest version・SHA lineage、staging/rollback/tombstoneの役割、公開metadataのcommit状態、最後にdurableになった
step、failure classである。Windows username、private path、token、SSH情報は保存しない。状態は
`Idle → LeaseHeld → Running(phase) → Committed/RolledBack/Failed`とし、crash・power loss・OS rebootはそのphaseのjournalを
flush・検証して`Interrupted → Recovery`へ遷移する。次回Setup/Apps entryはjournalを先に読み、owner identityが現行で
なければphaseごとのreplayまたはrollbackを一度だけ行い、最終invariantを再検証するまで成功表示、新しいoperation、
foreign cleanupを0とする。

各phaseのcommit、Retry、Cancel、resumeは`operation_id + journal_epoch`のCASでidempotentにし、late completionや古い
ownerのcallbackはno-opとする。ownerを再利用する場合も`(PID, PID start token, executable image identity/hash)`を毎回照合し、
同じ数値PIDの再利用を現ownerとみなさない。installed clientを停止・再起動・前面化するときは、さらに
`(client PID, client HWND, WindowInstanceGeneration)`を照合し、古いPID/HWNDへkill、message、focusを送らない。
接続経路のlistenerも管理対象に含め、固定`127.0.0.1:8787`のpre-existing listener、tunnel exit直後のforeign rebind、
schema-validでもowner不明なresponseをconnected/readyへ採用しない。Remoteはlistener owner `ProcessIdentity`とsupervised
tunnel generationをcycle前後で照合し、WSLはprofile-specific service/bootstrap generationと経路ownerを照合する。確認不能時は
candidateをrejectしてlast-goodを保持し、accepted response countを0とする。

#### 初回installのfault境界とresume（RC-102）

初回は`NotInstalled → Staging → Verified → PublishReady → Published`、中断後は
`Interrupted → Recovery → NotInstalled`とする。payload展開、全file verify、final root rename、shortcut公開、HKCU
uninstall公開、installed再検証の各直前・直後をcrash/power-loss/reboot fault pointにする。全file verifyとinstalled
再検証が終わるまでshortcut、HKCU、成功表示を公開せず、途中にfinal rootだけ、shortcutだけ、HKCUだけが残った場合も
Recoveryで新規公開を全て0に戻してstagingを隔離/cleanupする。初回のfailure/recoveryはsettings、Linux server、Linux history
へwriteせず、次回Retryは新しいoperationではなく未導入invariantから同じtransactionを再実行する。再起動後に
`final root=不存在、shortcut=不存在、HKCU=不存在`または三者の完全なverified lineageを確認できない状態は成功へ進めない。

#### updateのfault境界とresume（RC-103）

updateは`Published(V_old) → ShutdownPending → Staging(V_new) → Verified → Switching → Published(V_new)`、中断後は
`Interrupted → Recovery → Published(V_old / V_new)`とする。verified installed clientへのgraceful request、15秒wait、new
staging verify、old root→rollback rename、new root→old root rename、shortcut/HKCU/version commit、起動smokeの各境界を
fault pointにする。`V_new`の全manifest/file SHA、version、shortcut/HKCU、journal epoch、起動smokeが揃った場合だけ
新世代をcommitし、それ以外はrollback pathから完全な`V_old`（起動可能なbinary/root/shortcut/HKCU/version/hash）を一度だけ
再公開する。old/new rootの混在、metadataだけnew、stale completionによる旧clientの誤kill、settings/history writeは0とし、
reboot後のresumeがどちらも完全verifiedでない場合は`V_old`保持＋`INSTALL_OR_UPDATE_FAILED`のRetry/Cancelとする。

#### uninstallのfault境界とresume（RC-104）

uninstallは`Published → ShutdownPending → TombstoneReady → Unpublishing → Removed`、中断後は
`Interrupted → Recovery → Published / Removed`とする。verified client停止、installed root→tombstone rename、shortcut除去、
HKCU除去、tombstone/rollback/journal cleanup、最終不存在検証を各fault pointにする。reboot後はApps entryが同じ
`operation_id + journal_epoch`を再開し、binary/root/shortcut/HKCU/journal/stagingの全不存在とsettings/history hash不変を
確認するまで成功表示を0にする。commit前に復元できる障害は旧root/shortcut/HKCU/version/hashを一度だけ再公開し、OS lockで
完全復元できない場合はjournalとRetryを保持する。tombstoneだけ、shortcutだけ、HKCUだけが残るpartial removal、旧generationの
completion、通常uninstallによるsettings/history削除は成功にも次operationにも進めない。

#### singleton、re-entry、focusとjournal owner（RC-105）

lease scopeはcanonical install root＋operation kindであり、`LeaseHeld` ownerだけがjournal、staging、rollback、tombstoneを
作成・commit・cleanupできる。第二のSetup.exe、update、Apps uninstallまたは二重clickは新journal/stagingを作らず、現owner
が生きていてUI HWNDも現存する場合だけ`(PID,PID start token,HWND,WindowInstanceGeneration)`を再検証して一度前面化し、
それ以外はbusy/recovery表示だけを行う。owner crash後のtakeoverはprocess identityがdeadでありjournal phase/hashが整合する
場合だけ許可し、別ownerのlock/journal/tombstoneを削除しない。owner releaseもacquire時のidentityをcompareして同じleaseだけを
解放する。re-entry、Retry、Cancel、resumeはaction、focus、commit、cleanup各1回以下で、foreign ownerのcompletionはno-opとする。

## エラーと復旧操作

| class | 原因 | 影響 | primary | secondary | 保持 |
| --- | --- | --- | --- | --- | --- |
| `INSTALL_OR_UPDATE_FAILED` | setup/update transactionを検証またはcommitできない | 新版は未公開 | Retry | Cancel | 初回は未導入、updateは完全な`V_old`、settings/history |
| `UNINSTALL_FAILED` | uninstall transactionを完了または完全rollbackできない | 削除未完了 | Retry uninstall | Cancel | journal、settings/history、復元可能な旧版 |
| `CLIENT_SHUTDOWN_TIMEOUT` | verified clientが15秒以内に終了しない | file mutationは未開始 | Retry | Cancel | 現行client、全file/shortcut/HKCU/settings/history |

raw exception、Windows username、private path、token、SSH情報を画面・log・evidenceへ出さない。failure class、step、
version、redacted target kind、exit codeだけを保存する。

## X版との関係

Windows固有の導入・更新・削除作法だけを追加する。X/nativeのdata、period、DB、UI、daemon動作を変更せず、
Windows uninstall/updateからLinux側historyへアクセスしない。

## 非スクロール影響

progress log全文やfile一覧を画面へ詰め込まない。現在step、原因、影響、primary、Cancel、保持対象だけを同一viewportに
置き、詳細はredacted support artifactとして別保存する。長いpathで文字縮小・clip・root scrollを導入しない。

## 影響要求

`RC-102`, `RC-103`, `RC-104`, `RC-105`, `RC-106`, `WIN-E-015`, `WIN-H-001..012`, `WIN-L-005`, `WIN-L-010`, `WIN-L-015`,
`WIN-M-015..016`, `WIN-M-018`, `GLOBAL:AUD-021`, `WIN-INSTALL-01..04`, `TG-INST-01..02`。

## 証拠計画

clean install、初回各fault、running-client timeout、update success、各update fault/rollback、uninstall Cancel、各uninstall
fault/recovery、successを同一installer/payload/source releaseへ結合する。各phaseの直前・直後でcrash、simulated power loss、
OS reboot、resume、duplicate entry、owner takeoverを別caseにし、journal replay回数と`operation_id/journal_epoch`を記録する。
各caseでprocess identity、client PID/HWND/WindowInstanceGeneration、staging/final/rollback/tombstone/journal、shortcut/HKCU、
version/hash、settings、Linux history non-access、UIA/画像を採取し、reboot前後のrawを同一artifact SHAへ結合して別担当が判定する。

## 未確定

state machine、入口、15秒、公開境界、rollback、crash/power/reboot/resume、singleton/journal owner、failure class、保持、
oracleは要件契約として確定した。製品実装、実Windows host、artifact SHA、fresh画像、fault injection raw、独立製品判定は
未取得であり`PRODUCT_PENDING`を維持する。契約追加は製品PASSや完了を意味しない。

# 顧客向け運用ランブック（要求抽出契約）

状態: `EXTRACTION_CONTRACT / PRODUCT_PENDING`

この文書はRC-061〜063の操作・証拠契約である。コード、installer、installed API service、Windows hostの
実装・実行証拠は未取得なので、`PRODUCT_PENDING`の操作を現行成果物で実行可能と表示しない。ここで
固定したartifact名、path、argv、service名、操作結果は実装前の正本であり、実装者が別名や手動buildへ
置き換えてはならない。
秘密情報、認証token、session本文、raw host/user、key/path、argv、stderrは採取・保存・共有しない。

## 1. Linux/WSLの起動契約

### UIあり

契約コマンド（実装後のfresh hostでのみ実行）：

```bash
./run.sh
```

成功条件はSlint UIが起動し、server/API prepare、listener、health、status、必要時のauth確認を
別々に記録し、`state=ready AND authenticated=true`を満たしてMainのreadyへ到達することである。
API到達だけをreadyとしない。

### UIなし（開発用互換起動）

契約コマンド（実装後のfresh hostでのみ実行）：

```bash
CODEX_INFO_API_LISTEN=127.0.0.1:8787 ./run.sh
```

期待値はSlint component/window/event-loop生成=0、Slint HWND=0、`DISPLAY`/Wayland/X11利用=0、
visible HWND=0、hidden HWND=0、listenerはloopback `127.0.0.1:8787`だけ、外部bind=0である。この
repository用互換入口も内部では後述のGUI非依存`codex-info-server serve`へexecし、UI binaryをhiddenで
動かす実装を許さない。顧客の通常導線はinstalled serviceであり、Cargo、repository、`run.sh`を要求しない。

## 2. server/APIとrecorder

### 配布artifactとinstall

Linux/WSL server配布物は、同一source releaseへ結合した次のself-contained x86_64 bundleとする。

- `codex-info-server-setup`: payload/manifestを検証してinstall/update/rollback/uninstall/restore/migrateを行う実行program。
- `codex-info-server`: Slint、X11、Waylandをlink/loadしないheadless binary。
- `artifact-manifest.json`、GPL/third-party/runtime notice、3個のuser-systemd unit。

通常顧客にCargo/Rust toolchain、repository checkout、path編集を要求しない。release directoryからの手動Linux導入は
次の一つだけである。WindowsのWSL Setupでは同じsigned/hash-verified payloadを自動stageして同じinstall operationを
起動するため、このコマンドを利用者へ入力させない。

```bash
./codex-info-server-setup install
```

install先は`%h/.local/lib/codex-info-server/current/codex-info-server`、rollback用の直前verified世代は
`%h/.local/lib/codex-info-server/previous/`、管理入口は`%h/.local/bin/codex-info-server-setup`、unitは
`%h/.config/systemd/user/`に固定する。systemd user manager、同一filesystem atomic rename、manifest/SHA、
binary architecture、disk space、既存世代を全てmutation前に検証し、一つでも失敗すればfile/unit/enable状態を変更しない。

installed unitとargvはexactに次とする。

| unit | exact ExecStart意味 | owner |
| --- | --- | --- |
| `codex-info-recorder.service` | `%h/.local/lib/codex-info-server/current/codex-info-server record --interval 60` | canonical DB path/profileにつき唯一のwriter/supervisor |
| `codex-info-api.service` | `%h/.local/lib/codex-info-server/current/codex-info-server serve --listen 127.0.0.1:8787` | headless snapshot builder＋read-only REST publisher |
| `codex-info-server.target` | 上記2 unitを束ねる。binaryを実行しない | install/start/stop/restartのaggregate owner |

`codex-info-server`の`serve` processはSlint component/window/event-loopを生成せず、DISPLAY/Wayland/X11の
runtime link/loadを0、visible/hidden HWNDを0とする。`record` processはHTTP listenerを持たない。

### start・status・health・stop・restart

server/API prepare→listener→`GET /v1/health`→`GET /v1/status`→必要時だけauth開始→別のauth確認→
`state=ready AND authenticated=true`の導出順序を固定する（wire `ready` boolean field=0）。health=到達性、
status=状態、auth=認証操作、ready=認証確認後のMain状態であり、同じmarkerへ丸めない。

```bash
systemctl --user start codex-info-server.target
systemctl --user status codex-info-recorder.service codex-info-api.service
systemctl --user is-active codex-info-recorder.service codex-info-api.service
curl --fail --silent --show-error http://127.0.0.1:8787/v1/health
systemctl --user stop codex-info-server.target
systemctl --user restart codex-info-server.target
```

start成功は2 serviceの`active`、loopback listener 1、health JSON成功のANDだけであり、一方だけの
activeを成功表示しない。`GET /v1/health`と`GET /v1/status`はrecorderの稼働確認とは別のAPI観測である。appやtunnelが終了しても
recorderは独立ownerとして継続し、停止期間を後から補間しない。supervisorなしorphan tunnel=0、
同時tunnel=1、same-generation自動無限retry=0、child終了時のreap=1をraw process/listener logで確認する。

停止・再起動後はlistener、health、status、auth確認を順に再取得し、成功済みSetup/app確認を毎回表示しない。

### update・rollback・uninstall

new release directoryのverified setupを起動するupdateと、installed管理入口によるrollback/uninstallを
exactに次とする。

```bash
./codex-info-server-setup update
codex-info-server-setup rollback
codex-info-server-setup uninstall
```

updateはnew payloadを同一filesystem stagingへ置き、manifest/SHA/architecture/noticeを検証してからtargetを
停止し、current→previous、staging→currentをatomic切替し、target再起動、2 service active、health/status、
DB quick_check/row/fingerprintを検証する。失敗時はnewを成功表示せずpreviousへatomic rollbackして再検証する。
rollbackはpreviousが完全verifiedの場合だけ同じ順序で世代を入れ替える。uninstallはtargetをdisable/stopし、
unit、server binary、管理入口だけを除去する。通常uninstallはsettings、history DB、3 backup、source logsを削除しない。
purge optionはこの通常導線に設けず、顧客が誤操作で履歴を消せないようにする。

## 3. Windows installer、Start Menu、Setup、接続復旧

### installer・Start Menu・更新・rollback・uninstall

artifact filename `CodexInfo.WindowsClient.Setup.exe`のSHA-256をrelease manifestと照合してから、
WindowsのApps/installer entryから導入する。Start Menuの検査対象は次の固定entryである：

`%APPDATA%\Microsoft\Windows\Start Menu\Programs\Codex Info\Codex Info Monitor.lnk`

installer、installed binary、uninstaller、旧世代rollbackの実装とfresh Windows証拠は未取得である。
更新失敗時は新世代を成功扱いにせず、旧世代・settings・server historyを保持したままrollback
判定をOPENにする。アンインストールはWindows Apps entryまたはmanifest登録uninstallerだけを使い、
explicit purgeなしにsettingsやserver historyを削除しない。

Windows installerは同一source releaseの`codex-info-server` payloadとmanifestを内蔵する。WSL profileで
「サーバーを準備」を押すと、選択distribution内のowner限定nonce一時directoryへpayloadをstageし、
Windows側hashとWSL側hashを一致させた後、shell文字列展開なしの`wsl.exe` ArgumentListでsetup `install`と
`systemctl --user start codex-info-server.target`を起動する。Remote profileでは明示的な「サーバーを導入」
操作だけが、保存済みliteral Host aliasへ`scp.exe`でverified bundleをnonce pathへ転送し、`ssh.exe`で
setup `install`を実行する。自動startup/reconnectはinstallを再実行せずtarget startとhealthだけを行う。
mutation前の取消はwrite 0、途中失敗は旧server/unit/DBを保持し、秘密値やraw remote pathを設定・証拠へ保存しない。

### Setupの保存値と接続順序

settingsの正本keyは次の6個だけである：
`language,setupCompleted,connectionConfigured,timeZoneId,connectionProfile,connectionSelector`。
`connectionProfile`は`none|wsl|sshConfigAlias`、SSH selectorは
`^[A-Za-z0-9][A-Za-z0-9._-]{0,254}$`のliteral Host alias、WSL selectorはinstalled distributionの
exact tokenである。password/token/key/path/OpenSSH展開値/raw manual host/user/API URL/argv/stderrは
保存0とする。

Setup→server/API prepare→listener→health→status→auth開始（必要時）→別auth確認→
`state=ready AND authenticated=true`の導出順に進み、
health/status/authの意味を混同しない。old 4-key、corrupt settings、invalid selectorではWelcome loopへ
戻さず、Main disconnected + Settings recovery、automatic command=0とする。valid selectorを明示保存
するまでready/connectionConfiguredを偽装しない。

### SSH・WSL・one-session raw recovery

- 自動SSHはOpenSSHをArgumentListで直接起動し、shell/`cmd.exe`/PowerShellを経由しない。`BatchMode=yes`、
  hidden prompt=0、host-key解決はOpenSSH ownerとする。
- 未登録・変更host keyはconnectedとせず、fixed failure classを表示する。明示CTAを押した時だけ
  OpenSSH所有の一回の対話を許可する。
- `WSL recovery`、`remote recovery`、`one-session raw recovery`を別分類する。one-session rawは
  durable settings・再接続selector・完了状態にしない。
- app-wide supervisorはbootstrap/tunnel childを1つだけ所有し、saved selectorで次回自動再構築を
  行う。recorderはapp/tunnel終了後も継続する。

## 4. DB backup・restore・migration

DB保護は、source DB・既存backup・historyを失敗時に保持し、検証済みcandidateだけを切り替えることを指す。

DBを直接削除、編集、移動しない。backupはSQLite online-backup契約で世代
`usage_history.sqlite3.bak.1`〜`.bak.3`を保持し、各世代の`quick_check`、row count、deterministic
fingerprint、file SHA-256を記録する。backup失敗、`quick_check`失敗、row/hash不一致時はprune・switchを
行わず、source DBと既存backupを保持する。

restore/migrationの管理入口はexactに次とする。

```bash
codex-info-server-setup restore --generation 1
codex-info-server-setup migrate --dry-run
codex-info-server-setup migrate --apply
```

restoreは全service停止確認後、選択世代を別destinationへ検証し、source DB・selected backupの前後SHA/row/hashが
不変であることを確認してから同一filesystem staging→atomic切替→service再起動→health/status/UI reloadを行う。
失敗時は退避した現DBへ戻し、全backupを保持する。migration dry-runはwrite 0でschema/row/boundary計画だけを返す。
applyは別名candidate DBへtransaction適用→全row validate→件数/hash/期間境界比較→verified backup保持→
検証成功後だけatomic switchする。失敗時は旧DB、旧memory、旧backupを保持し、旧schemaを推測変換しない。

## 5. secret-safe support

問い合わせ時に共有できるのは、製品version、OS、再現手順、UTC時刻、終了コード、redacted fixed
failure class、DB/backup/installerのSHA-256、独立したprocess/listener状態だけである。password、token、
private key、秘密path、host/user、API URL、argv、stderr、session本文、raw response bodyは共有しない。
secret sentinel occurrence=0、raw persistence=0を確認できない報告は受入証拠にしない。

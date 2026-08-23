# UX Decision: Windows SSH/WSL connection path

Decision ID: `UX-20260822-SSH-001`

状態: `EXTRACTION_DECISION_RECORDED / IMPLEMENTATION_PENDING`

RC-061〜063の正本状態: `EXTRACTION_CONTRACT / PRODUCT_PENDING`。下記は保存schema・状態遷移・
プロセス所有・運用手順の契約であり、未取得の実装・host・artifact証拠をPASSへ昇格しない。

Decision IDは最初に判断を固定した2026-08-22を保持し、本ファイル名の日付2026-08-23は要求文書へ
正規化した日を表す。IDとfilenameの日付差は誤記や別Decisionではなく、この関係をmanifestで保持する。

## 利用者の課題

接続先を手作業で置換する説明や、SSH知識を前提とするコピー操作では、
Windows利用者は接続を完了できない。接続先、API到達性、Codex認証を同じ「接続済み」へ丸めると、
失敗原因と次操作も分からなくなる。

## 目的

SSH知識やPowerShell手入力を必須にせず、WSL/remoteの境界、API到達、Codex認証を別stepとして
完了できるようにする。markerだけで接続済みとせず、再接続可能な非秘密selectorを6-key設定へ
atomic保存する。秘密情報を製品へ保存せず、直接起動失敗時だけ安全なcopy fallbackを提供する。

## 検討した案

1. コマンド例だけを表示し、PowerShellで手動実行してもらう。
   未確定の接続先を誤実行する危険と知識依存を残すため棄却する。
2. SSH configをクライアントが完全parseし、HostName/User/IdentityFileを保存する。
   OpenSSHとの解釈差、秘密pathの露出、Include処理の複雑化を生むため棄却する。
3. WSLとremote SSHを明示profileとして分け、再接続可能なselectorだけを保存し、remoteでは
   literal Host aliasを`ssh.exe`へArgumentListで直接渡す。API確認と認証確認を別stepにする。
   知識依存と秘密保持を両立できるため採用する。

## 保存schema、profileとargv

保存対象のsettings key集合は次の6個に固定する。保存はcanonical JSONをtempへ書き、flush・
検証後にatomic replaceする。marker (`connectionConfigured`) だけを保存してselectorを失う状態、
または一時入力をselectorへ昇格する状態は作らない。

| key | 許可値・意味 | 保存境界 |
| --- | --- | --- |
| `language` | 既存のlocale string | 既存の設定契約に従う |
| `setupCompleted` | boolean | 既存の設定契約に従う |
| `connectionConfigured` | boolean marker | health/status/authの受理後だけ更新する |
| `timeZoneId` | `local` または `UTC` | 既存の設定契約に従う |
| `connectionProfile` | `none` / `wsl` / `sshConfigAlias` | 上記enum以外は拒否する |
| `connectionSelector` | profileに対応する非秘密selector | `none`ではliteral `none`、`wsl`ではinstalled distributionのexact token、`sshConfigAlias`ではliteral Host aliasだけを保存する |

`sshConfigAlias` selectorのgrammarは厳密に
`^[A-Za-z0-9][A-Za-z0-9._-]{0,254}$` とする。OpenSSHのHostName、User、Port、IdentityFile、
Include、解決後のhost、raw manual host/user、API URLはselectorではなく、一時入力または
OpenSSH所有の実行時情報であり、保存値ではない。password、token、key、path、argv、stderrは
すべて保存出現数0である。

| profile | 利用者入力 | tunnel開始 | 認証開始 | 保存 |
| --- | --- | --- | --- | --- |
| none | remote selectorなし | tunnel/bootstrapなし | authなし | `connectionProfile=none` と `connectionSelector=none` を含む6-key object |
| WSL | installed distribution一覧からexact tokenを選ぶ。host/user/port/key入力なし | 初回の明示「サーバーを準備」はWindows installer内のverified headless payloadをowner限定nonce stagingへ渡し、setup `install`→`codex-info-server.target` startを固定ArgumentListで行う。次回はtarget start＋healthだけ | WSL側の認証開始を別stepで行う | `connectionProfile=wsl` と選択したdistribution tokenだけ |
| SSH config alias | literal Host aliasを選ぶ。raw manual host/userはone-session raw recoveryだけ | 初回の明示「サーバーを導入」はverified bundleを`scp.exe`でnonce pathへ転送しsetup `install`を実行する。自動時は`ssh.exe`のArgumentListへ`BatchMode=yes`を含め、target start後に`-N`/`-L`と保存selectorを渡す | 明示CTA時だけOpenSSH所有の一回の対話を許可する。自動経路はhidden prompt=0 | `connectionProfile=sshConfigAlias` とliteral aliasだけ。展開値は保存しない |

`ssh.exe`、`wsl.exe`、bootstrap/tunnel childはshell、PowerShell、`cmd.exe /c`を介さず、
実行ファイルと個別tokenのArgumentListで直接起動する。clientはSSH configのliteral Host label候補を
表示できるが、`HostName/User/Port/IdentityFile/Include`を展開しない。selectorは1 argv tokenとして
渡し、解決・key選択・host-key確認はOpenSSHが所有する。自動Remoteは`BatchMode=yes`を必須とし、
hidden prompt=0。未登録または変更されたhost keyをconnectedへ丸めず、fixed failure classと
Settings recoveryへ遷移する。

UIなしのsilent RESTはSlint component/window/event-loopを生成せず、`DISPLAY`、Wayland、X11への依存を
0とする。Slint HWND=0（visible HWND、hidden HWNDとも0）であり、headless snapshot builderとread-only publisherだけを
許可する。このGUI依存ゼロ契約はRC-063の受入境界であり、実装・host・artifact証拠が未取得のため
`EXTRACTION_CONTRACT / PRODUCT_PENDING`を維持する。

## Setup step

1. profileと非秘密selectorを選択し、6-key設定候補を検証する
2. server/APIをprepareする。未導入なら利用者の明示操作で同一source releaseのverified `codex-info-server-setup install`を実行し、既導入なら`codex-info-server.target`をstartする。Cargo/repository/manual path入力を要求しない
3. `codex-info-api.service`のloopback listenerと`codex-info-recorder.service`のactiveを別々に確認する
4. `GET /v1/health`でAPI到達性だけを確認する
5. `GET /v1/status`で`state=auth_required,authenticated=false`と
   `state=ready,authenticated=true`を区別する（wireに`ready` booleanは存在しない）
6. 必要な場合だけ利用者が認証開始を押す。認証開始の成功をreadyとみなさない
7. 認証開始とは別の「認証を確認」でlater statusを取得し、
   `state=ready AND authenticated=true`の場合だけMain readyとする
8. ready後に6-key settingsをatomic保存する。成功済みSetup/app確認は一回だけで、poll・再接続・同一generationの自動再構築ごとにSetupを再表示しない

各stepは同一viewportに現在位置、入力/結果、primary action、Back/Cancelを表示する。成功済み
stepをpollのたびに再表示しない。connection markerはreachable APIまたは明示的auth_requiredを
受理した後だけ保存でき、remote入力を含まない。

## 失敗・recovery・lifecycle

- `WSL recovery`、`remote recovery`、`one-session raw recovery`を別のfailure sourceとして記録する。
  one-session rawはdurable settingsへ昇格せず、完了状態にも再接続selectorにもならない。
- old 4-key settings、corrupt settings、invalid profile/selectorはWelcome loopへ戻さず、Mainを
  disconnected、Settingsをrecovery ownerとして表示する。recoveryでの自動command count=0。
  明示的に有効なselectorを選び保存するまで、connectionConfigured/readyを偽装しない。
- app-wide supervisorがbootstrap/tunnel childを常に1つ所有する。保存済みselectorで次回自動再構築を
  行うが、same generationの自動無限retryは0。child終了時はreapしlistener消失を確認する。
  同時tunnelは1、supervisorなしorphan tunnelは0とする。
- Main/appが終了してもrecorderは独立ownerとして継続し、app/tunnel終了を理由にrecordingを止めない。
- DNS、alias、Include、key、host-key、password等のOpenSSH詳細はraw stderrを表示・保存せず、
  fixed failure classへ変換する。未登録/変更host keyはconnectedとせず、明示CTA時だけOpenSSH所有の
  一回の対話を許可する。
- API reachableだけでauthenticatedとせず、認証開始成功だけでもauthenticatedとしない。失敗時も
  Main/Settingsへ到達でき、Welcome/Setup loopへ戻さない。

## 影響要求

`WIN-C-016`, `WIN-E-001..016`, `WIN-F-004..011`, `WIN-I-001..005`,
`WIN-K-001..004`, `WIN-M-004`, `WIN-M-014..018`, `WIN-M-030`。

## X版との関係

X版の監視data、認証判定、last-good、期間、DBを変更しない。Windows固有に追加するのはWSL/remote
profile、shell-free process起動、Setup/Settings導線であり、API wireや認証成功条件を発明しない。

## 非スクロール影響

Setup各stepの現在地、入力/結果、primary action、Back/Cancelは同一viewportへ置く。stderr、長いhost、
SSH config候補を理由にroot/internal scrollへ逃げず、bounded選択と固定failure classを使う。

## 受入oracle（実装後）

process executable/ArgumentList token列、PID/supervisor/listener、health/status/auth request列、6-key
settings JSON schema、old4/corrupt/invalid recovery、UIA step/CTA、secret sentinel scanを同じartifact
SHAへ結合する。shell/cmd/PowerShell process 0、secret/raw persistence 0、selector grammar error 0、
orphan 0、同時tunnel 1、same-generation auto-retry無限=0、API/auth state混同0、Setup loop 0、
one-session raw durable completion=0のすべてを要求する。

## 証拠計画

上記oracleを同一release artifact SHA、source freeze、物理Windows host ID、UTC capture時刻へ結合し、
process/ArgumentList/listener/UIA/settings/secret scan/recovery sourceのrawを保存する。実装者と異なる
担当がWSL/remote/one-session rawの各成功・失敗・cancel・reopenをPASS/FAIL/INCONCLUSIVEで判定する。

## 未確定

6-key schema、profile/selector grammar、保存禁止、step、failure/recovery/lifecycle、server artifact/service名と
install/start/update/rollback/uninstall操作は確定した。実process、host、artifact SHA、操作log、fresh画像、
独立製品判定は未取得であり、`EXTRACTION_CONTRACT / PRODUCT_PENDING`を維持する。

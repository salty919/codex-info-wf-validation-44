<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Windows クライアント

`windows-client/` は、Linux / WSL で動く Codex Info の既存機能を Windows から利用する
デスクトップクライアントである。無料の Visual Studio Community で solution を開ける
Avalonia / .NET 10 プロジェクトであり、Windows が配布先、Linux / X11 が開発時の
実画面確認先になる。既存の Rust + Slint ネイティブ画面を置き換えず、同じサーバー状態を表示する。
日本語表示には既存配布物の `assets/NotoSansJP.ttf` を client assembly に埋め込み、
Linux と Windows の両方で同じ font fallback に依存しない。ライセンスは既存の
`assets/NOTICE.txt` の記載を引き継ぐ。

状態: `REQUIREMENTS_SELECTED / PRODUCT_PENDING`。SSH-001/RC-061〜063の設定、接続、headless、
supervisor、recorder、service lifecycleは要求抽出正本であり、実装・host・artifact・fresh image・
独立製品証拠を取得するまで製品PASSを主張しない。installed API serviceとrecorder serviceのexact
install/start/stop/restart/uninstall/rollback commandは未確定で、読者にpathやbinaryを推測させない。

## 利用者向け導入手順（Start メニューから起動）

### 1. Linux / WSL サーバー側

UIありの起動契約は次のとおりです。

```bash
./run.sh
```

UIなしsilent RESTの起動契約は次のとおりです。

```bash
CODEX_INFO_API_LISTEN=127.0.0.1:8787 ./run.sh
```

期待値はSlint component/window/event-loop生成=0、`DISPLAY`/Wayland/X11依存=0、Slint HWND=0
（visible/hiddenとも0）、listenerはloopback `127.0.0.1:8787`だけ、外部bind=0である。
headless snapshot builderとread-only publisherだけを許可する。このGUI依存ゼロ契約は実装未取得のため
PRODUCT_PENDINGである。

server/API prepare→listener→GET health→GET status→必要時auth-start→別auth-check→readyの順序を固定する。
installed API serviceのexact install/start/stop/restart/uninstall/rollback commandはRC-063時点で未確定で、
ここに実行可能なコマンドを発明しない。

recorderはUI/RESTと独立ownerであり、app/tunnel終了後も継続、同時tunnel=1、orphan tunnel=0、
same-generation auto retry infinite=0、child reap=1を要求する。recorder serviceのexact commandも
release manifest確定までPRODUCT_PENDINGである。

headless契約はSlint component/window/event-loopを生成せず、`DISPLAY`/Wayland/X11へ依存しない。
Windows側へ8787番ポートを公開するために、serverを`0.0.0.0`やLAN addressで待ち受けさせない。

### 2. Windows クライアントのインストール／アンインストール

artifact filename `CodexInfo.WindowsClient.Setup.exe`、per-user install、Start Menu、uninstaller、
rollbackは要求契約として記録する。installer binary、install root、shortcut、uninstallerのexact
manifest/command/fresh Windows evidenceは未取得でPRODUCT_PENDINGであり、pathを推測して実行しない。
インストーラーは資格情報、SSH鍵、raw接続先、selector以外の接続値を保存しない。

「アプリと機能」の登録名、Start Menu shortcut、uninstaller、rollbackの入口と、設定・Linux側履歴DBを
保持する境界は要求契約として記録する。exactな登録・実行ファイル・install root・manifest・commandと
physical Windows evidenceは未取得でPRODUCT_PENDINGであり、`CodexInfo.WindowsClient.Uninstaller.exe`や
PowerShell/scriptのpathを推測して実行可能とは記載しない。installed serviceのinstall/start/stop/restart/
uninstall/rollback commandも同じくPRODUCT_PENDINGである。

配布用installerのself-contained payload、第三者notice、Start Menu公開、rollback transactionは要求抽出の
対象である。Windows CI/.NET SDKによるexact build commandと生成物はrelease manifest確定後にだけ追加し、
この文書では実行可能なinstaller commandを提供しない。設定とLinux側の履歴DBを削除しない保持条件は変更しない。

### 3. SSH 転送と初回セットアップ

初回起動時は、画面の「初期設定」に従って次の順序で進む。
Windows OpenSSH の設定ファイルは `%USERPROFILE%\.ssh\config` を参照し、保存するのはそこに定義された
literal `Host` aliasだけとする。

1. `connectionProfile`と`connectionSelector`を選択する。profileは`none|wsl|sshConfigAlias`、WSL selectorはinstalled distribution exact token、SSH selectorはliteral Host alias（`^[A-Za-z0-9][A-Za-z0-9._-]{0,254}$`）。
2. server/API prepare→listener→`GET /v1/health`→`GET /v1/status`を別々に確認する。
3. 必要な場合だけauth-startを明示し、auth-start成功をreadyとしない。
4. 別のauth-checkでlater statusを取得し、wireの`ready` booleanは使わず、`state=ready AND authenticated=true`の導出条件だけでMainへ進む（ready wire boolean field=0）。
5. `language/setupCompleted/connectionConfigured/timeZoneId/connectionProfile/connectionSelector`の6-key objectをflush・validate後atomic replaceする。

自動RemoteはOpenSSHを直接ArgumentListで起動し、shell/cmd/PowerShell=0、`BatchMode=yes`、hidden prompt=0とする。
未登録/変更host keyはconnectedとせず、明示CTA時だけOpenSSH-ownedの一回のinteractiveを許可する。
raw manual host/userはone-session raw recoveryだけで、settings、selector、完了状態へ昇格しない。
old 4-key、corrupt settings、invalid profile/selectorはWelcome loopにせず、Main disconnected+Settings recovery、
automatic recovery command count=0とする。保存selectorが有効なら次回launchでapp-wide supervisorがbootstrap/tunnelを
自動再構築し、poll/reconnect/same-generation rebuildでSetup/app confirmationを再表示しない。

保存禁止はpassword/token/key/path、OpenSSH expanded values、raw manual host/user、API URL、argv、stderrである。
`HostName/User/Port/IdentityFile/Include`はclientが展開せず、literal Host labelだけをOpenSSHへ渡す。

## 通信境界

クライアントがHTTPで読む接続先は編集不可の
`http://127.0.0.1:8787/v1/status` と `http://127.0.0.1:8787/v1/details` だけである。
Linux の実アドレス、LAN アドレス、ホスト名、インターネット URL をHTTP endpointとして
入力・保存しない。SSH転送開始に必要なraw Linux host/IPまたはraw userはone-session raw recoveryとして
メモリ上だけで扱い、settings、shortcut、ログへ保存しない。durableに保存するのはprofileと、WSL installed
distribution exact tokenまたはSSH literal Host aliasの`connectionSelector`だけである。

```text
Windows client -- HTTP / 127.0.0.1:8787 --> SSH local forwarding
                                                  |
                                                  | encrypted + peer-authenticated by SSH
                                                  v
Linux / WSL -- 127.0.0.1:8787 --> Codex Info native UI + REST v1
```

クライアントは保存selectorを1 argv tokenとして、Windows標準の`ssh.exe`へ
`-o BatchMode=yes -N -L 8787:127.0.0.1:8787 <validated alias>`を直接ArgumentListで引き渡す。
自動Remoteは`BatchMode=yes`、hidden prompt=0、shell/cmd/PowerShell=0とする。
認証開始ボタンは、WSL profileではinstalled distribution tokenを含む`wsl.exe` ArgumentList、remote SSH
profileではliteral Host aliasを含む`ssh.exe` ArgumentListを一回だけ起動する。どちらも認証情報を受け取らず、
開始直後を認証完了とは扱わない。「認証を確認」で同じprofileの`/v1/status`を再取得し、
`state=ready`かつ`authenticated=true`になった場合だけ完了し、ready wire boolean field=0とする。未登録/変更host keyのautomatic routeは
connectedにせず、明示CTAの一回のOpenSSH-owned interactiveだけを許可する。

コピー用の表示文字列をlaunch inputへ再利用せず、shell/cmd/PowerShell経由の実行は行わない。
Linux / WSL側の起動契約は`CODEX_INFO_API_LISTEN=127.0.0.1:8787 ./run.sh`である。

HTTPS はここで必要としない。HTTP が使われるのは二つの loopback 終点と SSH トンネルの
内側だけであり、端末間の暗号化と相手認証は SSH が担当する。インターネット経由の利用は
この設定を広げず、別の認証・脅威モデルとして設計する。

## Windows clientの実装・受入境界

Visual Studio/.NET project、NuGet restore、build、test、client起動のexact commandは、今回の要求抽出の
実行契約ではない。製品/runtime/evaluationの証拠、artifact SHA、physical Windows host証拠が揃うまで
PRODUCT_PENDING/HOLDを維持し、読者が未確定のpathやcommandを推測して実行できる形にしない。実行時通信の
意味契約は固定loopback URLへのGET、SSH/WSL childの直接ArgumentList、shell/cmd/PowerShell=0である。

## 表示と更新

初回取得を直ちに行い、完了から 10 秒後に次の取得を行う。3 秒で応答がなければ失敗と
する。手動の「更新」も同じ単一要求ゲートを使い、要求中のクリックは待ち行列に入れず
無視する。Window を閉じると、タイマーと要求を cancellation token で停止する。

| 入力状態 | 状態帯 | 値の扱い |
| --- | --- | --- |
| `ready`（残量2%以下） | 利用枠の危険 | 最新の有効 snapshot を表示し、残量不足を赤で示す。 |
| `ready`（残量10%以下） | 利用枠の警告 | 最新の有効 snapshot を表示し、残量不足をアンバーで示す。 |
| `ready`（リセットまで24時間以内） | リセット警告 | 最新の有効 snapshot を表示し、リセット接近をアンバーで示す。 |
| `ready`（上記以外） | 正常 | 最新の有効 snapshot を表示する。 |
| `initializing` | Linux 側で準備中 | 有効 snapshot を表示する。 |
| `auth_required` | Linux 側で認証が必要 | 正常に受理した状態遷移として旧account可視値を直ちに空へ置換し、認証操作だけを表示する。旧quota/model/history/threadを現在値として表示しない。Linux側DB自体は削除しない。 |
| `error` | Linux 側の取得エラー | 直前の有効 snapshot があればstaleとして保持し、なければ未取得を表示する。通信障害とは混同しない。 |
| timeout、接続不能、HTTP 非 2xx | 接続エラー | 直前の有効 snapshot があれば保持し、「現在は更新できていません」と示す。 |
| content-type、サイズ、JSON、契約の不正 | 応答エラー | 直前の有効 snapshot があれば保持し、「現在は更新できていません」と示す。 |

`ready` の派生状態は、危険（残量2%以下）→警告（残量10%以下）→リセット警告
（リセットまで24時間以内）→正常の順に一つだけ選ぶ。`auth_required`、`error`、通信障害、
応答障害はこれより優先し、Wire の `state` を変更しない。

`auth_required` はinvalid responseではなく、認証epochを切り替える有効な消去遷移である。
この遷移を通常のlast-good保持で上書きしてはならない。旧accountのplan/quota/model/history/threadを
画面とアクセシビリティtreeから同じroot updateで除去し、Graphのmetric/toggleのようなaccount非依存controlだけを
保持できる。消去rootをMain生存中に適用できない場合は旧account情報を表示し続けずcontrolled shutdownとする。

最初の取得が失敗したときは、値欄を `未取得` とし、推測した 0 や前回プロセスの値を
表示しない。API の `observed_at` が `null` のときは「Linux の観測時刻: 未取得」、
`plan_label` が `null` のときは「プラン: 未取得」、`quota` が `null` のときは
「残り利用枠: 未取得」、モデル配列が空のときは「モデル利用: 未取得」と表示する。

## REST v1 の受理契約

クライアントは [REST API v1](REST_API_V1.md) の `GET /v1/status` と、履歴・Threads・ドル内訳を
含む `GET /v1/details` を受理する。
`Content-Type` は `application/json`、response headerは8 KiB以下とする。本文はtransfer後・
decode前で、`/v1/status`は65,536 bytes以下、`/v1/details`は33,554,432 bytes以下とする。
`Content-Length` が各上限を超える場合は本文を読まず、chunkedまたは不明長の本文は読み取り
途中で各上限を超えた時点で停止する。自動解凍は無効なので`Content-Encoding`付き応答を
解凍して受理しない。`/v1/details`は本文上限とは別にhistory periods 128件、history samples
100,000件、threads 256件、models 3件を上限とし、どれか一つでも超えたcandidate全体を拒否する。

トップレベルでは `api_version`、`state`、`observed_at`、`authenticated`、
`plan_label`、`quota`、`models`、`active_thread_count` の全キーが必須である。
`observed_at`、`plan_label`、`quota` だけは `null` を許す。未知キー、大小文字の違う
キー、必須キーの欠落、型違い、任意の object 階層での重複キーを拒否する。

| field | 受理する値 |
| --- | --- |
| `api_version` | 正確に文字列 `v1` |
| `state` | `initializing` / `ready` / `auth_required` / `error` |
| `observed_at`, `reset_at` | `null`（`observed_at`のみ）または JSON 整数の Unix 秒 `1..253402300799` |
| `plan_label` | `null` または改行・control・bidi formattingを含まない1〜64 Unicode scalar |
| `remaining_percent` | 有限 JSON number の `0..100` |
| `window_seconds` | JSON 整数 `1..Int64.MaxValue` |
| token / active thread count | JSON 非負整数 `0..UInt64.MaxValue` |
| model | 最大3件、重複なしの `SOL` / `TERRA` / `LUNA` |

`quota` が object なら `remaining_percent`、`reset_at`、`window_seconds`、`monthly` を
すべて必須かつ null 不可とする。各 model object も `name`、`input_tokens`、
`cached_input_tokens`、`output_tokens` をすべて必須かつ null 不可とする。整数値に
`1.0`、指数表現、文字列を使うことは許可しない。検証失敗の本文や例外の生メッセージは
画面・ログに表示しない。

## 後段の実装・証拠取得

preview、Windows実機、UI画像、contract gate、物理入力smoke、build/test、process/DB/host監査は後段の
独立受入で行う。対象artifact SHAとfresh証拠が未取得の間は、通常・auth・error・DPI・SSH/WSL・installerの
どの状態も製品PASSへ変換しない。未確定の実行path、placeholder、service commandをこの抽出文書から推測して
実行してはならない。

## Windows 配布時のライセンス通知

通常顧客向け配布は `win-x64` のself-contained client payloadを内蔵した、self-contained
single-file installerに固定する。clean supported Windowsで別途.NET Desktop Runtime、SDK、
Visual Studio、payload folder、build操作を要求してはならない。Windowsでlocked restore後、
repository既定のinstaller buildだけで最終setup executableを生成する。

installer buildのexact commandはPRODUCT_PENDINGである。後段のrelease gateは、Avalonia/Skia/HarfBuzz/ANGLE、
埋め込みフォント、rootの`THIRD_PARTY_NOTICES.md`と
`LICENSES/`に加え、self-contained payloadへ実際に入った.NET runtimeのライセンス・通知を
manifestから収集し、payloadへ同梱してからinstallerへ埋め込む。必要なruntime/packageの
通知、version、hashが一つでも欠ける場合はbuildまたは配布gateをFAILとし、その成果物を
顧客へ渡さない。ライセンス一覧と版は[第三者ライセンス通知](../THIRD_PARTY_NOTICES.md)を参照する。

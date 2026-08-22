# Windows client runtime evidence (2026-08-22)

Linux/WSLの評価環境へ.NET SDK 10.0.400を一時導入し、Windows Avalonia clientを
`net10.0` と `win-x64` publish対象として検証した。SDKはリポジトリ外の一時ディレクトリで、
ユーザー設定や認証情報は変更していない。

```text
dotnet test windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
Core:         28 passed / 0 failed
Presentation: 41 passed / 0 failed
```

Windows installer project build:

```text
dotnet build windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
Build succeeded; 0 warnings; 0 errors
dotnet publish windows-client/installer/CodexInfo.WindowsClient.Installer.csproj --runtime win-x64 \
  --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true \
  -p:PayloadZip=<published-client-payload.zip>
CodexInfo.WindowsClient.Setup.exe created with embedded Payload.zip

全ボーダーレス画面の実移動は、同一インストール成果物のfresh processで
`windows_window_move_smoke.ps1`を実行し、Main/Setup/Graph/Threads/Legal/Settingsの6件が
矩形変更を伴う`window-move: PASS`となった。
```

dotnet build windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
Build succeeded; 0 warnings; 0 errors

dotnet publish windows-client/src/CodexInfo.WindowsClient --no-restore --configuration Release \
  --runtime win-x64 --self-contained false --output <temporary>
Publish succeeded.
```

X11上ではfresh processの新規XID（Setup `0x1200012`、Main `0x120000f`）から、
初期設定（760x680）と通常画面（900x720）をキャプチャして目視した。初期設定画面は、Startメニュー導線とSSH/APIコピー欄、
接続エラー、認証確認、続行ボタン、太い日本語フォント、キーボード操作用の閉じるボタンを
含めて確認した。通常画面は、状態帯、残量、7セルゲージ、アカウント、履歴/モデル、
実行中Threads、推移/法的通知/設定/更新の各操作を確認した。

設定フローは、認証開始だけでは完了状態へ進めず、認証済みstatus snapshotを再確認してから
完了できることをPresentation fixtureで確認した。保存した画像とSHA-256は次のとおりである。

| 状態 | 画像 | SHA-256 |
| --- | --- | --- |
| native normal 900x480 | [native-normal-900x480.png](visual-2026-08-22/native-normal-900x480.png) | `2138826c156b79bf2197b7591a9aabde1a1375528677f4eabeb66eb0fc271f7f` |
| Windows Setup 760x680 | [windows-setup-760x680.png](visual-2026-08-22/windows-setup-760x680.png) | `7accff5d06566e169f73adfd789f7e548a817510a3b6b8a127158e4837d9ab80` |
| Windows Main 900x720（Setup overlay下のMain） | [windows-main-900x720-with-setup-overlay.png](visual-2026-08-22/windows-main-900x720-with-setup-overlay.png) | `8f09063c24c83d14b3f072fbf141d11afe983958b212a1e9a27e1963433ee7be` |

この節のX11/AvaloniaキャプチャはLinux側の画面であり、実Windowsの受入証拠とは分離して扱う。

## 実Windows画面追補（ホスト DPI-aware capture）

ホスト `DESKTOP-8TFIR5G`（2560x1440、DPI-aware APIで実ウィンドウ領域を取得）で、
インストール済みクライアントを起動して画面をキャプチャした。設定・履歴は変更せず、
設定ファイルは前後バイト列を復元した。

| 状態 | サイズ | 画像 | SHA-256 |
| --- | ---: | --- | --- |
| Windows通常/接続エラー | 1350x1080 | [windows-main-host-normal-dpiaware-1350x1080.png](visual-2026-08-22/windows-main-host-normal-dpiaware-1350x1080.png) | `7ECE27CC1A6EB8863126CF8BB0344149ACF21D7D5087C3618E743E3E45F4BC60` |
| Windows初期設定 | 3840x2160画面 | [windows-setup-host-fullscreen-dpiaware.png](visual-2026-08-22/windows-setup-host-fullscreen-dpiaware.png) | `6D8171E865737402E3B73DB8F5F817A8350B4837DA3AF2A5EC8F537AFCBD5847` |
| Windows Graph（fresh process、window-only） | 1410x960 | [windows-graph-fresh-window-only.png](visual-2026-08-22/windows-graph-fresh-window-only.png) | `22BA1C4DE8A40B02E865EF98110088401DF5A77DB6980D74CA4C6C66A22CAE6A` |
| Windows Threads（fresh process、1000x800） | 1500x1200 | [windows-threads-fresh-1000x800.png](visual-2026-08-22/windows-threads-fresh-1000x800.png) | `6CD0BF03F94E57BBF589EE421C23DA8FFE6802FC36195E84B5E5CB7CD1C00698` |
| Windows Legal（fresh process、1000x760） | 1500x1140 | [windows-legal-fresh-window-only.png](visual-2026-08-22/windows-legal-fresh-window-only.png) | `3202795D17C05F3AEEB7F14BC5E72C4BFB5E97BA9B66855BD257BDCD9EDCE51` |
| Windows Graph minimum（fresh process、700x480 logical） | 1050x720 | [windows-graph-fresh-min-700x480.png](visual-2026-08-22/windows-graph-fresh-min-700x480.png) | `D1CD32F3D9089EEA69D8F0743D7C6E6EAEFC22B6A4D1E826187A3013A9A92EC0` |
| Windows Setup minimum（fresh installed host、SSH direct/config guidance、680x600 logical） | 1020x900 | [windows-setup-fresh-min-680x600.png](visual-2026-08-22/windows-setup-fresh-min-680x600.png) | `1B63E2E143F413A82A94569D5EF2DD0392498F75814785033CF2FDDAB6B6F6D8` |
| Windows Main English（fresh process） | 1350x1080 | [windows-main-en-fresh.png](visual-2026-08-22/windows-main-en-fresh.png) | `71AE069CBB59BC6339C78CE204B7B55198350BD2FD59A8A41054DFF62592DAD4` |
| Windows Main German（fresh process） | 1350x1080 | [windows-main-de-fresh.png](visual-2026-08-22/windows-main-de-fresh.png) | `722599497D9FB9622E5F58296CFD718FAD883CD4BAF478BCD7C106E749CC1CA7` |
| Windows unknown locale fallback（fresh process） | 1350x1080 | [windows-main-xx-fresh.png](visual-2026-08-22/windows-main-xx-fresh.png) | `71AE069CBB59BC6339C78CE204B7B55198350BD2FD59A8A41054DFF62592DAD4` |
| Windows Settings（fresh process、scrollable） | 930x930 | [windows-settings-final.png](visual-2026-08-22/windows-settings-final.png) | `8B618F32ECA0808EE90FA17749A3955E6476DA5B6CF896387030F2FEDFB3DED6` |
| Windows quota warning（fresh process、10%） | 1350x1080 | [windows-main-warning-fixed.png](visual-2026-08-22/windows-main-warning-fixed.png) | `CE2F7D6145BBEA31224DDFF1E9C975782B9B3A0C20E0C71E4D53236ECD437201` |
| Windows quota critical（fresh process、2%） | 1350x1080 | [windows-main-danger-fixed.png](visual-2026-08-22/windows-main-danger-fixed.png) | `2078CF43FF6C7B7DDD2E74DD214DE871FCBBA45E55F44EACE2541A23E74B52F0` |
| Windows quota zero（fresh process、0%） | 1350x1080 | [windows-main-zero-fixed.png](visual-2026-08-22/windows-main-zero-fixed.png) | `8D94D806B1A3382C4B3A711934AB5CEE87A0B83FF3706E7D1DBC749243474DEE` |
| Windows quota full（fresh process、100%） | 1350x1080 | [windows-main-full-fixed.png](visual-2026-08-22/windows-main-full-fixed.png) | `BBFF74032800C0D9FDD55043292DFE592786611EE8B5C6A67304A0E05598A038` |
| Windows API error（fresh process） | 1350x1080 | [windows-main-error-fixed.png](visual-2026-08-22/windows-main-error-fixed.png) | `4E1DFCF84254571629BB627EB3815AB13E10D7006D642C6DA4ED0B81A2D17F0B` |
| Windows auth required（fresh process） | 1350x1080 | [windows-main-auth-fixed.png](visual-2026-08-22/windows-main-auth-fixed.png) | `CBB524E26731332A41513E902813A23ED42ACCC57D14900184D27C088AB58CE6` |
| Windows Setup SSH direct/config guidance（fresh installed host） | 1140x1020 | [windows-setup-ssh-guidance-host.png](visual-2026-08-22/windows-setup-ssh-guidance-host.png) | `229E01E71CA162A65DF4CCD72EF825979A62AA630B6C2FE550EF57C1F132D9FF` |

通常画面はタイトル、法的通知、設定、更新、接続エラー、固定SSH接続先が同一画面に収まり、
初期設定はSSH/APIコマンド、コピー、接続状態、認証確認、続行を確認した。Graphは既存テストプロセスを
停止した後の単一fresh processからwindow-onlyで取得し、期間・指標・系列切替、軸、系列ラベル、重なりのない時刻ラベルを確認した。
Threadsは親・子・孤立スレッド、深さ・モデル・コンテキスト・トークン・経過時間を表示し、1000x800のfresh画像でクリップなしを確認した。LegalはGPL、フォント、API、スキーマ、第三者通知、配布手順を同一スクロール面で確認した。
高DPIはDPI-aware APIによる実Windows画面（2560x1440）と論理最小サイズ画像で確認し、キーボード操作はStart Menu起動後の8回TAB＋ESC smokeで確認した。ブラウザ認証は実資格情報を扱わず、`wsl.exe -- codex login`起動・認証済みstatus再確認・未認証時の完了拒否をfixture/contractで確認する境界とした。英語・ドイツ語を実Windowsで表示し、未知locale `xx` が英語画像と同一SHA-256になる決定的fallbackを確認した。設定ファイルは実行前後でバイト列を復元した。

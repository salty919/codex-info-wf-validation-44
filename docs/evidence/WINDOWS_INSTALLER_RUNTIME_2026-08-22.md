# Windows installer evidence (2026-08-22)

The installer is now a real executable, not only a shortcut helper.

Implemented contract:

- `CodexInfo.WindowsClient.Setup.exe` is a single-file self-contained setup program.
- The published client and third-party notices are embedded as `Payload.zip`.
- Install is per-user under `%LOCALAPPDATA%\Programs\Codex Info Monitor` and does not require elevation.
- Start-menu shortcut and HKCU Apps/Uninstall registration are created only after the staged payload is validated and moved into place.
- The installed `CodexInfo.WindowsClient.Uninstaller.exe` removes the installed binaries, shortcuts, and registration.
- Client settings and Linux/server history are preserved; settings deletion is only available through the explicit `--purge-settings` option.
- A failed update keeps the previous installed generation and does not publish a partial shortcut.

Linux/WSL build evidence:

```text
dotnet restore windows-client/CodexInfo.WindowsClient.sln --locked-mode  PASS
dotnet build windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release  PASS (0 warnings, 0 errors)
dotnet publish self-contained client + installer ... --runtime win-x64 --self-contained true \
  -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:PayloadZip=<zip>  PASS
artifact: CodexInfo.WindowsClient.Setup.exe, 169796719 bytes
embedded resource marker: CodexInfo.WindowsClient.Installer.Payload.zip (1)
```

Host Windows smoke evidence (WSL2 interop, Windows host `DESKTOP-8TFIR5G`,
2026-08-22) is now available:

```text
Setup.exe executed from C:\Users\salty\Downloads\codex-info-test: ExitCode=0
per-user install: %LOCALAPPDATA%\Programs\Codex Info Monitor
client and installed uninstaller: present
Start-menu shortcut target/working directory: installed client/install directory
Desktop shortcut target/working directory: installed client/install directory
Start-menu target process: started and stopped successfully
uninstaller: ExitCode=0
after uninstall: install directory, shortcuts, and HKCU uninstall key absent
settings.json sentinel after uninstall: preserved
```

The first host run exposed a staging-path shortcut defect; `Program.cs` was
fixed to create shortcuts only after the staging directory is moved, and the
updated installer was rebuilt and rerun before recording the passing evidence.

## 追補（2026-08-22、SetupWindowの横溢れ修正後）

最新ビルドは `79f5afb374f58be082dda0268dfded2d2670f152262a09e860b9231b3ea40125`
（169,796,719 bytes）である。PowerShell 5.1 のホスト実機で旧世代を
`CodexInfo.WindowsClient.Uninstaller.exe --uninstall` により除去した後、引数なしの
Setupを実行した。`/quiet` は本インストーラーの契約外であり、使用していない。

```text
uninstall_old_exit=0
setup_exit=0
installed=True
startmenu=True
uninstaller=True
Start-menu target=C:\Users\salty\AppData\Local\Programs\Codex Info Monitor\CodexInfo.WindowsClient.exe
Start-menu working_directory=C:\Users\salty\AppData\Local\Programs\Codex Info Monitor
started_pid=29080; stopped=1
```

その後、最終成果物（`79f5afb374f58be082dda0268dfded2d2670f152262a09e860b9231b3ea40125`、
169,796,719 bytes）を同じホストへ再配置し、旧世代アンインストール→新規インストール→
Startメニュー起動を再実行した。

```text
uninstall_old_exit=0
setup_exit=0
target=C:\Users\salty\AppData\Local\Programs\Codex Info Monitor\CodexInfo.WindowsClient.exe
working_directory=C:\Users\salty\AppData\Local\Programs\Codex Info Monitor
started_pid=17500; stopped=1
uninstaller=True
```

この追補はインストール、Startメニュー経由の起動・停止、登録済みアンインストーラー
の存在を確認したもの。Graph/Threads/Legal全状態、高DPI、locale、キーボード操作は
同一成果物のWindows受入E2Eでfresh画像SHAとsmokeを確認済み。ブラウザ認証は資格情報を
扱わず、WSL login起動・認証済みstatus再確認・未認証完了拒否を安全境界として確認した。

## 追補2（2026-08-22、Settingsの横溢れ修正後）

`WindowDragBehavior`を画面絶対座標へ修正してclean publishし直した最新成果物のSHA-256は
`282093b1f973d4da610efa8eedfe48fa6864d63b3e496f5a632e06d7562f5536`である。
ホストWindowsへ再インストールし、Startメニューの実ショートカットを直接起動して停止した。

```text
target=C:\Users\salty\AppData\Local\Programs\Codex Info Monitor\CodexInfo.WindowsClient.exe
working=C:\Users\salty\AppData\Local\Programs\Codex Info Monitor
pid=11208 running=True
stopped=True
uninstaller=True
```

Settingsは620x620 logicalへ拡張し、長い外観説明をGrid列＋折返し、内容をScrollViewerへ分離した。
fresh host画像 [windows-settings-latest.png](visual-2026-08-22/windows-settings-latest.png) で、
言語/timezone、接続状態、認証確認、初期設定・法的通知・保存ボタンとスクロール領域を確認した。

## B2B更新／rollback／設定保持（fresh Windows PowerShell）

実ホスト `DESKTOP-8TFIR5G` の Windows 11 (`10.0.26200.0`) で、生成済み
`C:\Users\salty\Downloads\codex-info-test\CodexInfo.WindowsClient.Setup.exe`
を使い、2026-08-22 12:13:21 JST に fresh PowerShell 5.1 transcript を取得した。
配布物の SHA-256 は次のとおりである。

```text
Setup.exe  3818DEFEB93046D947DDAA165F80105C361C155EAAFF242DFFE01FC2691B814F
```

インストーラーが安全上許可するインストール先は現在ユーザーの
`%LOCALAPPDATA%\Programs\Codex Info Monitor` に固定されているため、別の
インストール先／Windowsユーザーへは分離できない。既存インストールを事前に
確認し、`%LOCALAPPDATA%\CodexInfo\settings.json` は読み取りと SHA-256 比較だけに
限定した。更新経路には一時センチネルを置き、DB・履歴・設定の削除や再生成は行って
いない。試験終了時は同じ Setup.exe で clean generation に戻した。

| シナリオ | 実機結果 | 証拠 |
| --- | --- | --- |
| 更新成功 | `ExitCode=0`。旧世代へ置いた `b2b-update-success-sentinel.txt` が消え、クライアント本体、Start Menuショートカット、HKCUアンインストール登録が存在 | raw transcript の `update-success` と各 `PASS` |
| 更新失敗／rollback | Start Menu の `Codex Info` ディレクトリを一時退避し、同名の通常ファイルを置く安全な衝突注入。ショートカット作成で `ExitCode=1`、旧世代の `b2b-rollback-sentinel.txt` と本体 SHA-256 が復元され、staging／previous-generation残骸なし | raw transcript の `rollback-on-shortcut-conflict` と `SETUP STDERR` |
| 設定保持 | 前後 SHA-256 が一致 (`4818DE1C...0F09E6`)。成功更新、rollback、最終clean installの各段階で一致 | raw transcript の `SETTINGS_SHA256_BEFORE/AFTER` と各 `PASS` |

実行ログ全文: [WINDOWS_INSTALLER_B2B_RUNTIME_2026-08-22.txt](WINDOWS_INSTALLER_B2B_RUNTIME_2026-08-22.txt)

## 追補3（2026-08-22、グラフ累積値・欠測値・初回SSH導線修正後）

固定.NET 10 SDKでwin-x64 self-contained payloadと単一ファイルSetupを再発行した。
最新SetupのSHA-256は
`b5cccbb2b949b6c57a8641b0beb96374315b5004e97bd0c3fe67ed16237b563d`である。ホストWindowsへ同一ファイルを再インストールし、Start-menuショートカットの更新と起動を確認した。

物理マウスを勝手に動かさないため、移動スモークは
`-AllowPhysicalInput`指定時だけ実行する契約へ変更した。未指定での受入ゲートはカーソル操作を行わず、移動の証拠は未検証として扱う。

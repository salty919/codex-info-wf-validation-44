# Windows クライアント現行版受入エビデンス（2026-08-22）

この記録は、グラフ修正を含む現行ソースから再発行した Windows クライアント
（publish apphost SHA-256 `34e235709b56b8de1edad8e40262512c4691bcce884b3b2b50276cec9f395568`、
managed client DLL SHA-256 `84baa617f82bce3e981a2173d4041581939f930b94c23b62c0d8089afcf3f430`）に
対するものです。過去版の画像・ハッシュは現行版の受入根拠に使用しません。

## ビルドと契約ゲート

固定 SDK `/home/salty/.codex_info_dotnet_sdk/dotnet` で次を実行しました。

```text
dotnet test windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
Core:         28/28 PASS
Presentation: 57/57 PASS
合計:         85/85 PASS

bash scripts/windows_client_contract_gate.sh
windows-client-contract-gate: PASS
```

## 現行インストーラ

```text
artifact: artifacts/windows-installer-20260822-5/CodexInfo.WindowsClient.Setup.exe
SHA-256:  8bccb053d6883b2079760c3a4a08d908dd401fd5300a7d6250f6484bf8ff59f4
host:     C:\Users\salty\Downloads\codex-info-staging\CodexInfo.WindowsClient.Setup.exe
```

## ホスト Windows 実測

現行インストーラで一度アンインストールし、設定ファイルを保持したまま再インストールしました。

```text
uninstallExit=0
installDirExists=False
shortcutExists=False
registryExists=False
settingsExists=True

installExit=0
exeExists=True
exeSha=34e235709b56b8de1edad8e40262512c4691bcce884b3b2b50276cec9f395568
managed client DLL SHA-256=84baa617f82bce3e981a2173d4041581939f930b94c23b62c0d8089afcf3f430
core DLL SHA-256=11c8f3b3be5d051585118b88bffa69bf8c45fdb87e27815199f0af49f3fa2853
shortcutExists=True
registryExists=True
settingsExists=True
settings SHA-256 before/after:
7e88111236341698a2ed444fef9dba4b300645700fb685607450c19f64504509
```

Start Menu ショートカットは次を指します。

```text
C:\Users\salty\AppData\Local\Programs\Codex Info Monitor\CodexInfo.WindowsClient.exe
WorkingDirectory:
C:\Users\salty\AppData\Local\Programs\Codex Info Monitor
```

Start Menu の `.lnk` を `Start-Process` で起動し、現行実行ファイルのプロセスを確認しました。

```text
Path: C:\Users\salty\AppData\Local\Programs\Codex Info Monitor\CodexInfo.WindowsClient.exe
MainWindowTitle: Codex Info Monitor
```

## 現行実画像

すべて現行インストールから新規プロセスを起動し、マウス入力を注入せずにキャプチャしました。

| 状態 | 画像 | SHA-256 |
| --- | --- | --- |
| 通常 | [windows-main-normal-v3.png](visual-2026-08-22-current-v3-full/windows-main-normal-v3.png) | `6c6108bc6f60c3cb6e7c9c38b416b950e338bd066aeeffc83e1ae0d82b1c9a05` |
| 警告/危険/0%/100% | [visual-2026-08-22-current-v3-full](visual-2026-08-22-current-v3-full/) | `76e6efe1667c3276f5dde11bf9286b5cf20979b35d4068ace8a7ba7ace19e6fb` / `59ddd0b53cf346811cfc59154be03970cb0575456ec2721d33812a97ae153945` / `57623eb33fb9baad4f93cfbc7d90d2600fb3ccb1bfaec6d05034c29060c48a1a` / `534944b488b7be8216ef7ce25be53a761ff9a56d038f941a8ed15b944b53da5e` |
| 認証/エラー | [visual-2026-08-22-current-v3-full](visual-2026-08-22-current-v3-full/) | `cbb524e26731332a41513e902813a23ed42accc57d14900184d27c088ab58ce6` / `4e1dfcf84254571629bb627eb3815ab13e10d7006d642c6da4ed0b81a2d17f0b` |
| グラフ（残量アイドル補正・初回段差後） | [windows-graph-current-v3.png](visual-2026-08-22-current-v3-full/windows-graph-current-v3.png) | `75b9b65f6b7c7699ddcf2c1f8f8a4abf9656a48033192d4d41e083ee364c75c6` |
| スレッド/設定/セットアップ/法的通知 | [visual-2026-08-22-current-v3-full](visual-2026-08-22-current-v3-full/) | `6cd0bf03f94e57bbf589ee421c23da8ffe680fc36195e84b5e5cb7cd1c00698` / `d93d5846e3c61c3b93b27e846d13cd585848c840146af51fa729f9e5005ea766` / `c5cd27d4201a3663af497282a40f3e88af9becddf4d1c6c553d5778f7d6552a5` / `7bb7f6132f14af27bb9c3a6ed9b45c7eff180512fc3e59f64dc8d5b8be4151a3` |

## 未完了ゲート

- 実マウスを動かす物理ドラッグ試験は、ユーザーの「マウスを勝手に奪わない」要求により実行していません。静的 API 禁止検査、ドラッグ単体テスト、画像キャプチャは実施済みですが、物理入力の受入は `INCONCLUSIVE` です。
- 独立評価担当による現行 SHA の最終判定待ちです。
- したがって、この文書だけでは Windows 版を 100% 完了とは扱いません。

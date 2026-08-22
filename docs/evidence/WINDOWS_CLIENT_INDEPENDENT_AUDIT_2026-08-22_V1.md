# Windows client independent audit V1

監査日: 2026-08-22
対象: 現行 `windows-client` ソース、`artifacts/windows-client-publish-20260822-3`、`artifacts/windows-installer-20260822-3`、ホスト上の per-user インストールと Start Menu ショートカット
監査担当: Luna 独立評価

## 判定

INCONCLUSIVE / HOLD

ソース上の機能パリティとインストール導線は確認でき、公開成果物とホストインストール済み実行ファイルの SHA-256 も一致した。しかし、現行成果物 SHA に結び付いた実画像、Windows 上の起動実測、ならびに要求された権威ある 83/83 テスト実行ログを確認できない。したがって PASS または完了扱いにはしない。

## 確認した証拠

### 成果物とホスト同一性

| 対象 | SHA-256 | 結果 |
|---|---|---|
| `artifacts/windows-client-publish-20260822-3/CodexInfo.WindowsClient.exe` | `34e235709b56b8de1edad8e40262512c4691bcce884b3b2b50276cec9f395568` | 記録済み |
| `artifacts/windows-client-publish-20260822-3/CodexInfo.WindowsClient.dll` | `29704db181314a4efbddee315fd2df0c25b7d4d150879c61728687083677e6b8` | 記録済み |
| `artifacts/windows-client-publish-20260822-3/CodexInfo.WindowsClient.Core.dll` | `11c8f3b3be5d051585118b88bffa69bf8c45fdb87e27815199f0af49f3fa2853` | 記録済み |
| `artifacts/windows-installer-20260822-3/CodexInfo.WindowsClient.Setup.exe` | `bb4ebdd730fb92b18a24733a638b439f75635a0a21b3d94e0077d480c67dab08` | 記録済み |
| ホスト `C:/Users/salty/AppData/Local/Programs/Codex Info Monitor/CodexInfo.WindowsClient.exe` | `34e235709b56b8de1edad8e40262512c4691bcce884b3b2b50276cec9f395568` | publish exe と一致 |

ホストでは以下を PowerShell で測定した。

- インストール先は `C:/Users/salty/AppData/Local/Programs/Codex Info Monitor`。
- Start Menu は `C:/Users/salty/AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Codex Info/Codex Info Monitor.lnk`。
- ショートカットの Target はインストール済み `CodexInfo.WindowsClient.exe`、WorkingDirectory はインストール先、Arguments は空、Icon は同 exe の index 0。
- これは導線の静的な実体確認であり、ショートカットから新規プロセスが起動したことの証拠ではない。

### X 版との機能パリティ（静的評価）

`windows-client/Controls/GraphPlotControl.cs`、`windows-client/ViewModels/DetailsWindowViewModels.cs`、`windows-client/Views/GraphWindow.axaml` を確認した。

- 系列: SOL、TERRA、LUNA の累積系列と remaining の線・マーカーを描画するプロパティとレンダリング経路がある。
- 期間: period collection、selected period、期間内へのフィルタ、current period の end clamp、period 境界ラベルがある。
- 残量: remaining の値を描画し、欠測値は `—` として扱う経路がある。
- 欠測・idle: sparse sample の補間/フィルタと idle interval の帯を構築し、初回観測および reset 境界を扱う実装がある。
- トグル: remaining、models、SOL/TERRA/LUNA 各系列、Tokens/Dollars metric の bindable state があり、切替時に再計算・通知する経路がある。

これはコード契約に対する PASS 相当の静的所見である。ただし、実画像での系列表示、期間切替、欠測表示、idle の塗り、塗り方向、トグル後の視覚結果を確認できていないため、受入判定は INCONCLUSIVE のままとする。

### Windows 導入・アンインストール導線（静的評価）

`windows-client/installer/Program.cs` を確認した。

- `%LOCALAPPDATA%/Programs/Codex Info Monitor` 以下への per-user install を検証する。
- staging、既存インストールの backup、atomic move、失敗時の復元を行う。
- Start Menu shortcut を Target/WorkingDirectory/Icon 付きで作成し、任意の Desktop shortcut と per-user uninstall registry metadata を扱う。
- uninstall は shortcut と registry を除去し、既定では settings/history を保持し、`--purge-settings` 時のみ設定削除を選べる。

ソース導線は要件に整合するが、実インストール→起動→アンインストール→再起動後の導線を一連で実測していないため、導入受入は INCONCLUSIVE である。

## テストゲート

| ゲート | 結果 | 根拠 |
|---|---|---|
| Windows client 83/83 | INCONCLUSIVE | 権威ある 83/83 の実行ログを取得できなかった。ソース属性の機械的スキャンは Core 20、Presentation 43、計 63 であり、83/83 の代替証拠ではない。 |
| `dotnet test` | INCONCLUSIVE | WSL で `dotnet test windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release` を実行したが、`dotnet: command not found`（status 127）。Windows 側のテスト完了ログは今回の監査証拠に存在しない。 |
| 起動経路 | INCONCLUSIVE | ショートカット Target 等は実測したが、現行 SHA の新規プロセス起動結果、終了結果、ログを取得していない。 |
| 実画像 | INCONCLUSIVE | 指定 publish/installer artifact roots の確認範囲に、現行成果物 SHA に結び付く graph/details の PNG 等の実画像がない。 |

## 未検証事項と受入ブロッカー

1. 現行 `windows-client-publish-20260822-3` の exe SHA に結び付く Windows 実機画像。少なくとも graph の系列/期間/remaining/欠測/idle/トグル状態を含む必要がある。
2. Windows 上での同一成果物に対する 83/83 テストの生ログ（テスト数と対象 SHA が同じ証拠に紐づくこと）。
3. Start Menu shortcut または installer 起動経路からの新規プロセス実測、正常終了、必要なら再起動後の導線。
4. 実導入後の uninstall と settings/history 保持、`--purge-settings` の明示的な削除導線。

上記が揃うまで、Windows クライアントの完了・PASS・100%要求達成は報告不可とする。今回の監査ではコード変更、インストール済みデータの削除、アンインストール操作は行っていない。

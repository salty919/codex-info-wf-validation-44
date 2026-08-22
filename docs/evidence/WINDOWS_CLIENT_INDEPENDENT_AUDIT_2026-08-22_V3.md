# Windows client independent audit V3

監査日: 2026-08-22
監査担当: Luna 独立評価
対象: 現行 windows-client ソース、publish-5 / installer-5、current-v3 画像、再インストール済みホスト

## 分割判定

| 領域 | 判定 |
|---|---|
| 現行コード・機能パリティ | PASS（静的/テスト） |
| publish-5 / installer-5 / 画像 SHA | PASS |
| 固定 SDK 85/85 と contract/E2E | PASS |
| Windows Start Menu 起動・keyboard smoke | PASS |
| physical cursor move smoke | INCONCLUSIVE（ユーザー要求で SKIP） |
| 総合受入 | **INCONCLUSIVE / HOLD** |

physical cursor test が未実行であるため、機能・成果物ゲートが PASS でも総合 PASS にはしない。

## SHA とホスト同一性

| 対象 | SHA-256 | 結果 |
|---|---|---|
| `artifacts/windows-client-publish-20260822-5/CodexInfo.WindowsClient.exe` | `34e235709b56b8de1edad8e40262512c4691bcce884b3b2b50276cec9f395568` | 実ファイル確認 |
| `artifacts/windows-client-publish-20260822-5/CodexInfo.WindowsClient.dll` | `84baa617f82bce3e981a2173d4041581939f930b94c23b62c0d8089afcf3f430` | 実ファイル確認 |
| `artifacts/windows-client-publish-20260822-5/CodexInfo.WindowsClient.Core.dll` | `11c8f3b3be5d051585118b88bffa69bf8c45fdb87e27815199f0af49f3fa2853` | 実ファイル確認 |
| `artifacts/windows-installer-20260822-5/CodexInfo.WindowsClient.Setup.exe` | `8bccb053d6883b2079760c3a4a08d908dd401fd5300a7d6250f6484bf8ff59f4` | 実ファイル確認 |
| `docs/evidence/visual-2026-08-22-current-v3/windows-graph-current-v3.png` | `75b9b65f6b7c7699ddcf2c1f8f8a4abf9656a48033192d4d41e083ee364c75c6` | 実ファイル確認 |

PowerShell で `C:/Users/salty/AppData/Local/Programs/Codex Info Monitor` の exe/DLL/Core DLL を測定し、publish-5 と一致した。Start Menu shortcut の Target は同インストール先 exe、WorkingDirectory は同ディレクトリ、Arguments は空、Icon は exe index 0 だった。
PowerShell uninstall inventory で登録を確認した: REGISTRY_MATCH Path=Microsoft.PowerShell.Core\Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\CodexInfo.WindowsClient DisplayName=Codex Info Monitor UninstallString="C:\Users\salty\AppData\Local\Programs\Codex Info Monitor\CodexInfo.WindowsClient.Uninstaller.exe" --uninstall

## テスト・E2E

実行: `bash scripts/windows_acceptance_e2e.sh`。固定 SDK `/home/salty/.codex_info_dotnet_sdk/dotnet` が存在し、終了コード 0。

- Core: 28/28 PASS
- Presentation: 57/57 PASS
- 合計: 85/85 PASS
- `windows-client-contract-gate`: PASS
- `windows-start-menu-smoke`: PASS
- `windows-keyboard-focus-smoke`: PASS
- `windows-physical-move-smoke`: SKIP（`CODEX_INFO_ALLOW_PHYSICAL_INPUT=1` の明示的 opt-in）
- `windows-acceptance-e2e`: PASS

E2E は `visual-2026-08-22-current-v3-full` の画像群と SHA、installer-5、現行文書契約、Start Menu 起動、keyboard traversal を検証した。physical cursor move だけは意図的に実行していない。

## GraphPlot / DetailsViewModel 静的評価

`GraphPlotControl.cs`、`DetailsWindowViewModels.cs`、`GraphWindow.axaml`、installer `Program.cs` と Presentation tests を確認した。

- SOL / TERRA / LUNA の累積系列、remaining 線・マーカー、period 選択、Tokens/Dollars、remaining と各系列の toggle を持つ。
- period 内フィルタ、current period の end clamp、モデル累積値の重複時 max、remaining 欠測の `—` を実装する。
- idle 区間はモデル累積値が変化しない区間として band 化し、reset anchor から初回観測までの境界を保持する。
- idle 中の quota 低下を消費と解釈せず、活動区間だけで remaining の補間を行う。初回段差を保持するため、idle の水平線から活動開始点へ段差が現れる。
- installer は per-user staging/backup/atomic move、Start Menu shortcut、uninstall と settings/history 保持および `--purge-settings` 分岐を持つ。

## 現行画像の目視

画像は 1410x960 RGBA。Current period、概算ドル、remaining、LUNA/TERRA/SOL の toggle、3 系列、remaining の下降線、左側 idle shading、活動開始時の初回段差、軸ラベルを確認した。v2 にあった初回区間の段差欠落は v3 画像では解消しているように見え、明白なクリップ・塗り方向・系列色崩れはない。

この 1 枚には欠測 remaining の表示状態は含まれないため、欠測の視覚確認は未完了。ただしコード、85/85 tests、E2E の現行画像ハッシュ契約は確認済みである。

## 未完了事項

1. physical cursor move smoke はユーザー要求により SKIP。物理ポインタ受入を要求する台帳行は INCONCLUSIVE のままにする。
2. 欠測状態の現行 SHA 結び付け画像を独立目視する場合は追加確認が必要。
3. 上記の未確認事項があるため、Windows クライアントを 100% 完了または総合 PASS と報告してはならない。

コード変更は行っていない。

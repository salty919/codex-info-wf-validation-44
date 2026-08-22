# Windows client independent audit V2

監査日: 2026-08-22
監査担当: Luna 独立評価
対象: 現行 windows-client ソース、publish-4 / installer-4、現行 v2 画像、ホスト再インストール後状態

## 判定

**INCONCLUSIVE / HOLD**

前回の画像不足は解消した。現行 v2 グラフ画像を目視し、E2E スクリプトも現行画像ハッシュ、installer identity、Start Menu 起動、keyboard smoke まで PASS した。一方、親の記録が示す Core 28 + Presentation 56 = 84 件に対し、同じ固定 SDK で今回再実行した実測は Core 28 + Presentation 57 = **85 passed** だった。機能失敗ではないが、要求台帳・証拠文書と raw test count の不一致であり、独立監査として PASS に昇格しない。

## 成果物と SHA

| 対象 | SHA-256 | 判定 |
|---|---|---|
| publish-4 `CodexInfo.WindowsClient.dll` | `2b75b2f2f356d2cff47c60403d523e9655633429f989932fe25dfab55baca35e` | 実ファイル一致 |
| publish-4 `CodexInfo.WindowsClient.exe` | `34e235709b56b8de1edad8e40262512c4691bcce884b3b2b50276cec9f395568` | 記録済み |
| publish-4 `CodexInfo.WindowsClient.Core.dll` | `11c8f3b3be5d051585118b88bffa69bf8c45fdb87e27815199f0af49f3fa2853` | 記録済み |
| installer-4 `CodexInfo.WindowsClient.Setup.exe` | `db8c8b0502aca60163352f579356c066d4d93d62151f50d1642f35131aceb7cb` | 実ファイル一致 |
| 現行 graph image | `d87c0fe2b4c1a2adc422e0270da7b1adbf085f8e96ded10e244bd3e2991948b0` | 実ファイル一致 |

ホストの `C:/Users/salty/AppData/Local/Programs/Codex Info Monitor` と Start Menu shortcut を PowerShell で再測定した。インストール済み exe/DLL のハッシュ、Target、WorkingDirectory、Arguments、Icon、HKCU uninstall registration は、現行 publish/installer の導線と整合した。

## 固定 SDK テストと E2E

実行コマンド: `bash scripts/windows_acceptance_e2e.sh`

終了コードは 0。固定 SDK が使用され、raw output は次の通りだった。

- Core: 28 passed, 0 failed
- Presentation: 57 passed, 0 failed
- 合計: 85 passed, 0 failed
- `windows-client-contract-gate`: PASS
- `windows-start-menu-smoke`: PASS
- `windows-keyboard-focus-smoke`: PASS
- `windows-physical-move-smoke`: SKIP（`CODEX_INFO_ALLOW_PHYSICAL_INPUT=1` の明示的 opt-in が必要）
- `windows-acceptance-e2e`: PASS

スクリプトは `visual-2026-08-22-current-v2-full` の 12 画像と各 SHA、installer-4 の SHA、現行文書契約を検証している。ただし、raw 85 件と `WINDOWS_CLIENT_CURRENT_2026-08-22.md` 等の 28/28 + 56/56 = 84 件の記録が一致していない。これは台帳/証拠整合性の受入ブロッカーである。

## グラフ機能パリティ

`GraphPlotControl.cs`、`DetailsWindowViewModels.cs`、`GraphWindow.axaml` と Presentation テストを確認した。

- SOL / TERRA / LUNA の系列、remaining 線・マーカー、期間選択、Tokens/Dollars 切替、各系列トグルが存在する。
- period 内のフィルタ、current period の end clamp、軸・境界ラベルがある。
- 欠測 remaining は `—` として扱う経路がある。
- idle 区間は activity sample から構築され、初回観測/reset 境界を扱う。今回の更新では idle 中の quota 低下をグラフの活動量として誤表示しない補正と、X 版に合わせた活動区間の補間・平滑化が入っている。
- 現行 graph image は 1410x960 RGBA。目視上、Current period、概算ドル、remaining、LUNA/TERRA/SOL の全系列トグル、3 系列、remaining の下降線、左側 idle shading、軸ラベルを確認でき、表示方向・系列色・レイアウトに明白な崩れはない。

画像に欠測マーカーを含む状態はこの 1 枚では表示されていないため、欠測の視覚受入はコード/テストと画像ハッシュによる補助確認に留まり、全状態の独立目視 PASS とはしない。

## Windows 導入・起動・アンインストール

`installer/Program.cs` の per-user install、staging/backup/atomic move、Start Menu shortcut、HKCU uninstall registration、既定の settings/history 保持と `--purge-settings` 分岐を静的確認した。現行文書には `uninstallExit=0` があり、ホスト再インストール後の exe/DLL、shortcut、uninstall registration を PowerShell で確認できた。Start Menu からの新規起動と keyboard focus smoke は今回の E2E 実行で PASS だった。

physical cursor move smoke は明示的 opt-in により SKIP であり、その実測は未検証として残る。ユーザー指定の SKIP 方針には従うが、物理入力までを含む完了判定には使用しない。

## 受入ブロッカー

1. Core 28 + Presentation 56 = 84 と記録された証拠と、今回の同一固定 SDK による 28 + 57 = 85 の raw 出力を統一し、対象成果物 SHA とテスト数を同一証拠へ固定すること。
2. 欠測表示を含む graph 状態の現行 SHA 結び付け画像を目視確認すること（現 graph 画像は idle/系列/remaining/トグルを確認済み）。
3. physical move smoke は明示的 SKIP のままなら、その扱いを受入台帳に明記すること。

以上により、前回 V1 の「画像なし・テスト未実行」は更新され、現行コード/成果物/起動 E2E の主要ゲートは PASS 相当になった。しかし、テスト件数の証拠不一致が残るため、総合判定は **INCONCLUSIVE / HOLD** とする。コードは変更していない。

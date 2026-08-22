# リリースマニフェスト（納品判定）

## 現在の判定

**RELEASE HOLD / 提出不可**

X版とのグラフ時間軸差分、累積値・欠測値処理、初回SSH導線、破損設定ループを修正した新ビルドを生成した。ただしホストWindowsでは既存PIDが実行ファイルをロックしており、現行ビルドの再インストールは未実施である。物理マウス入力はユーザー操作を奪わないため明示オプトインへ変更した。データ保護ゲートと独立監査が未完了のため、提出不可として固定する。前回の`RELEASE READY`記載は無断仕様差分を含む成果物に対する誤判定であり、撤回する。

## 確認済み成果物

| 項目 | 値 |
| --- | --- |
| Windows installer | `CodexInfo.WindowsClient.Setup.exe` |
| installer SHA-256 | `b5cccbb2b949b6c57a8641b0beb96374315b5004e97bd0c3fe67ed16237b563d` |
| Windows installer実機 | 旧世代の証跡のみ。現行b5cccビルドは既存プロセスのロックで再導入未完了 |
| systemd daemon | user登録 / active / SIGKILL後Restart / DB保持 PASS |
| Rust tests | lib 152 + main 164 + db 1 + security 13 + usage_store 36 PASS |
| Windows .NET tests | Core 28 + Presentation 41 PASS（固定SDK、最新ソース） |

## 出荷ブロッカー

- X版とWindows版のグラフ右端（現行期間を観測時刻まで描画）の同値性を修正後ビルドで再確認する。
- `windows_acceptance_e2e.sh` と `data_protection_gate.sh` を同一SHAで再実行し、close/jitterの再現がないことを確認する。
- 独立評価担当がグラフ仕様差分と全画面UXをraw証拠付きでPASSするまで納品判定を戻さない。
- 物理マウス移動試験はユーザーのカーソルを奪わないため、`-AllowPhysicalInput`指定時だけ実行する。未指定時は未検証として扱う。
- `GOV-SUBAGENT-01`の独立監査台帳と`docs/INDEPENDENT_AUDIT_LATEST.md`が同一SHAで`status: PASS`になるまで、completion guardを通過させない。

## 受入完了条件

このマニフェストは、最新ビルドに対する要求台帳全件、completion guard、data-protection gate、実機証拠、独立B2B監査がすべてPASSした後にのみ`RELEASE READY`となる。今回の判定根拠は[Windows受入E2E](evidence/WINDOWS_ACCEPTANCE_E2E_2026-08-22.md)、[DB保護実行証跡](evidence/DATA_PROTECTION_RUNTIME.md)、[要求監査](REQUIREMENTS_AUDIT_2026-08-22.md)に固定する。

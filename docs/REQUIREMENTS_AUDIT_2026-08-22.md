# 要求実装監査（2026-08-22）

これは、過去の会話で明示された要求を、現時点のコード・起動経路・文書・テストへ突き合わせた未実施項目の正本である。`implemented` はコード片が存在することだけを意味し、実運用・全状態・回帰証拠がないものは `partial` または `unverified` とした。`missing` は要求を満たす実装が確認できなかった項目である。

## 判定の意味

- **implemented**: 実装と対象テストが存在し、今回の監査で確認できた。
- **partial**: 一部の経路だけ実装されている、または要求の境界・失敗時動作が未実装。
- **unverified**: 実装らしきコードはあるが、要求された実画面・実プロセス・実DB状態の証拠がない。
- **missing**: 実装・自動検査・導入経路のいずれも確認できない。

## 未実施・未確認要求一覧

| ID | 過去要求 | 現状判定 | 確認できた事実 | 不足している証拠／実装 |
| --- | --- | --- | --- | --- |
| AUD-001 | 通常版を起動したままアカウント・残量が更新され続けること | implemented | `src/main.rs` のタイマー、account worker、`run.sh`、正常接続とapp-server停止時のruntime traceを確認 | 実アカウントの値そのものは取得しない（秘密情報保護のため）。状態遷移とlast-good保持を証拠とする |
| AUD-002 | 実行中スレッド全件と親子関係を表示すること | verified | `thread/list`、rollout、`ThreadsWindow`、native descendant補完、Windows fresh Threads画像（親・子・孤立、深さ、モデル、context、token、経過時間）、thread fixture tests、native thread contract testsを確認 | 失敗入力はfail-closedで空/前回値を保持する契約をテストで固定。証跡: `WINDOWS_ACCEPTANCE_E2E_2026-08-22.md` |
| AUD-003 | 利用中なのにグラフがidleにならないこと | verified | local JSONL collector、SQLite history、GraphWindow、oversized/recoverable tests、daemon E2Eのtoken増分120→240、fresh Graph画像を同じリリース成果物で確認 | 同一キーの履歴はtransactionalに最大値へ収束し、idle補間と終端保持をnative/Windows testsで固定。証跡: `DATA_PROTECTION_RUNTIME.md`, `WINDOWS_RUNTIME_2026-08-22.md` |
| AUD-004 | 巨大tool出力・不正recordが後続の有効recordを欠損させないこと | implemented | `read_recoverable_session_line`、recoverable rollout parser、focused tests が存在 | 実環境の秘密ログは収集せず、fixtureとfail-closed契約を証拠とする |
| AUD-005 | 既存のマスター可視化データ・期間データを壊さず復旧すること | verified | transaction/upsert、旧DB保持、backup-before-prune、DB保護E2Eのrow/file SHA不変、migration/restore失敗保持を現行releaseで確認 | 運用時の継続監視はrunbookのquick_check/hash手順で固定し、データを捏造する自動復旧は行わない |
| AUD-006 | app-server/RESTが停止している間も、常時記録部が変動を収集すること | implemented | 独立`--record-daemon`、persisted reset hint、bounded JSONL timeline、SQLite upsertを実装。`scripts/record_daemon_e2e.sh`でREST/UI停止後も追記トークンが120→240へ反映されることを確認 | daemon自体が停止中の値は引き続き捏造しない。入力が存在しない期間は欠測として保持する |
| AUD-007 | 監視サーバ側をdaemonとして自動起動すること | implemented | 通常main/REST起動から独立実行ファイルを`--record-daemon`でspawn。E2Eでlock/PID、REST終了後のdaemon継続、TERM後のlock解放を確認。`scripts/install_systemd_recorder.sh`でuser systemdへ実登録し、SIGKILL後の`Restart=on-failure`復旧とDB保持も確認。Python標準SQLiteで10631行、`quick_check=ok`、前後SHA-256一致を取得 | Windows parity・実画面・他のREG項目は統合ゲートで別IDとして完了確認済み |
| AUD-008 | 複数サーバ／複数collectorでも二重登録・衝突を起こさないこと | implemented | SQLite unique/transaction/busy timeout、daemon singleton lock、PID/inode確認、stale lock回収を実装。singleton/concurrent writer testsとE2Eを確認 | 複数collectorの同一サンプルは一意キーとMAX/COALESCEで収束し、捏造行を追加しない |
| AUD-009 | DBを数世代バックアップし、失敗時に元DBを維持すること | implemented | `UsageStore::backup_generations` と3世代・quick_check/reload test、policyが存在 | 定期監視・復元承認・通知先は顧客runbookの事前チェックと責任分界で固定 |
| AUD-010 | migrationで過去データを壊さず安全に切り替えること | implemented | `UsageStore::migrate_verified` がcandidate DB、全行validate、quick_check、fingerprint/期間比較、backup、atomic switch、失敗保持を実装し、成功/不正候補テストが成功。旧schemaは暗黙変換せず拒否 | 特定の未知旧schemaを推測変換することは安全要件に反するため実装しない。変換は呼び出し側の明示transformに限定する |
| AUD-011 | DB保護・回帰防止を仕様と制約として固定し、今後の変更で破壊しないこと | verified | `DATA_PROTECTION_POLICY.md`、統合台帳、data-protection gate、Windows acceptance gate、CI workflow、systemd template、独立監査を同一IDで固定 | canonical ledgerは全行をverifiedへ更新し、次回変更時にgateが再実行される fail-closed 契約を保持 |
| AUD-012 | 記録保護は余計な恒常CPU負荷を発生させないこと | implemented | daemon intervalを5〜3600秒にboundedし、file fingerprintが変化しない周期は全走査・DB書込を行わない。隔離E2Eで5秒間の変更なしCPU ticks=0/100（0%）を測定 | 大規模な実ユーザーデータは読み込まない。入力上限と変更検知を負荷境界の仕様とする |
| AUD-013 | Windows版はX版から機能を一つも落とさないこと | unverified | Main/Graph/Threads/Legal/Setup/Settings、API client、Core 28 / Presentation 41、最新installerを確認。グラフ累積・欠測処理修正後のfresh画像と同一SHA受入が未完了 |
| AUD-014 | Windows版にグラフ、履歴期間、ドル／token切替、モデル表示を備えること | verified | `GraphWindow`、`GraphPlotControl`、history period、metric toggle、Graph fixture tests、SQLite persistence tests、fresh normal/minimum Graph画像を確認 |
| AUD-024 | X版のグラフ時間軸意味を変更しないこと | unverified | `EffectiveGraphEnd`・分バケット・開始/終端アンカーを実装済みだが、現行SHAの独立監査・fresh Graph画像の同値確認が未完了 |
| AUD-015 | Windows版に実行中threadの詳細を表示すること | verified | `ThreadsWindow`、親子/孤児/context/token fixture tests、fresh Threads画像、keyboard smoke、native thread contract testsを確認 |
| AUD-016 | Windows版の認証開始・ブラウザ／WSL導線・認証確認を従来どおり維持すること | verified | `wsl.exe -- codex login`、認証確認ボタン、setup completion guard、auth/error fresh images、safe failure/retry presentation testsを確認 |
| AUD-017 | Windows版はWindowsらしい洗練された見た目・アイコン・太いフォントにすること | verified | Avalonia Fluent theme、Medium font、icons、focus style、tooltips/AutomationProperties、normal/minimum/high-DPI fresh images、keyboard smokeを確認 |
| AUD-018 | Windows版を多言語対応し、設定変更を全画面へ反映すること | verified | `UiText` catalog、settings/timezone persistence、language-change notifications、Core/Presentation tests、en/de/unknown fresh images、Settings final imageを確認 |
| AUD-019 | Windows版はStartメニューから起動すること | verified | 埋め込みpayloadを持つ`CodexInfo.WindowsClient.Setup.exe`をホストWindowsへ実インストールし、Startメニューshortcutのtarget/working directory確認、同shortcut経由のプロセス起動・停止、uninstall後の本体・shortcut・HKCU登録除去を確認。初回staging-path不具合は修正後に再実行 | 実ユーザー資格情報は扱わず、認証状態は別のauth受入画像・fixtureで検証 |
| AUD-020 | 初期導入、Linux/WSL REST起動、SSH転送、設定、再接続の動線を曖昧にしないこと | verified | `WINDOWS_CLIENT.md`、Setup/Settings、copyable SSH/API commands、daemon lifecycle/health手順、setup auth guard tests、fresh Setup/Settings images、installer/Start-menu smokeを確認 |
| AUD-021 | REST側はUIなしで起動し、直接起動でもUIが出ないこと | implemented | `CODEX_INFO_API_LISTEN`→`CODEX_INFO_REST_SILENT`、main側でもUI hide、文書とhealth endpointが存在 | REST/UI/daemonの3プロセス境界はrecord daemon E2EとStart Menu smokeで統合確認済み |
| AUD-022 | 変更中もデグレードを即座に検出し、要求忘れをサブエージェントで確認すること | verified | 統合台帳、completion guard、data-protection gate、Windows acceptance gate、CI workflows、fresh evidence、独立サブエージェント監査を固定 |
| AUD-023 | Windowsの全画面を移動できること | unverified | 過去成果物では6画面の実移動を確認したが、最新成果物ではユーザーのカーソルを奪わないため物理入力を未実行。`-AllowPhysicalInput`を明示した再検証が必要 |
| AUD-025 | SSH確認画面を毎回表示せず通常起動へ遷移できること | implemented | `ClientSettings.ConnectionConfigured`を非機密マーカーとして保存し、MainWindowは未設定時だけSetupを自動表示。設定保存テストとPresentation 41件、同一SHA再導入を確認。実ユーザーの認証環境は扱わず、状態遷移はfixtureで検証 |
| AUD-026 | テストがユーザーのマウスを勝手に動かさないこと | implemented | `windows_window_move_smoke.ps1`は`-AllowPhysicalInput`なしではSKIPし、受入ゲートも明示環境変数なしでは物理入力を行わない。製品コードにSetCursorPos/mouse_eventは存在しない |
| AUD-027 | 破損・途中書込み設定で歓迎画面を毎回表示しないこと | implemented | `SettingsCorrupt`を非永続マーカーとして保持し、破損JSONは安全な未接続状態へ落としてMainWindowのSetup自動表示を抑止。固定SDKでmalformed JSONとShouldOpenSetupを追加確認。ホスト同一SHAでの再起動rawは未取得 |

## 監査で確定した「隠れていた未実施」

初回監査で見つかったdaemon本体、自動起動、singleton、candidate migrationの実装は追加した。以下は実装追加時の記録であり、現行の統合ゲートによる閉鎖判定を上書きしない。

1. **daemonの実プロセスE2Eは追加済み**。REST/UI停止後の継続、JSONL追記、singleton lock、TERM cleanup、変更なし5秒CPU ticksを`record_daemon_e2e.sh`で確認した。
2. **特定旧schemaからの業務migrationは推測実装していない**。代わりに、明示transformをcandidate DBで検証してatomic switchする安全APIと成功/失敗テストを実装した。未知schemaは拒否する。
3. **Windows parityのコード・fixture・X11画像は追加済み**（一時.NET 10 SDKでCore 28 / Presentation 34、win-x64 publish、X11 fresh Setup/Main capture）。ホストWindowsでもfresh Graph/Threads/Legal/Setup、最小Graph/Setup、locale、keyboard、quota全状態のwindow-only画像を取得し、画像とSHA-256を`docs/evidence/visual-2026-08-22/`へ保存した。ブラウザ認証は実資格情報を扱わず、WSL起動・認証済みstatus再確認・未認証完了拒否を契約/fixtureで閉鎖した。
4. **Windows installer lifecycleを追加・実機確認**。`CodexInfo.WindowsClient.Setup.exe`へpublish payloadを埋め込み、per-user配置、Start-menu shortcut、HKCU uninstall registration、installed uninstaller、設定／履歴保持、CI上のinstall/uninstall smokeを実装した。ホストWindowsでも実インストール、Start-menu起動、アンインストール、設定保持を確認した。初回実行で見つかったstaging-pathショートカット不具合は修正し、修正版で再検証した。証拠は`docs/evidence/WINDOWS_INSTALLER_RUNTIME_2026-08-22.md`に記録した。

## 直ちに実装へ戻す順序

```text
要求台帳の統合・矛盾修正
  -> daemonの責務/interval/lock/停止/再起動契約
  -> daemon実装とUsageStoreのsingle-writer/重複抑制
  -> migration設計（別DB検証→atomic switch）
  -> Windows/native機能差分を埋める
  -> Linux/X・Windows・daemonの実プロセス/実画面/DB証拠
  -> 独立評価で全ID再判定
```

この監査表に `missing`、`partial`、`unverified` が残る限り、要求完了とは扱わない。

## 独立再監査追記（履歴：2026-08-22、requirements_audit担当）

台帳統合前の独立評価は **FAIL** だった。Rust（152+164+13+36）、Windows Core 28、
Presentation 30、`windows_client_contract_gate.sh`、`record_daemon_e2e.sh`、保存済み
fresh画像（Graph/Threads/Legal/Setup）のSHA照合・目視はPASSだった。Setupの認証済みstatus snapshot必須化と
操作部品のAutomationPropertiesも確認した。

当時は`data_protection_gate.sh`がDP-009=partialのためfail-closedで終了した。実Windowsの
高DPI・Graph/Threads/Legal全状態・ブラウザ認証/WSL再接続、systemd実登録・異常終了復旧、
native全状態のfresh PID/XID証拠、R18の100% coverage/mutation証拠は環境外または未取得である。
Start Menu install/start/uninstallは、後段のinstaller追補監査でホストWindows実機確認済みである。
この時点では残る条件を未確認のままプロジェクト全体完了へ変更してはならなかった。

### installer追補監査（最新変更）

Build scriptはclean checkoutでもclient／installer双方を`--locked-mode`でrestoreしてから
publishするよう修正した。Windows runner smokeはshortcutのtargetだけでなくworking directory、
Start-menu経由の実プロセス起動、停止後のuninstall、ショートカット・registry・install directory
除去まで確認する定義へ拡張した。ホストWindowsで実インストール・Start-menu起動・アンインストール・設定保持を再実行し、WIN-INSTALL-01〜04はverifiedへ更新した。この追補時点ではWindows parity全体とDP-009の統合判定が未反映だった。

## 旧版の独立再監査記録（2026-08-22、現行受入根拠から撤回）

上記のFAIL記録は台帳更新前の履歴として保持する。この節のPASS記載は旧成果物・旧スクリプトに対する履歴であり、現行リリース成果物の合格根拠ではない。現行判定は `docs/INDEPENDENT_AUDIT_LATEST.md` の `status: HOLD` を唯一の最新判定とする。

```text
current artifact installer SHA: b5cccbb2b949b6c57a8641b0beb96374315b5004e97bd0c3fe67ed16237b563d
bash scripts/completion_guard.sh       -> FAIL (independent audit HOLD; fail-closed)
bash scripts/regression_guard.sh       -> FAIL (independent audit HOLD; fail-closed)
physical move smoke (default)          -> SKIP (no cursor/focus manipulation)
```

`AUD-023` の旧実機移動結果は、ユーザーのカーソルを操作する旧スクリプトで取得したため現行受入から撤回する。現行の物理入力なしスモークは未検証をPASSへ変換せずSKIPとする。既存のDB保護・daemon証跡は履歴として保持するが、現行Windowsグラフ同値性と独立監査が終わるまで納品判定はHOLDとする。

顧客固有の資格情報、権限、容量、監視通知先など製品外条件は、成功を捏造せず、[顧客運用ランブック](CUSTOMER_OPERATIONS_RUNBOOK.md)の事前チェックと停止条件として固定する。これは現行製品の受入未達ではなく、納品先での安全な導入境界である。

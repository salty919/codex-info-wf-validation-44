# 要求管理台帳

この台帳にない振る舞いは、実装・評価・PRの対象にしない。要求を追加・変更した場合は、先にここへ安定ID、境界、失敗動作、独立オラクルを登録する。`verified`以外の行を残したまま最終ゲートを通過させてはならない。

## Windows版をX版品質へ揃える（2026-08-27）

- 目的: Windows版のデータ意味論、状態遷移、失敗時保持、Graph、Threads、起動表示、操作品質を、X版の検証済み正本と同等にする。
- 非目的: X版のレイアウトを無条件に複製すること、未取得値の推測、実Windows未確認項目のPASS化、PR・Releaseを要求完成前に開始すること。
- 不変条件: 同一fixtureの値・期間・欠測・残量・thread意味論はplatform間で一致する。部分世代、stale completion、失敗応答はlast-good rootを壊さない。Windows固有コードは表示・入力adapterに限定し、第二のデータ正本を作らない。
- 依存DAG: 共通wire/fixture正本 → Windows validation/state generation → Main/Graph/Threads projection → Window/Input/Accessibility adapter → 実Windows連結評価 → 最終判定。
- raw clause対応: 「Windows側の品質をX版にまで」→ WIN-PARITY-DATA/STATE/UX/OPS、「計画→分析→実装設計→TDD→連結評価→結果→判断→終了」→ PROC-WIN-PHASE-01、「Luna-maxをユニット単位、SOLは計画・設計・最終評価」→ PROC-WIN-DELEGATE-01、「`.com/{UUID}_req.md`/`_res.md`」→ PROC-WIN-HANDOFF-01、「2分周期、待機中は消費しない」→ PROC-WIN-MONITOR-01、「未完なら再計画して反復」→ PROC-WIN-CLOSURE-01。

| ID | 要求（観測可能な契約） | 境界・失敗動作 | 実装範囲 | 独立オラクル | 状態 |
| --- | --- | --- | --- | --- | --- |
| WIN-PARITY-DATA | Windows版はX版と同じquota、model usage、履歴期間、欠測、未使用帯、残量独立性、thread live意味論を同一fixtureから表示する | 重複時刻、初回未観測、長時間gap、reset境界、全model 0/複数、thread 0/複数/不正を推測で補わない | `src/*`共通wire/fixture、`windows-client/src/CodexInfo.WindowsClient.Core/*`、`windows-client/src/CodexInfo.WindowsClient/Graphing/*`、`windows-client/src/CodexInfo.WindowsClient/ViewModels/*` | X/Windowsが別helperではなく同じ固定fixture期待値を通るunit、REST fixture、実Windows Graph/Threads画像 | open |
| WIN-PARITY-STATE | Windows版は初回完全世代まで固定スピナーを表示し、定期更新・再接続・部分失敗・stale completionで画面を空にせず、完全なlast-good rootを保持する | health/status/details不一致、timeout、cancel、順序逆転、threadだけ失敗、初回失敗、再試行を有限state tableで固定 | `windows-client/src/CodexInfo.WindowsClient/ViewModels/*`、`Infrastructure/*`、`MainWindow.axaml` | state-machine unit、API integration、実Windows startup/refresh/failure UIAと動画または連続capture | open |
| WIN-PARITY-UX | Windows版のMain/Setup/Settings/Graph/Threads/Legalは正本geometry、DPI、配置、drag、keyboard、focus、accessibility、single-version表示を満たす | 900×480 fixed群、Graph 700×480以上、複数monitor、text scale、high contrast、child singleton、見えない配置をPASSにしない | `windows-client/src/CodexInfo.WindowsClient/*.axaml*`、window/input/accessibility adapter、Presentation tests、E2E | UIA geometry/pixel/keyboard checks、全登録surfaceのfresh Windows capture、独立意味棚卸し | open |
| WIN-PARITY-OPS | Windows版は接続、Setup/Settings、SSH/WSL起動、再接続、更新通知、install/update/uninstallで既存設定・履歴・last-goodを破壊しない | 初回/再起動/保存失敗/取消/接続不能/更新失敗/再入/重複操作を有限transactionとして扱う | `windows-client/src/*`、installer/update tools、Windows tests/E2E | process argv test、settings persistence、installer/update transaction tests、実Windows導入/復旧証拠 | open |
| PROC-WIN-PHASE-01 | 作業は計画→分析→実装設計→TDD実装→連結評価→結果報告→判断→終了の順に進める | 要求・設計・対象unitが未確定の状態で実装または最終ゲートを開始しない | `.com/*_req.md`、`.com/*_res.md`、`docs/REQUIREMENTS_LEDGER.md` | phaseごとの成果物と後続開始条件の照合 | open |
| PROC-WIN-DELEGATE-01 | SOL親が計画・設計・統合・最終評価を所有し、Luna-max子が狭い非重複unitの分析・TDD実装・連結評価を行う | 子は指示外編集・他差分の巻戻し・未実行PASSを行わない | `.com/*_req.md`、`.com/*_res.md` | req ownershipと実diff、res command evidenceの対応 | open |
| PROC-WIN-HANDOFF-01 | 各Luna unitは`.com/{UUID}_req.md`を入力正本とし、完了時に対応する`{UUID}_res.md`へ結果を書く | res欠落、UUID不一致、指示外成果は回収済みにしない | `.com/*_req.md`、`.com/*_res.md` | UUID一対一・必須section検査 | open |
| PROC-WIN-MONITOR-01 | SOL親はLuna作業中、2分間隔で対応resの有無だけを確認し、待機中にrepo・ログ・差分を読まない | 完了前の短周期poll、重複検査、同一入力の再実行をしない | `.com/*_res.md`、agent status | 2分wait履歴と一度だけの回収 | open |
| PROC-WIN-CLOSURE-01 | 全res回収後にSOLが条項別証拠を判断し、未完なら原因単位で再計画して新UUIDへ反復する | FAIL/INCONCLUSIVE、実Windows未確認、変更後の古い画像を終了扱いしない | `docs/REQUIREMENTS_LEDGER.md`、`.com/*_res.md` | 全行verified、fresh独立評価、同一revision実Windows証拠 | open |

## 起動CLI再設計（2026-08-27）

- 目的: 利用者向け起動契約を、無印、`--ui`、`--port PORT`、`--stop`、`--help`だけへ限定する。
- 非目的: LAN公開、任意bind address、内部収集テスト用CLI、旧オプション互換を提供しない。
- 不変条件: 待受addressは常に`127.0.0.1`、同一profileのdaemon ownerは1つ、停止対象はlock identityで検証したPIDだけ、未知・重複・欠落・範囲外引数は副作用なしで拒否する。
- 依存DAG: CLI parse → loopback config → service/UI lifecycle → Windows起動引数・systemd/E2E → 多言語help。
- raw clause対応: 「無印も引数ありもloopback」→ PROC-LAUNCH-01、「常駐解除」→ PROC-STOP-01、「不要な`--service`/`--ui-only`/`--all`/`--record-daemon`」→ PROC-OPTIONS-01、「各オプションの動作テスト」→ PROC-OPTIONS-01/PROC-STOP-01。

### 有限受理・拒否表

| argv | 終了/動作 | 副作用契約 |
| --- | --- | --- |
| なし | daemon+REST、127.0.0.1:8787 | Windowなし、owner最大1 |
| `--port P`、P=1..65535 | daemon+REST、127.0.0.1:P | Windowなし、非loopback bindなし |
| `--ui` | daemon+REST確保後にX UI | healthy owner再利用、無ければ1つだけ開始 |
| `--ui --port P`、P=1..65535 | 127.0.0.1:Pを使う上記UI | argv順序はこの形だけ |
| `--stop` | 下記停止契約 | daemon/REST/UIを新規生成しない |
| `--help` / `--h` / `-h` | locale別help、終了0 | daemon/REST/UI/DB変更なし |
| 旧6 option、未知語、誤記、重複、欠落、余分な位置引数、逆順、`--port=P`、help/stopとの混在、P=0/65536/負数/非数値/Unicode不正 | 終了非0 | daemon/REST/UI/DB/lock変更なし |

### `--stop`停止契約

- profileのlock pathが存在しなければ停止済みとして終了0。
- lock pathが存在するが、完全なlock identity・PID starttime・executable identityを検証できない場合は、何もsignalせず終了非0。
- 検証済み所有PIDにTERMを1回だけ送り、最大5秒間、同じlock pathが消えることを待つ。SIGKILLへ昇格しない。
- signal失敗、5秒timeout、停止中に別ownerが同じlockを取得、lockが残る場合は終了非0。
- DB、backup、reset hint、source JSONLの内容と存在を変更しない。listenerだけが存在しlock ownerを証明できない場合は無関係processとしてsignalしない。

| ID | 要求（観測可能な契約） | 境界・失敗動作 | 実装範囲 | 独立オラクル | 状態 |
| --- | --- | --- | --- | --- | --- |
| X-START-01 | X版の初回認証済み遷移では、ヘッダーと製品バージョンを固定し、quota/local usage/threadの部分世代を段階表示しない | 未認証画面は認証操作を表示し、認証済みだが未完成の間は内容領域を公開しない | `ui/app.slint`、`src/main.rs`、`src/main.rs::sync_ui`、`scripts/x11_startup_visual_gate.sh` | native startup state test、X11 fresh startup visual gate | verified |
| X-START-02 | X版は最初の完全な利用世代が揃うまで内容領域にスピナーを表示する | quota取得済みでもlocal usage未完了なら継続。完全世代公開後に解除 | `ui/app.slint` の `startup-loading`、`scripts/x11_startup_visual_gate.sh` | `native_startup_loading_requires_a_complete_authenticated_generation`、X11 startup visual gate | verified |
| X-START-03 | X版の初回local収集/app-server失敗時はスピナーを解除し、最後の完全表示または失敗状態を表示する | 部分値・0・未取得へ黙って置換しない | `src/main.rs::native_startup_loading`、status state | `native_startup_failure_releases_loading_surface` | verified |
| X-START-04 | X版の起動ウィンドウは主モニターの可視デスクトップ内へ配置し、利用者から見えない位置へ出さない | 別モニター・負座標・デスクトップ外の場合は合格にしない | `src/main.rs`、`run.sh`、`scripts/x11_startup_visual_gate.sh` | visible position test、X11可視範囲ゲート、実画面キャプチャ | verified |
| X-START-05 | `--ui` のdaemon/REST起動失敗時もX版GUIを表示し、接続失敗と再試行手段を提示する | サービス失敗でプロセス・ウィンドウを即時終了させない | `src/main.rs`、`run.sh` | 失敗ポートでの実起動保持確認、失敗状態テスト | implemented |
| X-GRAPH-01 | X版の過去グラフは期間境界・累積モデル値・未使用帯・残量を正本どおり表示する | 未観測長時間を斜めの使用量へ変換しない。残量をモデル使用量から推測しない | `src/main.rs`、`src/main.rs` graph projection | fixed graph fixture tests、X11 graph visual gate | verified |
| X-THREAD-01 | X版thread取得が不完全でも、完全な前回表示を部分値で上書きしない | malformed token_count envelopeだけは非livenessデータとして読み飛ばし、lifecycle/model不正は全体失敗 | `src/thread_contract.rs`、`src/main.rs` | parser/failure-class exact tests | verified |
| WIN-START-01 | Windows版の初回起動はhealth・status・detailsの一致した完全世代まで内容領域を隠し、固定位置のスピナーを表示する | 部分世代の順番描画・画面全体のばたつきを許可せず、失敗時は再試行を表示 | `windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml`、`windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs`、`windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/MainWindowViewModelTests.cs` | Windows presentation tests、実Windows UI Automation/E2E | implemented |
| WIN-GRAPH-01 | Windows版の過去グラフはX版と同じ期間境界・累積値・未使用帯・残量独立性を表示する | 未観測区間を使用量へ推測せず、専用ニュートラル色を失わない | `windows-client/src/CodexInfo.WindowsClient/Graphing`、`windows-client/tools/Run-WindowsClientE2E.ps1` | Windows graph projection tests、実Windows過去期間画像 | implemented |
| WIN-VERSION-01 | Windows版の製品バージョンはメイン画面に一度だけ表示し、子ウインドウのタイトルへ重複表示しない | X版と同じversion authorityを使用し、表示欠落・重複を許可しない | `windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml`、`windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs` | Windows contract gate、実Windows UI Automation | implemented |
| PROC-LAUNCH-01 | 起動モードを固定し、引数なし/`--port PORT`はloopback daemon+RESTのみ、`--ui`はdaemon+REST+X UI、`--ui --port PORT`は指定portで同じ動作をする | portは1..65535、addressは127.0.0.1固定。モード不一致、非loopback bind、二重daemon所有を合格にしない | `src/main.rs`、`run.sh`、`packaging/codex-info.service`、`README.md`、`README.en.md`、`DESIGN.md`、`SECURITY.md`、`scripts/record_daemon_e2e.sh`、`scripts/cli_contract_e2e.sh`、`docs/CUSTOMER_OPERATIONS_RUNBOOK.md`、`docs/REST_API_V1.md`、`docs/DATA_PROTECTION_POLICY.md`、`docs/WINDOWS_CLIENT.md` | 全受理形・port境界unit test、モード別実行、REST health | verified |
| PROC-STOP-01 | `--stop`は同一profileのlock identityで検証した常駐daemonだけへTERMを送り、lock解放まで待つ。停止済みなら成功する | 無関係PIDを停止しない。timeout、signal失敗は非0。DB/historyを削除しない | `Cargo.toml`、`Cargo.lock`、`src/main.rs`、`src/daemon.rs`、`scripts/cli_contract_e2e.sh`、`docs/CUSTOMER_OPERATIONS_RUNBOOK.md` | isolated HOMEでstart→health→stop→lock/listener消失、停止済み冪等test | verified |
| PROC-OPTIONS-01 | 公開引数は無印、`--ui`、`--port PORT`、`--ui --port PORT`、`--stop`、`--help`/`--h`/`-h`だけとする | `--service`、`--ui-only`、`--all`、`--listen`、`--record-daemon`、`--once`、誤記、重複、欠落を副作用なしで拒否する | `src/main.rs`、`src/i18n.rs`、`run.sh`、`scripts/record_daemon_e2e.sh`、`scripts/cli_contract_e2e.sh`、`docs/*`、`windows-client/src/*`、`windows-client/tests/*` | finite accept/reject matrix、旧option repository scan、helpとparser集合一致 | verified |
| PROC-HELP-01 | `--help`/`--h`/`-h`は起動せず、既存のi18nカタログで利用可能な起動モードと動作を表示する | ヘルプ要求でdaemon・REST・GUIを生成しない、固定単一言語の製品メッセージを出さない | `src/i18n.rs`、`src/main.rs`、`run.sh`、`docs/CUSTOMER_OPERATIONS_RUNBOOK.md` | 全対応言語のcatalog test、help mode unit test、`LC_ALL`を切り替えた`run.sh --help`実行ログ | verified |
| PROC-I18N-01 | 起動スクリプトを含む利用者向け固定メッセージは、本体と同じ対応localeのi18nカタログから導出する | 単一言語の製品文言をスクリプトへ複製しない | `src/i18n.rs`、`src/main.rs`、`run.sh`、`docs/PRODUCT_REQUIREMENTS.md` | 全対応言語の`launch_help`テスト、script固定文言検査、`LC_ALL`切替実行 | verified |
| PROC-LEDGER-01 | 要求台帳にない要求を実装・評価・PRに含めない。台帳の全行を最終ゲートで検証する | 台帳行追加後は旧証拠を無効化し、全行を再確認する | `run.sh`、`.github/workflows/rust.yml`、`.github/workflows/windows-client.yml`、`docs/PRODUCT_REQUIREMENTS.md`、`docs/REGRESSION_PREVENTION_POLICY.md`、`scripts/regression_guard.sh`、`scripts/windows_client_contract_gate.sh`、`scripts/x11_graph_visual_gate.sh`、`scripts/requirements_ledger_gate.sh`、`docs/REQUIREMENTS_LEDGER.md` | ledger gate、同一revisionの全ゲート | verified |

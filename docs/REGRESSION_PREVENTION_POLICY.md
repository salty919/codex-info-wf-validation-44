# 回帰防止規約（必須）

一度受入した機能を次の修正で失うことは、単なるテスト漏れではなく納品禁止の回帰である。本規約は今回限りではなく、以後すべての変更に適用する。

## 固定契約

| 契約ID | 固定する振る舞い | 回帰オラクル |
| --- | --- | --- |
| REG-WIN-DRAG | 全ボーダーレス画面は上端タイトル領域の明示的な左ドラッグで移動でき、ボタン操作を奪わない | `WindowDragBehavior.Attach` / `BeginMoveDrag`、Presentationテスト、対象release artifactの実機証拠（物理入力は明示許可時のみ） |
| REG-GRAPH-END | 現行グラフの終端は`min(reset_at, now)`で、開始アンカーから右端まで使用する | `EffectiveGraphEnd`テスト、グラフfresh画像、X版差分監査 |
| REG-GRAPH-SERIES | 累積最大値、初回未観測区間の水平保持、flat/rising線幅・opacity、未使用期間の専用ニュートラル帯（X/Windows共通の`#3F5D7C`・opacity `0.22`）を維持する。モデル使用後に遅れて届いた低い残量観測は反映し、残量観測が無い区間を料金から推測しない。通常sample内の両側実測で挟まれた欠測だけは正本規則で補間し、source cursorから回収不能と確定したdaemon gapは補間・旧値複製・推測をしない。 | Graph fixture、遅延残量観測回帰テスト、gap種別付きsource/DB/API契約検査、色定数・opacityテスト、独立X/Windowsグラフ監査 |
| REG-MAIN-REFRESH | 定期的なquota再取得は不完全な中間イベントであり、前回コミット済みのモデル・履歴・thread表示を空に戻さない。認証主体の変更/明示的なログアウトだけが可視状態をクリアし、rolling reset_atの揺れは同一期間として扱う。 | `periodic_quota_refresh_retains_last_good_main_snapshot`、quota→local commitの状態遷移テスト、X版fresh画面、Windows API parity証跡 |
| REG-SETUP-ONCE | 接続確認済みならSetupを再表示せず、6-key設定へ再接続に必要な非秘密profile selectorだけを保存する。raw host/user、OpenSSH展開値、password/token/key/path/API URL/commandは保存しない | 6-key atomic保存・old4 recovery・次回自動再接続・`ShouldOpenSetup`テスト、設定JSON secret scan |
| REG-NO-MOUSE-STEAL | 製品コードはカーソルを操作しない。物理入力試験はテストプロセスからだけ明示opt-inで実行し、製品実行へ混入させない | 製品ソースAPIスキャン、smokeの既定SKIP raw出力、CI受入時の`-AllowPhysicalInput`実行ログ |

## 強制ゲート

1. 変更前に`docs/PRODUCT_REQUIREMENTS.md`と対象仕様を読み、変更する観測結果と失敗時動作を決める。
2. 変更後に固定.NET SDKのCore/Presentationテスト、契約ゲート、`git diff --check`、native回帰ゲートを実行する。
3. インストーラを再発行した場合は、artifactとworkspace publish copyのSHAが一致し、ホストのインストール先SHAも一致することを確認する。
4. 独立サブエージェントが、実装者のPASS結論を見ずに上表を再評価する。1項目でもFAIL/INCONCLUSIVEなら`RELEASE HOLD`とする。
5. `docs/INDEPENDENT_AUDIT_LATEST.md`を`status: PASS`へ変更できるのは独立評価担当だけとし、主担当が手動でPASSへ書き換えてはならない。

6. `scripts/regression_guard.sh`は静的な文字列検査だけでPASSしてはならない。履歴・グラフの必須Rust回帰テスト（複数期間の境界、通常のmoving-reset、累積ドリフトする長時間moving-reset、使用量0の残量100% reset断片が前後の使用量期間を分割しないこと、観測されていない長時間を累積使用量の斜め線として描かないこと、モデル使用後の遅延した低残量観測を捨てないこと）を`--exact`で実際に実行し、対象テストが0件、未実行、失敗の場合は必ずFAILにする。さらにworking tree/index/current commitのdiff、format、全target check、全target test（実行件数>0）、release buildを同じゲートで検査する。履歴・グラフの変更は、この実行結果なしに完了判定してはならない。これはデグレード防止だけでなく、実行していない検証をPASS扱いする評価漏れの防止を目的とする。

7. PRのmerge前およびRelease jobは`bash scripts/final_acceptance_gate.sh <Windows UI E2E evidence>`を通過しなければならない。同ゲートはRustのformat/check/test/release build、必須回帰テスト、Windows UI AutomationのPASSログ、quota gauge証拠、過去期間グラフでLUNA/TERRA/SOLのモデル線と未使用期間の専用帯が既知の区間に実描画された証拠、E2Eが定義する19枚の名前付き画面キャプチャ（各PNG署名・非空）を検証する。さらに同じWindows実行で`windows_window_move_smoke.ps1 -AllowPhysicalInput`を実行し、source SHA付き`window-move-smoke: PASS`を必須とする。E2Eログの実行元SHAは対象SHAと一致し、各画像のSHA-256は同じ実行のcapture行と一致しなければならない。E2E出力は実行開始時に消去し、追記・前回証跡の再利用を許可しない。証拠ディレクトリ、ログ、対象マーカー、SHA、画面のいずれかが欠ける場合は`HOLD`とし、未確認をPASSへ変換しない。

8. Windows workflowのpath filterはnative collector/API、Slint UI、Cargo manifest/lockfile、build/run script、全scripts/docsを含める。これらの変更でWindows/native回帰ゲートが起動しない状態を許可しない。契約ゲートはこのtrigger一覧とRust toolchain、native gateの必須コマンド自体も検査する。

9. CI checkoutは履歴比較を必要とする全jobで`fetch-depth: 0`を使用する。全targetテストは実行件数0を許可せず、Windowsテストはpassed>0かつskipped=0を満たさなければならない。件数を出せない、または一部targetが未実行の結果はPASSではなくFAIL/HOLDとする。

## 回帰発生時

回帰を検出した時点で、前回のPASS証拠を現行成果物の証拠として再利用しない。影響する観測結果だけを再検証し、無関係な検査や文書hash更新を連鎖させない。過去DB・履歴を削除して見かけ上直すことは禁止する。

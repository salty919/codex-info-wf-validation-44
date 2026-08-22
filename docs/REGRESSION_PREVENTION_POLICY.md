# 回帰防止規約（必須）

一度受入した機能を次の修正で失うことは、単なるテスト漏れではなく納品禁止の回帰である。本規約は今回限りではなく、以後すべての変更に適用する。

## 固定契約

| 契約ID | 固定する振る舞い | 回帰オラクル |
| --- | --- | --- |
| REG-WIN-DRAG | 全ボーダーレス画面は上端タイトル領域の明示的な左ドラッグで移動でき、ボタン操作を奪わない | `WindowDragBehavior.Attach` / `BeginMoveDrag`、Presentationテスト、同一SHAの実機証拠（物理入力は明示許可時のみ） |
| REG-GRAPH-END | 現行グラフの終端は`min(reset_at, now)`で、開始アンカーから右端まで使用する | `EffectiveGraphEnd`テスト、グラフfresh画像、X版差分監査 |
| REG-GRAPH-SERIES | 累積最大値、初回未観測区間の水平保持、flat/rising線幅・opacityを維持する。通常sample内の両側実測で挟まれた欠測だけは正本規則で補間し、source cursorから回収不能と確定したdaemon gapは補間・旧値複製・推測をしない | Graph fixture、gap種別付きsource/DB/API契約検査、独立X/Windowsグラフ監査 |
| REG-SETUP-ONCE | 接続確認済みならSetupを再表示せず、6-key設定へ再接続に必要な非秘密profile selectorだけを保存する。raw host/user、OpenSSH展開値、password/token/key/path/API URL/commandは保存しない | 6-key atomic保存・old4 recovery・次回自動再接続・`ShouldOpenSetup`テスト、設定JSON secret scan |
| REG-NO-MOUSE-STEAL | 製品コードはカーソルを操作しない。物理入力試験は明示opt-in以外で実行しない | 製品ソースAPIスキャン、smokeの既定SKIP raw出力 |

## 強制ゲート

1. 変更前に `scripts/requirements_intake_guard.sh` の要求抽出ゲートを通し、要求ID・担当サブエージェント・所有ファイル・受入オラクルを`docs/AGENT_REQUIREMENTS_TRACKER.md`へ登録する。抽出ゲートFAIL中の実装開始は禁止する。
2. 変更後に固定.NET SDKのCore/Presentationテスト、契約ゲート、`git diff --check`、native回帰ゲートを実行する。
3. インストーラを再発行した場合は、artifactとworkspace publish copyのSHAが一致し、ホストのインストール先SHAも一致することを確認する。
4. 独立サブエージェントが、実装者のPASS結論を見ずに上表を再評価する。1項目でもFAIL/INCONCLUSIVEなら`RELEASE HOLD`とする。
5. `docs/INDEPENDENT_AUDIT_LATEST.md`を`status: PASS`へ変更できるのは独立評価担当だけとし、主担当が手動でPASSへ書き換えてはならない。

## 回帰発生時

回帰を検出した時点で、前回のPASS証拠を「現行成果物の証拠」として再利用してはならない。影響を受ける全行を`unverified`へ戻し、同一SHAで再ビルド・再検証する。過去DB・履歴を削除して見かけ上直すことは禁止する。

# Codex Info（Rust / X Window / WSLg）

Codex App ServerからChatGPT/Codexアカウントのレート制限と週次または月間のリセット時刻を取得し、WSLのX Windowに表示します。WebサーバーやTkinterは使いません。UIはRustの宣言的GUIツールキットSlintで構成しています。

## 起動

```bash
git clone https://github.com/salty919/codex_info_v2.git
cd codex_info_v2
./run.sh
```

別の場所へコピーして使う場合は、履歴の保存先を明示できます。

```bash
CODEX_INFO_DATA_DIR="$PWD/data" ./run.sh
```

初回起動時のnative Window titleは「アカウント未接続 — プラン未設定」です。アプリ内では認証パネルが接続状態を案内します。

1. 「認証を開始」を押す
2. 「認証ページを開く」を押す
3. ブラウザでOpenAIアカウントにログインし、Codexへのアクセスを許可する
4. X Windowに戻り、「認証状態を確認」を押す

このアプリにユーザー名、パスワード、APIキーを入力する必要はありません。認証処理とトークン保存はCodex CLIに任せます。認証済みなら起動時に自動接続します。

## 必要環境

- Rust/Cargo
- WSLgまたはXサーバー（`DISPLAY`が設定されていること）
- `codex` CLI（`codex app-server --stdio`が実行できること）

日本語表示用のNoto Sans JPフォントを`assets/`に同梱しているため、ホスト側の日本語フォント追加インストールは不要です。

初回起動時にCargoが依存クレートを取得・ビルドします。画面が出ない場合は次を確認してください。

```bash
echo "$DISPLAY"
codex login status
codex app-server --help
```

## 取得する情報

アプリはCodex App ServerへJSON-RPC接続し、次のAPIを利用します。

- `account/read` — 認証状態とアカウント情報
- `account/login/start` — 未認証時のChatGPTログイン開始
- `account/rateLimits/read` — 使用率とリセット時刻
- `thread/list` — 最新候補から実行中のスレッド全件とモデルを取得（native sub-agentは検証済みrollout/状態DBの親子関係を補完）

取得したトークンやパスワードはアプリのファイルへ保存しません。Codex側の認証ストアが管理します。

## 表示と更新

- 残り時間は日・時・分で読みやすく表示
- レート制限は1分ごとに再取得
- 認証済みのnative Window titleはMainが`<email> — <localized plan>`、Threadsが`<email> — <localized plan> — Threads`、Graphが`<email> — <localized plan> — Graph`です。用途の日本語は各Window内の見出しで表示します。未認証・初期・logout時は全Windowで「アカウント未接続 — プラン未設定」となります。HeaderとAccountActivityにはemail/planを重複表示しません。
- 認証済み画面の「グラフ」ボタンから1つのグラフウインドウを開きます。残り利用枠・LUNA/TERRA/SOLを個別にON/OFFでき、初期状態は全系列ONです。ドル表示は各モデルの入力・キャッシュ・出力合計を独立した平滑化ラインで描き、表示中モデルの個別最大値へ縦軸を合わせます。リセット直後（0）から現在時刻まで表示します
- グラフの1分サンプルはSQLite（`history/usage_history.sqlite3`）へ保存し、JSONは後方互換の読み込み・移行元として扱います。同一リセット期間・同一分は最大値を保持して再計測で減少しません。通常起動で削除されるのは3か月より古い行だけです。グラフ上部の「ドル／トークン」で、ドルは累積額、トークンは各モデルの時間帯別使用量へ切り替えられます（初期値はドル）。
- `CODEX_INFO_DATA_DIR`を指定すると、そのディレクトリ配下へ履歴を保存します。既存の`CODEX_HOME/history`（または`CODEX_INFO_MIGRATE_FROM`）からの移行は追加・冪等で、移行元は変更・削除しません
- 週次または月間の対象期間の残り時間は、端数も含めた7セルのゲージで表示
- `~/.codex/sessions`に履歴がある場合は、週次または月間の対象期間を表示し、その期間内のSOL/TERRA/LUNAの入力（非キャッシュ）・キャッシュ入力・出力トークン数と、[OpenAI Developer Docsのモデル料金表](https://developers.openai.com/api/docs/models)に基づく予想ドル額（整数部のみ）を各カテゴリの独立したドル列に表示します。見出しはモデル・入力・キャッシュ・出力だけです。クレジット換算は行いません。連続する同一累積スナップショットは差分0として二重計上しません。その他のモデルは表示しません。
- 履歴は直近3カ月を保持し、グラフの期間履歴listから過去のリセット期間を選択できます。最新の実行中スレッドは全件を表示します。canonical active snapshotの取得順は`updatedAt desc, id desc`のままですが、Threads画面だけが親を先に置くdepth-first・subtree-contiguous順へ投影し、role/depth/orphanをtree guideで示します。
- プランはschema検証済みアカウント情報からnative Window titleへ表示します。Enterpriseの`individualLimit`は月間枠として扱い、`unlimited`は固定上限なしとして表示します。認証状態や固定月間上限を、レスポンスにない情報から推測しません。
- リセット前後24時間は状態バナーで明示
- 認証失敗・Codex未起動などは画面にエラー表示

## Windowサイズとプレビュー

Main/Threads画面はclient 900×480px固定（初期・最小・最大900×480）で、本文を左右30pxに揃えます。Slintのmin/max hints、winitの`Resized` mismatch event guard、X11 state monitorで固定し、mismatch時のevent-triggered `request_inner_size`は許可します。timer/Slint `set_size`修復、hide/recreate/retryは行いません。移動・最小化・復元・閉じるは通常動作します。Graph画面はclient 940×640px（初期）、最小700×480pxで、最大幅・最大高さを設けず拡大・最大化・全画面化を許可します。状態別の確認には`CODEX_INFO_PREVIEW=auth|normal|warning|reset-warning|error|zero|full|monthly|unlimited|idle`を使い、グラフ表示は`CODEX_INFO_PREVIEW=graph|graph-old`で確認できます。`CODEX_INFO_PREVIEW_SIZE`はMain/Threadsでは900×480px、Graphでは幅・高さそれぞれ700×480pxを下限として適用し、上限は設けません。メイン画面の指定例は`CODEX_INFO_PREVIEW=normal ./run.sh`です。

## UIを調整する場所

画面の余白・色・文字サイズ・角丸は`ui/theme.slint`に集約しています。ヘッダー、利用枠、期間ゲージ、状態バナー、認証パネルは`ui/components.slint`の再利用部品です。レイアウトを変更するときはRustコードを座標調整せず、これらのトークンまたは部品のプロパティを変更してください。

## ライセンス

別途明記された第三者素材を除き、このリポジトリの独自コードと文書は[GNU General Public License version 3](LICENSE)のみに基づいて提供されます（SPDX: `GPL-3.0-only`）。

同梱フォント、Codex CLIから生成したプロトコルスキーマ、Slint、およびRust依存クレートには、それぞれのライセンスが引き続き適用されます。詳細は[第三者ライセンス通知](THIRD_PARTY_NOTICES.md)を参照してください。

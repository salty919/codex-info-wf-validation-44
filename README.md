<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Codex Info（Rust / X Window / WSLg）

English quick start: [README.en.md](README.en.md) · 多言語化仕様: [docs/LOCALIZATION.md](docs/LOCALIZATION.md)

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

初回起動時の画面内タイトルは`Codex Info`です。ネイティブタイトルバーは使わず、アプリ内では認証パネルが接続状態を案内します。

1. 「認証を開始」を押す
2. 「認証ページを開く」を押す
3. ブラウザでOpenAIアカウントにログインし、Codexへのアクセスを許可する
4. X Windowに戻り、「認証状態を確認」を押す

このアプリにユーザー名、パスワード、APIキーを入力する必要はありません。認証処理とトークン保存はCodex CLIに任せます。認証済みなら起動時に自動接続します。

## 必要環境

- Rust/Cargo
- WSLgまたはXサーバー（`DISPLAY`が設定されていること）
- `codex` CLI（`codex app-server --stdio`が実行できること）

日本語・韓国語表示用フォントは`assets/`に同梱してSlintへ埋め込み、ホスト側フォントに依存しません。著作権とライセンスは[第三者ライセンス通知](THIRD_PARTY_NOTICES.md)と[assets/NOTICE.txt](assets/NOTICE.txt)に記載します。

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

### 言語・時刻・日本語表示

- 固定UI文言は起動時に`LC_ALL` → `LC_MESSAGES` → `LANG`の順で選び、日本語、英語、中国語（簡体）、韓国語、スペイン語、フランス語、ドイツ語、ポルトガル語、イタリア語、ロシア語を表示します。`C`、`POSIX`、未対応localeは英語です。
- localeとIANA timezoneはプロセス起動時に確定します。絶対時刻・履歴期間・グラフ横軸はそのtimezone、経過時間・残り時間はUTC差分で表示します。`TZ`にはIANA IDを指定でき、無効値はUTCとして扱います。
- 固定文言のcatalog、locale境界、時刻表示の仕様は[多言語化仕様](docs/LOCALIZATION.md)にまとめています。

- 残り時間は日・時・分で読みやすく表示
- レート制限は1分ごとに再取得
- ネイティブタイトルバーは使わず、各Windowの画面内タイトル領域に埋め込みフォントの見出しと自前の移動・最小化・閉じる操作を配置します。固定Windowには最大化操作を表示しません。
- 認証済み画面の「グラフ」ボタンから1つのグラフウインドウを開きます。残り利用枠・LUNA/TERRA/SOLは凡例で個別に表示／非表示を切り替え、非表示系列は色とラベルのコントラストを下げて示します。初期状態は全系列表示です。各モデルの入力・キャッシュ・出力合計を独立した累積ラインで描き、表示中モデルの個別最大値をドル軸へ使います。全モデルの累積値が変化しない未使用区間はプロット下地の薄い帯で示し、右端の値は系列色のリーダー線で終端へ結びます。リセット直後（0）から現在時刻まで表示します
- グラフの1分サンプルはSQLite（`history/usage_history.sqlite3`）へ保存し、JSONは後方互換の読み込み・移行元として扱います。同一リセット期間・同一分は最大値を保持して再計測で減少しません。通常起動で削除されるのは3か月より古い行だけです。グラフ上部の「ドル／トークン」で、ドルは累積額、トークンは各モデルの時間帯別使用量へ切り替えられます（初期値はドル）。
- `CODEX_INFO_DATA_DIR`を指定すると、そのディレクトリ配下へ履歴を保存します。既存の`CODEX_HOME/history`（または`CODEX_INFO_MIGRATE_FROM`）からの移行は追加・冪等で、移行元は変更・削除しません
- 週次または月間の対象期間の残り時間は、端数も含めた7セルのゲージで表示
- `~/.codex/sessions`に履歴がある場合は、週次または月間の対象期間を表示し、その期間内のSOL/TERRA/LUNAの入力（非キャッシュ）・キャッシュ入力・出力トークン数と、[OpenAI Developer Docsのモデル料金表](https://developers.openai.com/api/docs/models)に基づく予想ドル額（整数部のみ）を各カテゴリの独立したドル列に表示します。見出しはモデル・入力・キャッシュ・出力だけです。クレジット換算は行いません。連続する同一累積スナップショットは差分0として二重計上しません。その他のモデルは表示しません。
- 履歴は直近3カ月を保持し、グラフの期間履歴listから過去のリセット期間を選択できます。最新の実行中スレッドは全件を表示し、累積tokenと`model_context_window`から算出したコンテキスト使用率と上限を`使用率% / 上限トークン`で併記します。Threads画面は親を先に置くdepth-first・subtree-contiguous順へ投影し、role/depth/orphanをtree guideで示します。
- プランはschema検証済みアカウント情報から判定します。Enterpriseの`individualLimit`は月間枠として扱い、`unlimited`は固定上限なしとして表示します。認証状態や固定月間上限を、レスポンスにない情報から推測しません。
- リセット前後24時間は状態バナーで明示
- 認証失敗・Codex未起動などは画面にエラー表示
- 画面右上の「法的通知」から、GPLの無保証条項と第三者素材のライセンスを確認できます。ソース配布物の`LICENSE`、`THIRD_PARTY_NOTICES.md`、`LICENSES/`も併せて保持してください。

## Windowサイズとプレビュー

ネイティブタイトルバーは全Windowで無効にし、ボタン以外の画面領域をドラッグして移動できます。Main/Threads/Legalはclient 900×480pxまたは720×520pxで固定し、最大化・リサイズ操作を表示しません。Graphはclient 940×640pxを初期値、700×480pxを最小値、上限なしとし、四隅の広い対角領域または辺からリサイズできます。状態別の確認には`CODEX_INFO_PREVIEW=auth|normal|warning|reset-warning|error|zero|full|monthly|unlimited|idle|legal`を使い、グラフ表示は`CODEX_INFO_PREVIEW=graph|graph-old`で確認できます。`CODEX_INFO_PREVIEW_SIZE`はGraphの初期サイズを上書きするレイアウト検証用です。メイン画面の指定例は`CODEX_INFO_PREVIEW=normal ./run.sh`です。

## UIを調整する場所

画面の余白・色・文字サイズ・角丸は`ui/theme.slint`に集約しています。ヘッダー、利用枠、期間ゲージ、状態バナー、認証パネルは`ui/components.slint`の再利用部品です。レイアウトを変更するときはRustコードを座標調整せず、これらのトークンまたは部品のプロパティを変更してください。

## ライセンス

Copyright (C) 2026 salty919

別途明記された第三者素材を除き、このリポジトリの独自コードと文書は[GNU General Public License version 3](LICENSE)（SPDX: `GPL-3.0-only`）で提供します。英語の[LICENSE](LICENSE)がGPLv3の正文であり、正式な条件・定義・免責を定めます。[LICENSE.ja.md](LICENSE.ja.md)は日本語案内です。著作権一覧は[COPYRIGHT](COPYRIGHT)を参照してください。

同梱フォント、Codex CLIから生成したプロトコルスキーマ、Slint、およびRust依存クレートは各上流ライセンスで提供します。ソース・バイナリ配布物には[第三者ライセンス通知](THIRD_PARTY_NOTICES.md)、[assets/NOTICE.txt](assets/NOTICE.txt)、[LICENSES/](LICENSES/)を同梱します。

# Codex Info Wiki

このWikiは **Codex Info**（Rust + Slint + WSLg）向けの運用・開発情報を1か所に集約したページです。  
本体の最新情報は本リポジトリの[README.md](../README.md)を参照してください。

## 目次

- [導入と起動ガイド](./導入と起動ガイド.md)
- [開発・運用メモ](./開発・運用メモ.md)
- [GitHub Wiki 反映ガイド](./GitHub-Wiki-反映ガイド.md)

## 1分でわかる概要

- Codex App ServerとJSON-RPCで接続し、認証状態・レート制限・リセット時刻・利用履歴を表示
- UIは**Slint**で構築（Web UI / Tkinterは未使用）
- 日本語を含む複数言語に対応（locale自動判定）
- 認証は`codex` CLIに委譲し、APIキーやパスワードはアプリ保存しない

## 表示で見る主な画面情報

- メイン画面
  - 認証状態（未認証・認証済み・エラー）
  - レートリミット残量（ゲージ）
  - リセットまでの残り時間
  - モデル別の使用トークン集計（USD見積）
  - 法的通知へのリンク
- スレッド画面
  - 実行中スレッドと`usage% / 上限`表示
  - スレッド階層
- グラフ画面
  - 残量/モデル別系列を分離表示（表示ON/OFF可能）
  - 1分サンプルをSQLiteへ蓄積

## 主要な動作条件

- Rust/Cargo（`$HOME/.cargo/bin/cargo`が取得可能）
- WSLg または X11 が使える表示環境（`DISPLAY`）
- `codex app-server --stdio` が起動できること

## 免責とライセンス

- 本リポジトリのソース・ドキュメントはGPL-3.0-only
- 法的通知、第三者素材はそれぞれ`LICENSE` / `LICENSE.ja.md` / `THIRD_PARTY_NOTICES.md` / `assets/NOTICE.txt` / `LICENSES/` を参照

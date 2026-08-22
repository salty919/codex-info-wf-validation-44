# B2B納品受入基準

この文書は、Codex Info Monitorを顧客環境へ引き渡す前の出荷ゲートである。実行できない試験、静的確認だけの試験、過去の画像の再利用はPASSとして扱わない。

## 出荷停止条件

- `scripts/completion_guard.sh` または `scripts/data_protection_gate.sh` が非0
- 要求台帳に `open`、`partial`、`unverified`、`inconclusive` が残る
- 最新ビルドと証拠のハッシュが一致しない
- 実Windowsでインストール、Startメニュー起動、更新、アンインストールを確認できない
- native/Windowsの各状態（認証、通常、警告、エラー、0/中間/100%、Graph、Threads、Legal、最小幅、高DPI）をfreshプロセスで確認できない
- DBの行数、SHA-256、SQLite `quick_check`、バックアップ世代、migration失敗時の原本保持を確認できない
- OSS notices、ライセンス、バージョン、再配布物の一覧が納品物に含まれない

## 必須納品証拠

| 分類 | 必須証拠 |
| --- | --- |
| 要求 | 要求ID→実装→テスト→実機証拠のトレーサビリティ表 |
| ビルド | lockfile固定restore、warning/error、成果物SHA-256、対象RID |
| UI | 状態・サイズ・DPI別のfresh画像、目視チェックリスト、操作記録 |
| Windows | Setup、Start Menu、Apps登録、更新rollback、uninstall、設定保持 |
| Linux/daemon | user systemd登録、health、SIGKILL復旧、singleton、CPU境界 |
| DB | backup世代、quick_check、row/hash不変性、migration成功/失敗 |
| セキュリティ | endpoint/認証/ログ/秘密情報/権限境界のテスト結果 |
| OSS | `THIRD_PARTY_NOTICES.md`、`LICENSES/`、同梱ファイルの照合 |
| 運用 | 導入、更新、rollback、復旧、アンインストール、問い合わせ先 |

## 判定者

実装担当者とは別の評価担当が、最新証拠を直接確認して判定する。実装者の「完了」宣言は受入証拠の代替にならない。

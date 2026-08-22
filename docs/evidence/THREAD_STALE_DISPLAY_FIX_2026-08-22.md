# Threads stale display fix evidence (2026-08-22)

対象: `src/main.rs`

修正:

- `ActiveThreadUpdate::Failed` 時に `active_threads` をクリアする。
- `apply_thread_error` 時にも stale rows をクリアする。
- ネイティブ子スレッドは、DBに残る履歴だけでは実行中とみなさず、現在のCodexプロセスが保持するrollout pathに限定する。
- quota・認証・履歴は保持し、`thread_error` は維持する。
- ステータスを「前回値を保持」から取得失敗・実行中状態未確認へ変更する。
- 履歴とスレッドが同時に失敗する場合も、実行中状態が未確認であることを明示する。

固定検証:

```text
cargo fmt --check: PASS
cargo test: PASS（全375テスト、失敗0。失敗cycleから完全snapshotへ復帰し、production secure-open境界とEOF未完了全分類を通る回帰テストを含む）
cargo build --release --locked: PASS
```

追加・更新した確認:

- Failed後の active rows が0
- Snapshot / NoThread の既存動作
- stale epoch の no-op
- public snapshot / public details / Threads rows が0
- `./run.sh`（UI付き、`CODEX_INFO_DEBUG=1`）の実機ログ: `thread root snapshots=1`, `thread descendant snapshots=0`, `thread descendants skipped inactive=53`, `thread snapshot rows=1`。周期更新でも `rows=1`を確認。
- `native_live_state_matrix_is_fail_closed_across_path_and_rollout_states` が running/terminal/invalid/missing × active/inactive child の6ケースをテーブル駆動で検証。
- 現行release実行ファイル SHA-256 `816de5ddd00d8d12cc42b20fdac974b6a79d7da162e99d2c9d26a8824bb8726e`。同一SHAの通常 `./run.sh` UIによる実画面・状態別証拠が揃うまでUI受入はHOLDとする。

実Windows/X WindowのThreads画面キャプチャ、同一SHAの視覚キャプチャは未取得であり、
独立監査がPASSするまで要求行は`HOLD`とする。

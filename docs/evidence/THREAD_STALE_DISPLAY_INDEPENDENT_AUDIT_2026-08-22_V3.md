# Threads stale display 独立監査 V3 (2026-08-22)

## 判定

`INCONCLUSIVE/HOLD`。

静的コードと `cargo test --quiet` は今回の対象要件を通過している。しかし、
Failed / `apply_thread_error` と合成失敗状態を同一成果物の実 Windows/X Window
プロセスで表示した最新キャプチャがない。既存の Threads 画像は通常の行表示であり、
失敗時の 0 行・失敗文言・quota/history 保持を画面で確認する証拠にはならないため、
この監査では `PASS` にしない。

## 対象

- `src/main.rs` の `ActiveThreadUpdate::Failed`、`apply_thread_error`、thread/local の
  `auth_epoch` 境界、`refresh_partial_failure_status`
- 対象テスト: Failed 後の rows、quota/history 保持、stale epoch no-op
- 今回の評価ではソースコードを変更していない。作業ツリーにある対象外の変更は戻していない。

## 要件別結果

### 1. Failed / 取得エラーで rows=0

静的・単体テストは PASS 相当。

- `apply_active_thread_update` の `ActiveThreadUpdate::Failed` は
  `active_threads.clear()` を実行し、`true` を返す (`src/main.rs:5213-5225`)。
- `apply_thread_error` も fresh snapshot を得られなかった場合に
  `active_threads.clear()` を実行する (`src/main.rs:5449-5459`)。
- `apply_thread_result` は Failed の戻り値を `thread_error` に反映し、rows 数を
  debug counter に出す (`src/main.rs:5434-5447`)。
- `quota_projection_and_thread_state_transitions_are_atomic` は Failed 後の
  `active_threads`、public snapshot の `active_thread_count`、public details の
  `threads`、presentation rows がすべて空であることを確認する
  (`src/main.rs:8150-8158`)。

### 2. quota / plan / reset / history を保持

静的・単体テストは PASS 相当。thread failure 経路が変更するのは thread の
checking/error、poll 時刻、rows だけであり、quota、plan、reset、history を書き換えない。
`thread_failure_preserves_quota_plan_reset_and_history` は remaining、reset、plan、
history の保持と rows=0 を確認する (`src/main.rs:8358-8377`)。

### 3. stale epoch event は no-op

静的・単体テストは PASS 相当。

- `apply_thread_result` と `apply_thread_error` は、未認証または
  `auth_epoch != self.auth_epoch` の場合、状態変更前に return する
  (`src/main.rs:5434-5437`, `5449-5452`)。
- `stale_thread_and_local_results_are_complete_no_ops` は古い Snapshot、Failed、
  Error、local success/error を順に適用し、threads、history、model usage、cost、
  status、checking/error flags がすべて元の値のままであることを確認する
  (`src/main.rs:8400-8445`)。

### 4. 失敗文言と合成 failure status

静的確認は PASS 相当だが、合成分岐の専用実行テストは未確認。

- thread 単独 `(false, true)` は「スレッド情報の取得に失敗し、実行中の状態は未確認です。」
  を含む (`src/main.rs:5480-5484`)。
- local と thread の合成 `(true, true)` も「履歴とスレッド情報の取得に失敗し、
  実行中の状態は未確認です。」を含む (`src/main.rs:5469-5474`)。
- local 単独 `(true, false)` は「履歴は前回値を保持しています。」を表示し、thread の
  未確認状態を誤って主張しない (`src/main.rs:5476-5479`)。

上記の文字列は静的に確認できるが、今回の `cargo test --quiet` の既存テストには
`(true, true)` を直接作って status を assert するケースがない。そのため、合成文言の
実行時証拠は未取得として扱う。

## Raw evidence

実行コマンドと結果:

```text
cargo test --quiet

running 152 tests
test result: ok. 152 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 164 tests
test result: ok. 164 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 13 tests
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 36 tests
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

合計 366 tests、失敗 0。

目視した既存画像:

- `docs/evidence/visual-2026-08-22/native-runtime-2026-08-22/native-multi-thread-threads.png`
  — X Window の通常 Threads 行（8 threads）で、Failed 状態ではない。
- `docs/evidence/visual-2026-08-22/windows-threads-fresh-window-only.png`
  — Windows の通常の複数 Threads 行で、Failed 状態ではない。

既存の `THREAD_STALE_DISPLAY_FIX_2026-08-22.md` も、修正後の実 Windows/X Window
Threads 失敗画面キャプチャと同一成果物の実プロセス再確認が未取得と記録している。
今回確認した画像一覧には通常表示の画像はあるが、Failed/apply_thread_error の 0 行と
合成 failure status を示す最新画像はない。

## 未検証事項と最小の差し戻しタスク

1. 現在の成果物を実プロセスとして起動し、Failed / `apply_thread_error` を発生させた
   最新の X Window と Windows Threads 画面を取得する。
2. その画像で rows=0、quota/history の保持、失敗文言の「実行中の状態は未確認です。」を
   目視し、同一成果物に結び付ける。
3. 合成 `(true, true)` status を直接実行する単体テストを追加するか、同等の実行ログを
   保存する。追加後は新規 Luna セッションで再評価する。

実画像の不足が解消されるまで、本件の最終判定は `INCONCLUSIVE/HOLD` とする。

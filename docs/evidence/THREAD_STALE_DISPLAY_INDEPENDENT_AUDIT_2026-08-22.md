# Threads stale display 独立監査 (2026-08-22)

## 判定

`INCONCLUSIVE/HOLD`。実 Windows/X Window の最新画面キャプチャがなく、UI の受入条件を独立に完了判定できないため、`PASS` にはしない。なお、静的コードと raw テストには以下の確認結果がある。

## 対象と前提

- 対象: `src/main.rs` の `ActiveThreadUpdate::Failed`、`apply_thread_error`、thread event の epoch 判定、関連テスト。
- 対象証拠: `docs/evidence/THREAD_STALE_DISPLAY_FIX_2026-08-22.md`。
- 今回の監査ではソース、テスト、指定証拠以外の変更は評価・変更していない。作業ツリーには対象外の既存変更もある。

## 要件別の確認

### 1. Failed / 取得エラーで実行中行が 0 件になる

静的確認は `PASS` 相当。`apply_active_thread_update` の `ActiveThreadUpdate::Failed` 分岐は `self.active_threads.clear()` を実行して `true` を返す。`apply_thread_result` はその結果を `thread_error` に反映し、`apply_thread_error` も worker が fresh snapshot を作れない場合に `self.active_threads.clear()` を実行する。

関連テスト `quota_projection_and_thread_state_transitions_are_atomic` と `thread_failure_preserves_quota_plan_reset_and_history` は、失敗後の `active_threads`、public snapshot の `active_thread_count`、public details の threads が空であることを検証している。`Snapshot` / `NoThread` の既存遷移も同じテストで確認される。

### 2. quota / history を保持する

静的確認は `PASS` 相当。thread 失敗経路が変更するのは thread checking/error、poll 時刻、行だけで、quota、plan、reset、history を書き換えない。`thread_failure_preserves_quota_plan_reset_and_history` は `remaining_percent`、`reset_at`、`plan_label`、`history.samples` の保持を検証している。

### 3. stale epoch event が no-op になる

静的確認は `PASS` 相当。`apply_thread_result` と `apply_thread_error` は、未認証または `auth_epoch != self.auth_epoch` の場合、状態変更前に return する。`stale_thread_and_local_results_are_complete_no_ops` は stale な Snapshot、Failed、Error を順に適用し、threads、history、model usage、cost、status、checking/error flags が全て元のままであることを検証している。

### 4. status が未確認を明示し、旧値保持と誤解させない

thread のみの失敗 (`local_usage_error == false`, `thread_error == true`) は静的には適合している。文言は次のとおりで、旧値保持を示さず「実行中の状態は未確認です」を明示する。

```text
利用枠は更新しました。スレッド情報の取得に失敗し、実行中の状態は未確認です。
```

ただし、local と thread が同時に失敗する `(true, true)` 分岐は次の文言であり、`未確認` を明示していない。

```text
利用枠は更新しました。履歴とスレッド情報の取得に失敗しました。
```

要件を全ての取得エラー合成状態にも適用する場合、この分岐は要件 4 の静的な不足である。該当する組み合わせの status テストも確認できない。少なくとも独立監査の受入証拠としては未確定とする。

## Raw evidence

実行したコマンド:

```text
git status --short
git diff -- src/main.rs docs/evidence/THREAD_STALE_DISPLAY_FIX_2026-08-22.md
rg -n 'ActiveThreadUpdate|apply_thread_error|ThreadEvent::(Error|Update)|auth_epoch|未確認|取得できません|スレッド情報' src/main.rs docs/evidence/THREAD_STALE_DISPLAY_FIX_2026-08-22.md tests
sed -n -e '5110,5245p' -e '5420,5505p' -e '5535,5605p' -e '8130,8465p' src/main.rs
cargo test --quiet
```

`cargo test --quiet` の raw 結果は次のとおり。

```text
running 152 tests       ... test result: ok. 152 passed; 0 failed
running 164 tests       ... test result: ok. 164 passed; 0 failed
running 1 test          ... test result: ok. 1 passed; 0 failed
running 13 tests        ... test result: ok. 13 passed; 0 failed
running 36 tests        ... test result: ok. 36 passed; 0 failed
running 0 tests         ... test result: ok. 0 passed; 0 failed
```

合計 366 tests、失敗 0。これは静的・単体テストの証拠であり、実画面の証拠ではない。

## 未検証事項と最小の差し戻しタスク

`THREAD_STALE_DISPLAY_FIX_2026-08-22.md` 自身が「実 Windows/X Window の Threads 画面キャプチャ、同一 SHA の実プロセス再確認は未取得」と記録している。したがって、次の独立評価まで HOLD とする。

1. 修正後の同一成果物 SHA で実プロセスを起動し、Threads の Failed / 取得エラー状態を最新キャプチャで確認する。少なくとも行 0 件、quota/history 保持、status の視認性を画面上で確認する。
2. `CODEX_INFO_PREVIEW` 等で必要な状態を起動し、古いキャプチャを再利用せずに保存する。
3. 要件 4 を合成失敗にも適用するなら、`(true, true)` の status に「実行中の状態は未確認です」を含め、組み合わせテストを追加する。その後、新規 Luna セッションで再評価する。

この監査ではソースコードを変更していない。監査文書の追加だけを行った。

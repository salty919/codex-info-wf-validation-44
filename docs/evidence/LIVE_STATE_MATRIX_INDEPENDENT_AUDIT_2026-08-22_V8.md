# Live-state matrix independent audit V8 (2026-08-22)

## 判定

`INCONCLUSIVE/HOLD`

V7以前の結論は前提にせず、現行作業ツリーを新規に読み直した。今回のRust静的ゲートとテストは通過し、production path の secure-open、L0の候補・cycle原子性、tail/record isolation、RPC/epoch境界をコードとテストで確認できた。しかし、受入条件が要求する現行release成果物SHAに結び付いた実X/Windows最新画像、実app-server停止→再起動trace、複数server/collector traceを確認できない。したがって静的PASSを実環境受入PASSへ昇格できない。

## 対象と成果物

- 作業ツリー: `/home/salty/code/codex_info_v2`
- 対象コード: `src/main.rs`, `src/thread_contract.rs`, `src/thread_state.rs`, `src/security.rs`, `src/usage_store.rs`
- 対象文書: `DESIGN.md`, `docs/DATA_PROTECTION_POLICY.md`, `docs/LIVE_STATE_DECISION_MATRIX.md`, `docs/REQUIREMENTS_LEDGER.md`
- 監査中のソース変更: なし（本ファイルだけを追加）
- release executable: `target/release/codex_info`
- release SHA-256: `dda697c3c36810307dae425e102bcd93854e24f4258ac2f3ad91cf8fcbe93cda`
- executable size: `59,804,592` bytes

## 検証コマンド

| Gate | 結果 |
| --- | --- |
| `cargo fmt --check` | PASS (status 0) |
| `cargo check --locked` | PASS (status 0) |
| `cargo test --locked --quiet` | PASS (status 0) |
| `cargo build --release --locked` | PASS (status 0) |

`cargo test --locked --quiet` の raw 結果は `152 + 168 + 1 + 13 + 36 = 370 passed, 0 failed, 0 ignored` だった。

## 実装・テストの監査結果

### L0 live-state admission

`docs/LIVE_STATE_DECISION_MATRIX.md` の候補schema、sessions root配下のcanonical path、現cycleのactive path、rollout最後のrunning task、owner/root/edge graph、current cycle/epoch/RPCというAND条件と、`src/main.rs` の `complete_rollout_prefix_len`、`open_codex_session_paths`、候補読取、`src/thread_contract.rs` のcycle accumulatorを突合した。次の境界テストが現行ソースに存在し、370件の実行で通過した。

- `native_live_state_matrix_is_fail_closed_across_path_and_rollout_states`
- `thread_c_candidate_failure_rejects_the_complete_cycle`
- `thread_c_snapshot_rejects_partial_candidate_reads`
- `thread_c_all_current_cycle_failure_classes_return_no_partial_snapshot`
- `thread_c_no_thread_and_all_candidate_failure_are_distinct`
- `open_codex_session_paths_accepts_only_bounded_codex_fds_under_sessions_root`
- `active_thread_adapter_rejects_partial_rollout_fallback`

### tail、record isolation、candidate reject、recovery

production側では `complete_rollout_prefix_len` がEOF直前の未改行tailを次cycleへ保留し、`read_recoverable_session_line` が改行済みのoversize/invalid UTF-8/invalid JSONを単一recordとして隔離する構造を確認した。`thread_contract::parse_rollout_reader_recoverable` と次のテストも確認した。

- `thread_c_rollout_bytes_utf8_jsonl_and_size_boundaries`
- `recoverable_rollout_parser_keeps_running_state_around_large_tool_output`
- `oversized_tool_records_do_not_hide_following_usage_samples`
- `malformed_tool_records_do_not_hide_following_usage_samples`
- `thread_c_rollout_last_valid_task_event_controls_running`
- `recovery_backfill_is_one_shot_until_authenticated_quota_returns`
- `thread_failure_recovery_requires_a_new_complete_snapshot`

文書の契約も、改行済み単一recordの隔離と、EOF以外の部分行・I/O・差替え・資源上限のcandidate/file fail-closedを分けて記載しており、`DESIGN.md`、`DATA_PROTECTION_POLICY.md`、`LIVE_STATE_DECISION_MATRIX.md` 間の今回の対象節に矛盾は見つからなかった。

### DB、RPC、epoch

`UsageStore` のtransaction/upsert、backup/migration/corrupt-schema保護と、RPCのmismatch/timeout/error redaction、thread/local stale epochの完全no-opを対象コードとテストで確認した。代表的な実行済みテストは次のとおり。

- `rpc_request_enforces_mismatch_timeout_and_error_redaction`
- `stale_thread_and_local_results_are_complete_no_ops`
- `stale_local_result_from_old_period_is_a_no_op`
- `local_success_is_the_only_path_that_commits_usage_and_history`
- `thread_failure_preserves_quota_plan_reset_and_history`
- `quota_event_is_pure_and_account_read_branch_has_no_thread_or_local_calls`

これらは静的・fixture境界の証拠であり、下記の実プロセス証拠を代替しない。

## 受入を止める未検証事項

1. **現行release SHAに結び付いた実X画像がない。** `docs/evidence/LIVE_STATE_RUNTIME_2026-08-22.md` が記録する対象SHAは `124a41aa7e20a24bc1ec2adac2948f2b89be8d158adf89696f4a9c61162983d4` で、今回の `dda697c3...` と一致しない。同文書自身も900x480 captureのviewport切れとUI受入HOLDを記録している。今回のSHAでの未認証・通常・警告・error・zero/full・最小幅のfresh X captureと目視証拠は確認できない。
2. **現行release SHAに結び付いた実Windows画像がない。** `WINDOWS_ACCEPTANCE_E2E_2026-08-22.md` は歴史的証跡/current HOLDと明記し、`WINDOWS_RUNTIME_2026-08-22.md` のWindows画像SHAは今回のLinux release executable SHAへ結び付いていない。Windows実機の現行SHA fresh imageをPASS証拠にできない。
3. **app-server停止→再起動traceがない。** active_pathsの変化、古いsnapshot保持、次の完全snapshotのみの公開を同一現行成果物で確認する実プロセスtraceを見つけられない。
4. **複数server/collector同時実行traceがない。** admission分離、旧epochの現行snapshot上書き防止、DB writer競合の実運用traceを確認できない。fixture/unitテストは存在するが、要求された実traceではない。
5. `docs/REQUIREMENTS_LEDGER.md` の `DP-001`、`DP-009`、`LIVE-001` は現時点でも `HOLD` であり、台帳自身の終了条件（HOLD/未検証ゼロ）を満たしていない。

## 最小の戻しタスク

現行release SHA `dda697c3c36810307dae425e102bcd93854e24f4258ac2f3ad91cf8fcbe93cda` を固定し、次を取得して新規Lunaセッションで再評価する。

1. 新規PIDの実X起動による全要求状態・最小幅capture、SHA、目視チェック結果。
2. 実Windows hostのfresh process/image、installer identity、現行成果物SHAの紐付け。
3. app-server停止→再起動のPID/active-path/epoch/debug trace。
4. 複数server/collector同時実行のadmission・DB・stale event trace。
5. 上記証拠を要求台帳へ登録し、completion/data-protection/regression gateを同じ成果物で再実行する。

上記が揃うまで、本監査の判定は `INCONCLUSIVE/HOLD` とする。

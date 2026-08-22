# LIVE state matrix independent audit V4 (2026-08-22)

判定: **INCONCLUSIVE / HOLD**

## 対象と再現性

- 評価は実装担当の結論を引き継がず、最新版 HEAD のみを読み取って実施した。
- source HEAD: `a0bcfe2759ef9ab3df301eba0dbcefdd25c1cde5`
- release binary SHA-256: `124a41aa7e20a24bc1ec2adac2948f2b89be8d158adf89696f4a9c61162983d4`
- 対象: `src/main.rs`、`src/thread_contract.rs`、`src/thread_state.rs`、`docs/LIVE_STATE_DECISION_MATRIX.md`、`DESIGN.md`、`docs/DATA_PROTECTION_POLICY.md`、`docs/REQUIREMENTS_LEDGER.md`
- ソースコードは変更していない。このファイルだけを監査証拠として追加した。

## 静的実装監査

- `open_codex_session_paths`（`src/main.rs:1169`）は `/proc` の `codex` process（comm と exe basename）だけを対象にし、session root 配下へ canonical 化できる `.jsonl` fd を bounded set（process/fd/file 上限）として `active_paths` に収集する。空集合は `NoThread` になる。
- thread/list は `ThreadCycleAccumulator` に cycle 内で蓄積し、pagination/cursor、最大 page/item、envelope、重複 ID の整合性を検証する。terminal page まで到達しなければ公開しない。
- root は canonical rollout path が `active_paths` に含まれること、rollout の完全行 prefix を読み直せること、最後の task state が running であることを同一 cycle で検証する。path 不在と terminal は候補を公開せず、active candidate の read/parse failure は `CycleError` となり部分 snapshot を公開しない。
- native child は state DB の owner root から bounded parent/child graph と rollout path を read-only で取得し、DB schema/row/path/edge の検証、cycle/depth/件数上限を通過したものだけを候補にする。child も `active_paths` と running rollout を再検証し、非 active/terminal は除外、read/parse failure は cycle 全体を `Failed` にする。
- 収集結果は ID でまとめて updated-at 降順に並べ、空なら `NoThread`、それ以外は `Snapshot`。RPC failure、invalid envelope、pagination/cursor failure、DB graph failure は `Failed` として旧完全 snapshot 保持へ渡る。
- thread worker は lazy start、initialize、Stop 時の kill/reap、failure 後の当該 server kill/reap と次回 read での再起動を実装している。同一 callback 内の自動 respawn は行わない。
- `apply_thread_result` / `apply_thread_error` は未認証または epoch 不一致を no-op とし、logout/auth clear は epoch を進め thread bridge と表示 thread を消去する。

## テスト網羅性（ソース上の確認）

次のクラスは unit/test fixture 上で確認できた。

- active fd の bounded/canonical filtering
- root の active/path-missing/terminal/invalid と empty/mixed
- child の active running、inactive path、terminal、invalid/partial
- DB の missing row、duplicate/conflicting row、孤立/dangling、cycle
- pagination cursor cycle/budget、duplicate ID、RPC/read failure、partial publish 防止
- stale thread/local/account epoch の no-op
- worker Stop/kill-and-reap の静的契約確認

scoped anchor の生出力:

src/thread_state.rs:253:fn native_thread_title(
src/thread_state.rs:712:    fn native_descendant_title_uses_agent_task_name_when_database_title_is_empty() {
src/thread_contract.rs:1698:    fn thread_c_schema_manifest_matches_pinned_cli_0147() {
src/thread_contract.rs:1905:    fn thread_c_request_first_page_exact_literal() {
src/thread_contract.rs:1920:    fn thread_c_request_followup_cursor_boundaries_and_omissions() {
src/thread_contract.rs:1957:    fn thread_c_page_envelope_schema_matrix_is_atomic() {
src/thread_contract.rs:2106:    fn thread_c_thread_schema_required_and_type_matrix() {
src/thread_contract.rs:2160:    fn thread_c_session_source_all_schema_valid_forms() {
src/thread_contract.rs:2294:    fn thread_c_session_source_invalid_union_matrix_rejects_item() {
src/thread_contract.rs:2453:    fn thread_c_schema_valid_auxiliary_fields_are_ignored_and_invalid_rejected() {
src/thread_contract.rs:2656:    fn thread_c_candidate_semantic_id_updated_path_boundaries() {
src/thread_contract.rs:2796:    fn thread_c_candidate_order_updated_desc_then_id_desc() {
src/thread_contract.rs:2818:    fn thread_c_pagination_terminal_and_empty_page_matrix() {
src/thread_contract.rs:2847:    fn thread_c_cursor_cycle_and_32_page_budget_fail_closed() {
src/thread_contract.rs:2906:    fn thread_c_1024_unique_item_budget_exact_boundary() {
src/thread_contract.rs:2948:    fn thread_c_identical_duplicate_deduplicates_only_once() {
src/thread_contract.rs:2965:    fn thread_c_same_id_latest_and_equal_timestamp_conflict_rules() {
src/thread_contract.rs:3004:    fn thread_c_private_accumulator_abort_never_yields_partial_snapshot() {
src/thread_contract.rs:3049:    fn thread_c_rollout_last_valid_task_event_controls_running() {
src/thread_contract.rs:3098:    fn thread_c_model_scalar_and_label_complete_matrix() {
src/thread_contract.rs:3153:    fn thread_c_cumulative_total_token_literal_matrix() {
src/thread_contract.rs:3179:    fn thread_c_context_window_is_taken_from_the_latest_token_count() {
src/thread_contract.rs:3214:    fn thread_c_known_token_invalid_event_rejects_entire_rollout() {
src/thread_contract.rs:3243:    fn thread_c_initial_null_token_count_is_a_safe_noop() {
src/thread_contract.rs:3255:    fn thread_c_known_task_and_model_required_field_matrix_rejects_file() {
src/thread_contract.rs:3288:    fn thread_c_rollout_bytes_utf8_jsonl_and_size_boundaries() {
src/thread_contract.rs:3361:    fn thread_c_unknown_well_formed_events_do_not_change_snapshot() {
src/thread_contract.rs:3381:    fn thread_c_accepts_live_rollout_envelopes_with_top_level_type_and_payload() {
src/thread_contract.rs:3414:    fn thread_c_tracks_latest_user_instruction_timestamp_for_duration_display() {
src/thread_contract.rs:3447:    fn thread_c_candidate_failure_rejects_the_complete_cycle() {
src/thread_contract.rs:3474:    fn thread_c_snapshot_rejects_partial_candidate_reads() {
src/thread_contract.rs:3496:    fn thread_c_valid_running_without_token_keeps_thread_with_none() {
src/thread_contract.rs:3532:    fn thread_c_no_thread_and_all_candidate_failure_are_distinct() {
src/thread_contract.rs:3563:    fn thread_c_parent_and_child_are_all_published_with_relation_metadata() {
src/thread_contract.rs:3619:    fn thread_c_current_process_filter_excludes_stale_sessions_without_failure() {
src/thread_contract.rs:3649:    fn thread_c_all_current_cycle_failure_classes_return_no_partial_snapshot() {
src/thread_contract.rs:3696:    fn thread_c_title_name_preview_fixed_literal_matrix() {
src/main.rs:301:    fn native_label(self) -> &'static str {
src/main.rs:935:fn native_detail_window_title(
src/main.rs:954:fn native_account_window_title(account_title: &str) -> String {
src/main.rs:1169:fn open_codex_session_paths(
src/main.rs:1250:fn fetch_active_thread_update(
src/main.rs:1282:fn fetch_active_thread_update_for_paths(
src/main.rs:1299:fn fetch_active_thread_update_for_paths_and_state(
src/main.rs:8423:    fn stale_thread_and_local_results_are_complete_no_ops() {
src/main.rs:8470:    fn stale_local_result_from_old_period_is_a_no_op() {
src/main.rs:8740:    fn native_title_bars_are_ascii_safe_and_keep_move_context() {
src/main.rs:9089:    fn open_codex_session_paths_accepts_only_bounded_codex_fds_under_sessions_root() {
src/main.rs:9246:    fn multiple_running_threads_are_all_published_with_stable_order() {
src/main.rs:9442:    fn native_completed_rollout_is_excluded_from_published_snapshot() {
src/main.rs:9507:    fn native_stale_running_descendant_not_held_open_is_excluded() {
src/main.rs:9557:    fn native_live_state_matrix_is_fail_closed_across_path_and_rollout_states() {
src/main.rs:9700:    fn native_descendant_failure_rejects_root_snapshot_atomically() {
src/main.rs:10211:    fn native_window_contracts_keep_non_graph_windows_move_only() {

graph/DB anchor の生出力:

262:        .map_err(|_| ThreadStateError::InvalidRow)?
265:        security::bounded_thread_title(preview).map_err(|_| ThreadStateError::InvalidRow)?;
278:        .map_err(|_| ThreadStateError::InvalidRow)?
303:fn creates_cycle(parent: &str, child: &str, parents: &HashMap<String, String>) -> bool {
322:fn relation_depth(
331:            return Err(ThreadStateError::Cycle);
344:fn read_descendants(
447:                ) = row.map_err(|_| ThreadStateError::InvalidRow)?;
454:                    return Err(ThreadStateError::InvalidRow);
457:                    return Err(ThreadStateError::InvalidRow);
460:                    return Err(ThreadStateError::Cycle);
464:                        return Err(ThreadStateError::InvalidRow);
467:                let rollout_path = rollout_path.ok_or(ThreadStateError::InvalidRow)?;
483:                    preview.as_deref().ok_or(ThreadStateError::InvalidRow)?,
490:                    updated_at: updated_at.ok_or(ThreadStateError::InvalidRow)?,
503:                        return Err(ThreadStateError::InvalidRow);
539:pub fn load_native_descendants(
568:        .map_err(|_| ThreadStateError::Replaced)?;
569:    let after = fs::symlink_metadata(after_path).map_err(|_| ThreadStateError::Replaced)?;
571:        return Err(ThreadStateError::Replaced);
783:            Err(ThreadStateError::Cycle)
794:            Err(ThreadStateError::InvalidRow)
840:        assert_eq!(result, Err(ThreadStateError::InvalidRow));

## 文書との突合

- `docs/LIVE_STATE_DECISION_MATRIX.md` の AND 条件（schema valid、sessions root 内 canonical path、current `active_paths`、running terminal state、graph valid、current cycle/epoch/RPC）と実装の分岐は整合する。
- `DESIGN.md` の thread cycle atomic publish、DB履歴だけで live を再生しない契約、RPC/EOF/timeout の fail-closed、worker stop/reap、epoch no-op と整合する。
- `docs/DATA_PROTECTION_POLICY.md §2-11` は root/child とも同一 cycle の active path + running を要求し、判定不能時の旧完全 snapshot 保持を規定しており、実装の `Failed` 経路と整合する。
- `docs/REQUIREMENTS_LEDGER.md` の LIVE-001 は現在も `HOLD`、終了前チェックにも独立証拠・実機検査待ちが残る。台帳の独立証拠リンクは V2 を指したままで、今回の V4 証拠への更新は未実施である。
- 複数 server について、実装の `active_paths` は `/proc` 全 codex process の path union であり、server identity ごとの admission を証明する仕組みや実機試験結果はこの範囲では確認できない。これは「複数 server の admission 分離」を静的に合格扱いしない理由である。

## 必須コマンド

実行したコマンドは次の通り。

```text
cargo fmt --check
cargo check --locked
cargo test --locked --quiet
cargo build --release --locked
```

raw status:

```text
CARGO_STATUS fmt=0 check=0 test=0 build=0
test result: ok. 152 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 166 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

この監査では静的コマンドの成功だけを実機証拠とは扱わない。

## 未検証事項と HOLD 理由

- 最新成果物 SHA に結び付いた実 X Window の新規画像（root/child/empty/mixed、error、terminal、状態別表示）がない。
- Windows の実機画像・導入・process/DB/daemon の受入証拠がない。
- app-server の停止→reap→再起動を実プロセスで観測した trace がない。
- 複数 app-server/client の同時実行と cross-server admission 非混入を実機で観測した trace がない。
- stale epoch/RPC failure は unit fixture で検証したが、実 server の timeout/EOF/restart と最新 SHA の runtime trace へ結び付いていない。

したがって、静的実装と unit coverage は広いものの、要求された独立実環境証拠が欠けており、LIVE-001 を PASS にはできない。最小の返却タスクは、最新 SHA 固定で stop/restart・複数 server・RPC fault・stale epoch の runtime trace と X/Windows の新規画像を取得し、V4 を台帳の独立証拠へリンクしてから再評価することである。

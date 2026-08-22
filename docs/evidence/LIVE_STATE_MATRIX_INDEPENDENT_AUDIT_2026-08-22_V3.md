# LIVE-001 独立監査 V3（2026-08-22）

## 結論

**INCONCLUSIVE/HOLD**。

静的ゲートと単体テストは通過したが、LIVE-001 の解除条件にある現行成果物の実 Windows/X 画面キャプチャ、および実プロセスの停止・再起動を含む同一成果物への証跡連結をこの監査では確認できない。`docs/LIVE_STATE_DECISION_MATRIX.md` 自身も、独立監査・実プロセス再起動・実画面キャプチャが終わるまで `LIVE-001` を `HOLD` としている。

## 読み取り範囲

以下だけを読み取った（実装者の結論は前提にしていない）。

- `src/main.rs`（ライブ path scan、thread cycle、native child gate、worker、関連テスト）
- `DESIGN.md` のデータ契約（L0、行 78--86 付近）
- `docs/DATA_PROTECTION_POLICY.md` §2--§11
- `docs/LIVE_STATE_DECISION_MATRIX.md`
- `docs/TEST_GAP_REGISTER_2026-08-22.md`

## 実行証拠

実行ディレクトリは `/home/salty/code/codex_info_v2`。

| コマンド | 結果 |
| --- | --- |
| `cargo fmt --check` | PASS（終了コード 0） |
| `cargo check --locked` | PASS（終了コード 0） |
| `cargo test --locked` | PASS（終了コード 0）。lib 152、main 166、DB runtime 1、security 13、usage_store 36、合計 368 tests passed |
| `cargo build --release --locked` | PASS（終了コード 0） |

実画像のキャプチャ、X Window の目視、Windows 実プロセス導入・再起動は実行していない。よって geometry、表示状態、実プロセス所有 path、再起動後の fresh snapshot は未検証である。

## 判定

### 1. root/child の path と rollout state

実装上、`fetch_active_thread_update_for_paths_and_state` は次を確認している。

- root candidate は sessions root 配下の canonical regular rollout かつ `active_paths` に含まれるものだけを選ぶ。
- native child は DB から補完した後にも `active_paths.contains(&descendant.rollout_path)` を要求する。path がなければ child を skip する。
- child rollout の parse error は `ActiveThreadUpdate::Failed`、terminal は公開せず、running だけを追加する。
- native descendant の DB load error は root だけの部分結果へ降格せず `Failed` とする。

新設 `native_live_state_matrix_is_fail_closed_across_path_and_rollout_states` の6ケースは、次の child 側分岐を固定している。

| ケース | child fixture | child path | 期待 |
| --- | --- | --- | --- |
| `running-active` | running | active | root + child |
| `running-inactive` | running | inactive | root only |
| `completed-active` | terminal | active | root only |
| `invalid-inactive` | invalid | inactive | root only（inactive なので parse しない） |
| `invalid-active` | invalid | active | `Failed` |
| `missing-row` | DB child row missing | active | `Failed` |

ただしこの matrix の fixture は全ケースで root rollout を running とし、root path も `active_paths` へ無条件に挿入している。従って、要求の「root/child × path 有無 × running/terminal/invalid/missing」を root 側について同一 integration matrix で網羅した証拠ではない。`native_completed_rollout_is_excluded_from_published_snapshot` と `native_descendant_failure_rejects_root_snapshot_atomically` は child terminal/invalid の確認であり、root-only terminal/invalid/path-not-held-open の直接証拠とは区別する。

### 2. DB 欠落・重複・cycle・dangling

`DESIGN.md` L0 と保護規約 §2.11 は、DB の `threads` / `thread_spawn_edges` を履歴 inventory とし、DBだけで liveness を証明しないこと、欠落・重複・孤立・cycle・dangling・判定矛盾を fail-closed にすることを要求している。実行結果では次の固定試験が PASS だった。

- `thread_state::tests::graph_cycle_dangling_row_depth_and_unsafe_path_fail_closed`
- `thread_state::tests::invalid_descendant_does_not_return_a_partial_snapshot`
- `thread_state::tests::descendants_follow_parent_child_grandchild_and_exclude_other_roots`
- main 側 `native_live_state_matrix...` の `missing-row`

これは DB graph loader の unit evidence としては有効だが、現行成果物の実 SQLite を複数 process で開いた実障害証拠ではない。特に duplicate/孤立/cycle/dangling の各入力が、実 server owner path と RPC envelope を同じ cycle で通る integration trace は未取得である。

### 3. process 停止・再起動、複数 server、RPC、stale epoch

次の既存契約テストは PASS だった。

- `thread_contract::tests::thread_c_current_process_filter_excludes_stale_sessions_without_failure`
- `tests::open_codex_session_paths_accepts_only_bounded_codex_fds_under_sessions_root`
- `tests::rpc_request_enforces_mismatch_timeout_and_error_redaction`
- `tests::stale_thread_and_local_results_are_complete_no_ops`
- `tests::clearing_or_changing_authentication_advances_epoch`
- `tests::multiple_running_threads_are_all_published_with_stable_order`
- `thread_contract::tests::thread_c_private_accumulator_abort_never_yields_partial_snapshot`

また、thread worker は失敗時に child app-server を kill/reap して次回 read 用 server を再生成し、結果へ auth epoch を付けていることを `src/main.rs` の worker 実装で確認した。

一方、実 app-server を停止→再起動し、複数 server/collector が同時に存在する状態で `active_paths` の変化、RPC failure、再接続後の新しい完全 snapshot を取得した fresh runtime trace はない。単体試験の存在だけでこの実環境条件を PASS にはしない。

### 4. 部分 snapshot 拒否と旧完全 snapshot 保持

コードと試験は、cycle 内の envelope/rollout/DB descendant の失敗を部分 snapshot として公開しない方向で整合している。

- `native_descendant_failure_rejects_root_snapshot_atomically` は child の invalid rollout で root だけを成功値にしないことを確認する。
- `thread_c_all_current_cycle_failure_classes_return_no_partial_snapshot` と `thread_c_private_accumulator_abort_never_yields_partial_snapshot` は cycle-private accumulator の破棄を確認する。
- `tests::thread_failure_preserves_quota_plan_reset_and_history`、`tests::quota_projection_and_thread_state_transitions_are_atomic` は state 適用側の保持契約を確認する。
- `apply_thread_result` / `apply_thread_error` は現行 auth epoch だけを受理し、thread failure を quota/history の部分更新へ混ぜない。

従って「部分結果を新しい成功値として公開しない」という unit/static evidence は PASS。ただし、旧完全 thread snapshot を保持した画面を現行 PID・現行 build で目視した証拠はなく、保持契約の実 UI gate は未検証である。

## 重要な所見

1. **HOLD（必須証跡欠落）**: 現行成果物の実 Windows/X 最新画像、実 process 停止・再起動、画像と build SHA の連結がない。プロジェクト規約により PASS へ昇格不可。
2. **要追加試験（root matrix の未網羅）**: 新しい6ケースは child state/path を振るが、root を常に active＋running に固定している。root path 不在、root terminal、root invalid（および root の open FD がない停止境界）を同じ fetch integration matrix に追加すべきである。
3. **証跡メタデータ不一致**: `docs/TEST_GAP_REGISTER_2026-08-22.md` の TG-THREAD-02 は「367 tests」と記載するが、同じ workspace で今回の `cargo test --locked` は 368 tests passed と報告した。現行 SHA と件数を台帳・evidence で再照合する必要がある。
4. **未検証 runtime**: DB graph の fail-closed、RPC/stale epoch の unit evidence はあるが、実 app-server 再起動・複数 server/collector・画面反映を一つの現行成果物で通した証拠はない。

## Luna 実装担当へ戻す最小タスク

- root fixture の `active_paths` と rollout state を可変にした table-driven integration test を追加し、root/child の path 有無・running/terminal/invalid/missing を明示する。
- duplicate/孤立/cycle/dangling、process stop/restart、複数 server、RPC failure、stale epoch を同じ判定表と evidence manifest に結び付ける。
- 現行 release SHA で実 process trace と X/Windows の fresh image を取り直し、旧完全 snapshot保持、empty と failure の区別、主画面の状態を独立目視する。
- 368 tests の実行件数へ台帳を更新し、全 evidence を同一 SHA へ再照合する。


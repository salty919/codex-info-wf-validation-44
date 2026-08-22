# Live state matrix 独立監査 V5（2026-08-22）

## 判定

`INCONCLUSIVE/HOLD`。今回の候補差分を独立に静的再検証し、要求された4コマンドとraw test総数は通過した。しかし、要求台帳と設計正本が要求する実X/Windowsの最新画像、実プロセス停止・再起動、複数serverのtraceがこの監査入力に存在せず、実環境のLIVE-001をPASSへ昇格できない。V4の監査結果・画像・traceは開かず、結論を流用していない。

## 監査範囲

- `src/main.rs`
- `src/thread_contract.rs`
- `src/thread_state.rs`
- `DESIGN.md`
- `docs/LIVE_STATE_DECISION_MATRIX.md`
- `docs/DATA_PROTECTION_POLICY.md`
- `docs/REQUIREMENTS_LEDGER.md`
- 本証拠ファイル

ソースコードは変更していない。

## 静的・テスト検証

| コマンド | 結果 |
| --- | --- |
| `cargo fmt --check` | PASS (exit 0) |
| `cargo check --locked` | PASS (exit 0) |
| `cargo test --locked --quiet` | PASS (exit 0) |
| `cargo build --release --locked` | PASS (exit 0) |

`cargo test --locked --quiet` のraw結果は、152 + 167 + 1 + 13 + 36 + 0 = **369 tests passed**, failed 0、ignored 0、measured 0。候補が追加した `thread_failure_recovery_requires_a_new_complete_snapshot` も含めて通過した。

確認できた主要な根拠は次のとおり。

- `src/thread_contract.rs:1512` の `select_active_threads_parsed_where` は、同一cycleのadmitted candidateの読取／rollout失敗を `ThreadCycleOutcome::CycleError` とし、他candidateだけの部分snapshotを返さない。
- `src/main.rs:1299` のRPC・root選択・native descendant選択は `CycleError`／DB graph・rollout失敗を `ActiveThreadUpdate::Failed` へ伝播する。
- `src/main.rs:5456` 付近のstate適用は、Failed cycle時に旧active thread行をクリアし、完全な後続 `Snapshot` または正常な `NoThread` でのみ復帰する。追加された recovery test はこの境界を確認している。
- root/childのactive path、terminal、invalid、missing-row、stale childを `native_live_state_matrix_is_fail_closed_across_path_and_rollout_states`、`native_descendant_failure_rejects_root_snapshot_atomically`、`native_stale_running_descendant_not_held_open_is_excluded` が固定している。
- `src/thread_state.rs` はquery-only/read-only DB、schema/row/path検証、dangling/cycle/depth/unsafe pathをfail-closedにしている。RPC timeout/error/redaction、stale auth epochのno-opもテストで確認した。
- `parse_rollout_reader_recoverable` と oversized/malformed recordのテストは、完全なtool recordの隔離と前後のrunning/model/token状態保持を確認している。

## 文書整合性

`docs/LIVE_STATE_DECISION_MATRIX.md` のAND条件、root/child/empty/mixed/DB/RPC/epoch表、失敗後は新しい完全cycleだけで復帰する条件は、上記の静的実装・テスト境界と概ね一致する。`docs/DATA_PROTECTION_POLICY.md` §2-11 も、DB履歴をlivenessの証明にせず、active path + rollout terminal stateを要求している。

ただし、PASS前に解消すべき文書上の不一致が1件ある。

- `DESIGN.md` のlive rollout記述は、一方で「同じdevice/inodeの書込途中の末尾行はcandidate失敗にせず次cycleへ保留」とし、同じ段落／L0周辺で「途中行はcandidate全体をreject」とも記述している。実装は `complete_rollout_prefix_len` により末尾の未完了行を次cycleへ保留する側の挙動である。どの「途中行」を指すか（末尾partial tailと途中truncation等）を正本上で分離してから受入判定すべきである。

また、`docs/REQUIREMENTS_LEDGER.md` の `LIVE-001`、`DP-001`、`DP-009` は現時点で `HOLD` のままであり、LIVE-001の独立証拠欄はV4ファイル名を指している。V5の独立証拠を登録する台帳更新と、以下の実環境証拠の追加が必要である。

## 未検証事項（PASSを妨げるもの）

- 最新成果物で起動した実X WindowおよびWindowsクライアントの最新画面キャプチャ（状態別・サイズ別を含む）。
- 実app-server停止→再起動で `active_paths` が変化し、新しい完全snapshotだけが公開されること。
- 複数server/collectorの同時実行traceと、旧epochが現行snapshotを上書きしない実trace。
- 要求台帳が要求する最新runtime traceと統合／データ保護gateの同一成果物への結び付け。

従って、静的ゲートと369件のテストはPASSだが、要求全体の最終判定は `INCONCLUSIVE/HOLD` とする。最小の次タスクは、実X/Windows画像、停止再起動trace、複数server traceを最新成果物へ結び付け、上記のDESIGN.md文言と要求台帳のV5証拠参照を更新したうえで、新規の独立Lunaセッションに再評価させることである。

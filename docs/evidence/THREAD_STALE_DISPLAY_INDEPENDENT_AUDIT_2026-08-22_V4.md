# Thread stale display — 独立再監査 V4

監査日: 2026-08-22 (Asia/Tokyo)  
担当: Luna 独立評価  
対象: `src/main.rs` の active-path descendant gate、stale `Failed` clear、複合 failure status、epoch fence、および対象回帰テスト  
判定: **INCONCLUSIVE / HOLD**

## 範囲と判定理由

親から指定された対象ファイル・対象差分だけを確認した。ソースコードは変更していない。Windows/X Window を起動した実画像キャプチャと、全状態（未認証・通常・警告・エラー）の目視レビューはこの独立監査では実施していないため、プロジェクトの UI 完了ゲートに従い PASS ではなく HOLD とする。静的・単体テスト上の今回対象機能には FAIL は見つからなかった。

## 要求ごとの確認

| 要求 | 結果 | 証拠 |
|---|---|---|
| DB に残る停止済み child を active snapshot に混入させない | PASS (静的/単体) | `fetch_active_thread_update_for_paths_and_state` は descendant の `rollout_path` が `active_paths` に含まれない場合に先に skip。`native_stale_running_descendant_not_held_open_is_excluded` は child rollout に terminal event がなく parser 単独なら running でも、active path 不在なら root 1 行だけを assert。 |
| current root のみ（root snapshots=1、descendant snapshots=0、rows=1）と矛盾しない | PASS (対象テスト) / 実プロセス未検証 | `CODEX_INFO_DEBUG=1 cargo test --quiet native_stale_running_descendant_not_held_open_is_excluded -- --nocapture` の raw output は `thread root snapshots=1`、`thread descendant snapshots=0`、`thread descendants skipped inactive=1`。テスト本体は `rows.len() == 1 && rows[0].id == "root"` を assert。unit test は worker 経路ではないため `thread snapshot rows=1` の worker debug 行そのものは出ない。 |
| `Failed` 時は rows=0 | PASS (単体) | `apply_active_thread_update(ActiveThreadUpdate::Failed)` と `apply_thread_error` が `active_threads.clear()`。`quota_projection_and_thread_state_transitions_are_atomic`、`thread_failure_preserves_quota_plan_reset_and_history` が空 rows/public details を確認。 |
| Failed 時に quota/history を保持 | PASS (単体) | 上記 `thread_failure_preserves_quota_plan_reset_and_history` が remaining/reset/plan/history を比較し、失敗 status を確認。 |
| stale epoch は no-op | PASS (単体) | `stale_thread_and_local_results_are_complete_no_ops` は stale thread snapshot/Failed/error と local success/error 後に rows/history/model/status/flags が不変であることを assert。account error 後の queued 結果も `account_error_fences_queued_thread_and_local_results_without_clearing_last_valid_values` で確認。 |
| local + thread の combined failure status | PASS (コード/単体) | `refresh_partial_failure_status` の `(true, true)` は「ローカル履歴とスレッド情報を安全に取得できませんでした。」および利用枠保持の status を設定。local/thread failure の各テストが individual path を確認し、`apply_thread_result`/`apply_thread_error` が status refresh を呼ぶ。 |

## 検証コマンド

全コマンドは `/home/salty/code/codex_info_v2` で実行し、終了コード 0。

```text
cargo fmt --check                         PASS
cargo check                               PASS
cargo test --quiet                        PASS
cargo build --release                     PASS
```

`cargo test --quiet` の raw 結果: 152 + 165 + 1 + 13 + 36 + 0 tests の各 suite が全て `ok`、合計 367 passed、0 failed、0 ignored。release build は `Finished release profile`。

対象 raw test:

```text
$ CODEX_INFO_DEBUG=1 cargo test --quiet native_stale_running_descendant_not_held_open_is_excluded -- --nocapture
running 1 test
[codex-info] thread active owner roots=1
[codex-info] thread root snapshots=1
[codex-info] thread descendant snapshots=0
[codex-info] thread descendants skipped inactive=1
.
test result: ok. 1 passed; 0 failed; 0 ignored
```

## 未検証事項 / HOLD 条件

- 新しい成果物を `run.sh` で実際の X Window に起動していない。
- 新しいプロセスに結び付く画面キャプチャを撮影・目視していない。
- したがって rows=1 の worker debug 行、実画面の rows=0 表示、quota/history の視覚保持、全状態のレイアウト・重複文言・ゲージ方向は未検証。
- 親の実環境 debug evidence に `thread snapshot rows=1` が含まれている場合、今回の root=1/descendant=0 と整合する。ただし本監査で得た unit raw output 自体には worker 行はない。

以上により、コード/単体ゲートは PASS 相当だが、UI 完了条件を満たす独立評価の最終判定は **INCONCLUSIVE / HOLD**。Windows/X capture を取得した新規 Luna セッションで再評価が必要。

## 追補（2026-08-22、要求抽出継続）

要求抽出の行固有11列契約は `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md` に追加された。
ただし証拠は取得しておらず、active snapshot、stale epoch、Failed rows=0、last-good保持、非スクロールの
viewport/overflow/clip/focus/DPI、ライフサイクルの再入・キャンセル・終了・再開はいずれも
`INCONCLUSIVE / HOLD` のままである。したがってこの追補は実装・UI PASSを意味しない。

## 追加監査: 226行要求台帳と traceability design（文書のみ）

この追加監査では、コード、テスト、ビルド、画面を確認していない。対象は次の2文書だけである。

- `docs/WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md`

判定: **INCONCLUSIVE / HOLD**。台帳自身も `EXTRACTION_INCOMPLETE`、設計も `IN_PROGRESS / HOLD` を宣言しており、文書だけから要求閉鎖を PASS とする根拠はない。

### 確認できた整合

- row register の `WIN-*` 行は 226 行で、タイトルの「226件」と件数は一致する。
- 全行の状態が `INCOMPLETE` であり、未抽出を `verified` に丸めていない。traceability design §6 の「226行すべての11列・独立突合まで `EXTRACTION_INCOMPLETE`」という閉鎖条件とは、この状態表示の方向性だけは整合する。

### 重要所見（重大度順）

1. **BLOCKER — 行固有 traceability が未実装。** traceability design §1 は各 `WIN-X-NNN` に 11 列（requirement_id の旧ID/分割履歴、actor_entry、precondition、observable、data_visual_contract、failure_persistence_contract、security_performance_contract、implementation_target、test_oracle、evidence、independent_reviewer）を要求する。一方 row register の実列は `ID / 要件本文参照 / owner / 実装面 / test oracle / evidence / 状態` の7列だけで、actor、前提、観測値、データ/視覚契約、失敗・永続化契約、security/performance、独立判定が行ごとに存在しない。`baseline row WIN-X-NNN`、カテゴリ既定の test/evidence だけでは設計 §1 の禁止する推測・汎用値を解消できない。

2. **HIGH — 複数責務の未分割が設計規則と矛盾。** traceability design §4 は一行に異なる責務が2つ以上残る場合に分割し、旧IDとの対応を記録することを要求する。しかし WIN-K-001..016 は `abnormal/DPI/multi-monitor/window lifecycle` を一つの実装面・一つの境界/並行性 matrix・一つの process geometry/log 証拠へ一括し、WIN-M-001..030 は `purpose/navigation/non-scroll/design decisions` を一括している。WIN-D-001..012 も `active snapshot projection/Legal` を同一 owner/oracle/evidence にまとめており、スレッド live-state と Legal notice の責務境界・分割履歴がない。行数を226に合わせるための無根拠統合を禁止する同じ §4 に対し、現行台帳は行固有の分解を示していない。

3. **HIGH — スレッドのライフサイクル契約が観測可能な要求へ落ちていない。** WIN-D の行には、停止済み/履歴 child の除外、active snapshot の owner/path 条件、root/descendant 行数、Failed 時 rows=0、stale epoch の no-op、quota/history の last-good 保持、再試行・停止・復旧境界を記録する列値がない。設計 §1 が要求する `observable` と `failure_persistence_contract` が空のため、`live-state + notice matrix` というラベルだけでは、スレッド表示とライフサイクル要求の漏れを判定できない。

4. **HIGH — non-scroll UX の受入条件が行固有でない。** WIN-M の全行が `UX decision matrix` と `all-view fresh images + keyboard log` を共有するだけで、view/viewport、固定ヘッダー、許容 overflow、scroll 禁止/例外、clip、focus/keyboard、DPI 境界、状態別の期待値を持たない。設計 §4 の UI/デザイン具体化（viewport、余白、整列、focus/hover/disabled、DPI、非スクロール条件）を満たす証拠列ではなく、全画面画像を一括指定しているため、非スクロールの漏れと重複を独立に検出できない。

5. **HIGH — lifecycle の開始・終了・再入場契約が不在。** WIN-K は「window lifecycle」を実装面に含めるが、行ごとの actor/entry、busy/re-entry、cancel、close、異常終了、singleton、再開、プロセス所有、DPI/monitor 遷移、許容 geometry が記録されていない。設計 §4 の `提供する`、`自動`、`確認する` の具体化規則に対する受入式・raw evidence の対応がない。

6. **MEDIUM — 表示所有権と重複防止の境界が曖昧。** WIN-D の Legal notices と WIN-M の UX design decisions は、文言・表示場所・目的/ナビゲーションの責務が交差し得るが、row register に `data_visual_contract` や行固有の表示所有者・重複禁止欄がない。設計 §1 は表示所有者を observable/data contract として要求するため、同じ事実の別カテゴリ重複を排除できない。

7. **MEDIUM — 依存関係がカテゴリ止まり。** design §3 は `WIN-I/J -> WIN-B/D -> ... WIN-K/M -> WIN-L` を示すが、台帳に predecessor、依存行 ID、依存先を HOLD に留める判定欄がない。カテゴリが未確定でも個別行を誤って閉じないための機械的 trace が作れない。

### 最小修正タスク（次の文書/要求抽出担当へ）

各 `WIN-X-NNN` に design §1 の11列を追加し、baseline 本文から actor/entry、precondition、observable、data/visual、failure/persistence、security/performance を行固有に転記する。特に WIN-D は live snapshot、child lifecycle、Legal notice を責務単位へ分割または旧ID付きで境界化し、WIN-K は abnormal input/DPI/monitor/window lifecycle、WIN-M は purpose/navigation/non-scroll/design decision を分離する。各行に exact oracle、否定条件、fresh raw/image/process evidence、artifact SHA、独立 reviewer、PASS/FAIL/INCONCLUSIVE 式を割り当て、依存行が HOLD の間は WIN-L まで verified にしない。

### PID 所有確認

親が示した PID `688374` を対象 PID に限定して確認した時点で `ps -p 688374` は空であり、当監査が起動したプロセスとは確認できなかった。所有不明のプロセスを停止していない。今回の監査コマンドで起動したプロセスは終了済み。

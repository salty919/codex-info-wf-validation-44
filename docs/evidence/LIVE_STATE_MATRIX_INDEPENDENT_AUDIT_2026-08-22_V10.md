# Live-state matrix independent audit V10 (2026-08-22)

## 判定

`FAIL`（local valid UTF-8 EOF rollbackの回帰証拠不足）

コード経路そのものはV9の指摘を修正しており、EOF未完了recordを3経路で拒否する。しかし、今回の受入条件が明示する「valid EOF未改行のlocal rollbackテスト」は、提示されたテスト内容では実証されていない。加えて、実X/Windows現行SHA画像、停止→再起動trace、複数server/collector traceは依然としてなく、実環境受入は `INCONCLUSIVE/HOLD` である。

## 最新成果物とゲート

- release executable: `target/release/codex_info`
- release SHA-256: `879b2fd23697516f055635ba2f60bef2b4913bf44c5aaf273d2bb60c632b7eb7`
- `cargo fmt --check`: PASS
- `cargo check --locked`: PASS
- `cargo test --locked --quiet`: PASS、`153 + 169 + 1 + 14 + 36 = 373 passed; 0 failed`
- `cargo build --release --locked`: PASS
- コード変更: なし（本報告ファイルのみ追加）

## EOF全分類の実装確認

### 改行済みrecord

`src/security.rs:552-559` は上限内の改行済みrecordを `terminated=true` として返す。改行済みoversizeは `LimitExceeded`、改行済みinvalid UTF-8は `Parse` となり、recoverable rollout/local collectorはこの2種類だけをskipする。改行済みmalformed JSONはreaderではなくJSON parserで失敗し、record/candidate契約どおり隔離またはfail-closedになる。

### EOF未完了record

- EOF valid UTF-8: `read_bounded_jsonl_record` は `(line,false)`、`read_bounded_jsonl_line` は `Unterminated`、`RpcLine::read` も `Unterminated`。
- EOF invalid UTF-8: `read_bounded_jsonl_record` が `Unterminated`。
- EOF oversize: `drain_until_newline` がEOFを返し、`Unterminated`。
- local collector: `read_recoverable_session_line` は record APIの `terminated` を必須検査し、falseなら `Unterminated` を返す。`collect_session_file` と timeline collectorは途中集計をrollbackする。
- strict/recoverable rollout: `Unterminated`をskipせず `RolloutError::UnterminatedLine` としてcycle/candidateを拒否する。

この実装構造は `DESIGN.md:117`、`docs/DATA_PROTECTION_POLICY.md:39,53`、LIVE_STATE決定表のlive tail/record isolation契約と整合する。

## 検証済みテストと不足

確認できたテストは次のとおり。

- `jsonl_reader_rejects_unterminated_invalid_and_oversized_records`
  - security APIでEOF invalid UTF-8、EOF oversize、EOF valid UTF-8のline/RPC拒否を確認。
- `thread_c_rollout_bytes_utf8_jsonl_and_size_boundaries`
  - strict rolloutのvalid EOF tailを `UnterminatedLine` として拒否。
- `recoverable_rollout_parser_keeps_running_state_around_large_tool_output`
  - 改行済みoversizeのrecord isolation。
- `oversized_tool_records_do_not_hide_following_usage_samples`
  - local collectorの改行済みoversize isolation。
- `malformed_tool_records_do_not_hide_following_usage_samples`
  - local collectorの改行済みmalformed JSON isolation。
- `unterminated_session_record_rolls_back_the_whole_local_input`
  - local collectorのEOF未完了入力rollback。

ただし最後のlocal rollback fixtureは `valid` な完全recordの後へ `b"\\n{\\xff"` を追加している。EOF未完了部分はinvalid UTF-8であり、valid UTF-8 JSON（例 `b"\\n{}"`）のEOF未改行recordではない。したがって、実装がvalid EOFを拒否することはコード読査で確認できても、要求されたlocal collector回帰テストの証拠にはならない。

また、local collectorについてEOF oversizeを含む全分類を直接確認するテストは見当たらない。security reader単体の分類テストだけでは、collectorの全体rollbackまで証明しない。

## 残る受入ブロッカー

1. **P1 — local valid UTF-8 EOF rollback test不足:** `unterminated_session_record_rolls_back_the_whole_local_input` の末尾をvalid UTF-8 JSON・未改行へ変更し、既存の有効値がrollbackされることをassertする必要がある。EOF oversize local rollbackも同じfixture群で追加する。
2. **P1 — 現行release SHAの実環境証拠不足:** release SHA `879b2fd...` に結び付く実X/Windows fresh imageがない。`LIVE_STATE_RUNTIME_2026-08-22.md` は別SHA `124a41aa...` とviewport切れ/HOLDを記録し、Windows受入文書もcurrent HOLD/historical扱いである。
3. **P1 — process trace不足:** app-server停止→再起動のactive path/epoch trace、および複数server/collector admission分離・stale event非上書きtraceがない。
4. `docs/REQUIREMENTS_LEDGER.md` の `DP-001`、`DP-009`、`LIVE-001` は依然 `HOLD`。ledgerの独立証拠欄もV8を参照している。

## 最小の戻しタスク

1. local rollback fixtureをvalid UTF-8 EOF tailにし、EOF oversizeも追加して `Unterminated` と全体rollbackを直接assertする。
2. `cargo fmt --check`、`cargo check --locked`、`cargo test --locked --quiet`、`cargo build --release --locked`を再実行し、raw件数とSHAを更新する。
3. 新SHAへ紐付けた実X/Windows画像、停止→再起動trace、複数server traceを取得し、新規Lunaセッションで再評価する。

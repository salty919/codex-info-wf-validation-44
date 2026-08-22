# Live-state matrix independent audit V9 (2026-08-22)

## 判定

`FAIL`（EOF未完了recordのproduction local/RPC境界）

実環境証拠については別途 `INCONCLUSIVE/HOLD` である。現行release SHAに結び付いた実X/Windows最新画像、app-server停止→再起動trace、複数server/collector traceを確認できないためである。

今回の `src/security.rs` 差分は、EOF未完了のinvalid UTF-8/oversizeを `Unterminated` として改行済みrecordのrecoverable skipから分離し、strict rollout parserが `UnterminatedLine` へfail-closedする点では正しい。しかし、valid UTF-8のEOF未改行recordが別経路で受理されるため、今回の境界要求全体は不合格とする。

## 対象成果物とゲート

- 対象: `src/security.rs`, `src/thread_contract.rs`, `src/main.rs`、`DESIGN.md`、`docs/DATA_PROTECTION_POLICY.md`、`docs/LIVE_STATE_DECISION_MATRIX.md`
- release executable: `target/release/codex_info`
- release SHA-256: `2f0c31ada866e2ca9ec8929a58540072e091ea961956a668719ca4f682c98eb3`
- `cargo fmt --check`: PASS
- `cargo check --locked`: PASS
- `cargo test --locked --quiet`: PASS、`153 + 168 + 1 + 14 + 36 = 372 passed; 0 failed`
- `cargo build --release --locked`: PASS
- ソース変更: なし（本報告ファイルのみ追加）

## 差分で確認できた正しい部分

`src/security.rs:536-586` は、改行済みoversizeを `LimitExceeded`、改行済みinvalid UTF-8を `Parse`、EOF未完了のinvalid UTF-8/oversizeを `Unterminated` と分類する。`src/thread_contract.rs:1239-1250` の strict parser は `Unterminated` を `RolloutError::UnterminatedLine` に変換し、`terminated == false` も拒否する。`parse_rollout_reader_recoverable` は `LimitExceeded | Parse` のみをskipし、`Unterminated`を返すため、live rolloutの strict/recoverable 境界は決定表と整合する。

新規境界テスト `jsonl_reader_rejects_unterminated_invalid_and_oversized_records`、既存の `recoverable_rollout_parser_keeps_running_state_around_large_tool_output`、`thread_c_rollout_bytes_utf8_jsonl_and_size_boundaries`、`oversized_tool_records_do_not_hide_following_usage_samples`、`malformed_tool_records_do_not_hide_following_usage_samples` は全て通過した。

## Blocking finding: valid UTF-8 EOF tailの受理

### 再現可能なコード経路

1. `read_bounded_jsonl_record` は、EOF時に `line` が空でなければ、valid UTF-8について `Ok(Some((line, false)))` を返す。
2. `read_bounded_jsonl_line` (`src/security.rs:589-592`) は `map(|(line, _)| line)` で `terminated` を破棄する。
3. `read_recoverable_session_line` (`src/main.rs:3626-3644`) は `read_bounded_jsonl_line` の `Ok(Some(line))` をそのまま返す。
4. `collect_session_file` / timeline collector (`src/main.rs:3806`, `3863`) は、そのvalid JSONを通常recordとしてparseし、前後の有効値とともに集計対象へ進める。

したがって、例えば有効なnewline付きrecordの後に、valid JSONだが末尾改行のないrecordを置くと、local historyがその不完全recordを採用し得る。`docs/DATA_PROTECTION_POLICY.md:39,52-54` の「ローカル履歴のEOF未完了レコード（valid/invalid UTF-8、oversizeを含む）は入力全体をrollback」と一致しない。

同じフラグ破棄は `RpcLine::read` (`src/security.rs:607-608`) にも伝播し、valid JSONのEOF未改行をRPC `Line`として扱う。JSONL framingをstrictにするなら、これは `Closed`/`Failed` ではなく未完了frameとして拒否すべき境界である。

### 証拠の不足

新規security testはinvalid UTF-8とoversizeのEOF未完了だけを検証し、valid UTF-8のEOF未改行を検証していない。production local-history側にもその回帰テストはない。既存のoversize/malformed record isolationテストは末尾を改行しており、この欠陥を覆わない。

## 決定表・文書整合性

live rolloutについては、`LIVE_STATE_DECISION_MATRIX.md` の「改行済みoversize/invalid-UTF8はrecord isolation」「EOF直前の未改行tailは次cycleまでhold」と strict parser/secure-open path が一致する。一方、同文書とDPPが区別している local history EOF rollbackは、`read_bounded_jsonl_line`経由のproduction collectorで満たされない。よって文書間の規範は一貫しているが、production実装が一部未達である。

L0のcandidate/active path/rollout running/graph/epoch/RPC AND条件、candidate reject、partial snapshot拒否、stale epoch no-opの既存テストはゲート内で通過している。ただし、上記record framing違反のため、これらの静的テストPASSだけで全要求PASSにはできない。

## 実環境受入ブロッカー

- `docs/evidence/LIVE_STATE_RUNTIME_2026-08-22.md` のX実行証跡は別SHA `124a41aa...` で、captureのviewport切れとUI HOLDを記録している。
- Windows証跡は現行Linux release SHA `2f0c31...` に結び付いておらず、受入文書自身がcurrent HOLD/historical扱いとしている。
- app-server停止→再起動によるactive path/epoch変化の実traceがない。
- 複数server/collector同時実行のadmission分離・stale event非上書きの実traceがない。

## Luna実装担当へ戻す最小修正タスク

1. `read_bounded_jsonl_line` が `Some((line, false))` を `SecurityErrorKind::Unterminated` として返すようにする、または全production callerをrecord APIへ移して `terminated` を必ず検査する。
2. valid UTF-8 EOF未改行について、security API、`RpcLine::read`、local collector（前後recordがあっても入力全体rollback）の回帰テストを追加する。
3. strict/recoverable rollout、local history、RPCの三経路で `LimitExceeded/Parse` のみrecoverable、`Unterminated`はfail-closedであることを再実行する。
4. 修正後の新しいrelease SHAに対して実X/Windows fresh image、停止→再起動trace、複数server traceを取得し、新規Lunaセッションで再評価する。

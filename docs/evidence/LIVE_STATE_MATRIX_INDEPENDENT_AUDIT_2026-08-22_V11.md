# Live-state matrix independent audit V11 (2026-08-22)

## 判定

`FAIL`（実環境証拠HOLD、およびlocal EOF契約の文書表現不一致）

V10で不足していたvalid JSON EOF未改行とoversize EOF未改行のlocal rollbackテストは確認でき、EOF実装のblocking code gapは解消している。ただし、受入完了に必要な現行release SHAへ結び付く実X/Windows画像・停止→再起動trace・複数server/collector traceは依然ない。また、`DATA_PROTECTION_POLICY.md` と `DESIGN.md` のlocal EOF契約の列挙が一致していないため、全要求PASSとはしない。

## 最新成果物とゲート

- release executable: `target/release/codex_info`
- release SHA-256: `816de5ddd00d8d12cc42b20fdac974b6a79d7da162e99d2c9d26a8824bb8726e`
- `cargo fmt --check`: PASS
- `cargo check --locked`: PASS
- `cargo test --locked --quiet`: PASS、`153 + 171 + 1 + 14 + 36 = 375 passed; 0 failed`
- `cargo build --release --locked`: PASS
- コード変更: なし（本報告ファイルのみ追加）

## EOF全分類の実装・テスト確認

`src/security.rs` は次の分類を実装している。

- 改行済みvalid UTF-8: `terminated=true` で受理。
- 改行済みinvalid UTF-8: `Parse`。recoverable層のみrecord isolation。
- 改行済みoversize: `LimitExceeded`。行末までdrainし、recoverable層のみrecord isolation。
- EOF未完了valid UTF-8: record APIは`(line,false)`、line/RPC APIは`Unterminated`。
- EOF未完了invalid UTF-8: `Unterminated`。
- EOF未完了oversize: `drain_until_newline`のEOF判定で`Unterminated`。

`src/thread_contract.rs` の strict/recoverable rollout parser は `Unterminated` と `terminated=false` を `RolloutError::UnterminatedLine` として拒否し、`LimitExceeded|Parse`だけをskipする。`src/main.rs` の `read_recoverable_session_line` はrecord APIで `terminated=true` を必須化し、local collector/timelineはエラー時に途中結果をrollbackする。

今回確認した回帰テストは次のとおり。

- `jsonl_reader_rejects_unterminated_invalid_and_oversized_records`
  - EOF invalid UTF-8、EOF oversize、EOF valid UTF-8のline/RPC拒否。
- `thread_c_rollout_bytes_utf8_jsonl_and_size_boundaries`
  - strict rolloutのvalid EOF tail拒否。
- `recoverable_rollout_parser_keeps_running_state_around_large_tool_output`
  - 改行済みoversize isolation。
- `oversized_tool_records_do_not_hide_following_usage_samples`
  - local collectorの改行済みoversize isolation。
- `malformed_tool_records_do_not_hide_following_usage_samples`
  - local collectorの改行済みmalformed JSON isolation。
- `unterminated_session_record_rolls_back_the_whole_local_input`
  - local EOF invalid UTF-8 rollback。
- `valid_json_unterminated_session_record_rolls_back_the_whole_local_input`
  - local EOF valid JSON rollback（末尾`{} `相当、改行なし）。
- `oversized_unterminated_session_record_rolls_back_the_whole_local_input`
  - local EOF oversize rollback。

以上により、今回要求されたEOF分類とlocal rollbackのテスト証拠は揃っている。`collect_session_timeline_file`にも同じterminated必須・events truncate経路があることをコードで確認した。

## 文書整合性のblocking finding

`docs/DATA_PROTECTION_POLICY.md:39,53` は、local historyのEOF未完了レコードを **valid/invalid UTF-8、oversizeを含む** と明記し、入力全体rollbackを要求している。一方 `DESIGN.md:117` は、同じlocal JSONL契約を「EOFで未完了のinvalid-UTF8/oversize record」と列挙しており、valid UTF-8 EOF未改行recordを明記していない。

実装とテストはvalid JSONも拒否するため動作上の欠陥ではないが、要求されたDESIGN/DATA_PROTECTION_POLICY契約の完全な統一には未達である。`DESIGN.md:117` をDPPと同じく「valid/invalid UTF-8、oversizeを含むEOF未完了record」と明記し、文書ゲートを再実行する必要がある。

## 残る実環境受入ブロッカー

1. `docs/evidence/LIVE_STATE_RUNTIME_2026-08-22.md` のX証跡は対象SHA `124a41aa...` であり、今回の `816de5dd...` ではない。900x480 captureはviewport切れを理由にHOLDとしている。
2. Windows証跡は現行Linux release SHAへ結び付いておらず、受入文書自身がhistorical/current HOLDと記録している。
3. app-server停止→再起動のactive path/epoch変化と、新しい完全snapshotだけを公開する実traceがない。
4. 複数server/collector同時実行のadmission分離、DB競合、旧epochイベント非上書きの実traceがない。
5. `docs/REQUIREMENTS_LEDGER.md` の `DP-001`、`DP-009`、`LIVE-001` は `HOLD` のままで、LIVE-001の証拠欄もV8を参照している。

## 最小の戻しタスク

1. `DESIGN.md:117` のlocal EOF未完了契約をDPPと同じvalid/invalid UTF-8・oversize列挙へ更新する。
2. 文書整合性ゲートを再実行する。
3. release SHA `816de5ddd00d8d12cc42b20fdac974b6a79d7da162e99d2c9d26a8824bb8726e` に結び付けた実X/Windows fresh image、停止→再起動trace、複数server traceを取得し、新規Lunaセッションで再評価する。

# Live state matrix 独立監査 V6（2026-08-22）

## 判定

`INCONCLUSIVE/HOLD`。最新版の `DESIGN.md` を正本として、V5の結論を前提にせず、現行作業ツリーの実装・テスト・文書を再確認した。Rustの静的ゲートとraw 369件は通過したが、最新版成果物SHAに結び付く実X/Windows画像、実app-server停止→再起動trace、複数server/collector同時実行traceがこの監査入力にない。加えて、rolloutのoversize/invalid-UTF8の扱いが `DESIGN.md`、`DATA_PROTECTION_POLICY.md`、production parserの間で一意に定義されていないため、PASSへはできない。

## 監査対象と成果物

- `DESIGN.md` のデータ契約（特に rollout JSONL、L0、local JSONL）
- `src/main.rs` の secure-open、snapshot境界、active thread適用
- `src/thread_contract.rs` の bounded rollout parser、cycle atomicity
- `src/security.rs` の bounded JSONL reader
- `docs/LIVE_STATE_DECISION_MATRIX.md`
- `docs/DATA_PROTECTION_POLICY.md`
- `docs/REQUIREMENTS_LEDGER.md`
- 現行release binary: `target/release/codex_info`
- 現行release binary SHA-256: `31a523e6797708381deffd6843513e4581c585c54649ae59e2a4674a2b8089fb`

ソースは変更していない。この報告ファイルだけを監査証跡として追加した。

## 検証コマンド

| コマンド | 結果 |
| --- | --- |
| `cargo fmt --check` | PASS (exit 0) |
| `cargo check --locked` | PASS (exit 0) |
| `cargo test --locked --quiet` | PASS (exit 0) |
| `cargo build --release --locked` | PASS (exit 0) |
| `bash scripts/completion_guard.sh` | FAIL (exit 1: latest independent subagent auditがPASSではない) |
| `bash scripts/requirements_intake_guard.sh` | FAIL (exit 1: WIN-PAR-06、WIN-PAR-14、WIN-ACC-02がunverified) |

`cargo test --locked --quiet` のraw結果は次の6 harnessである。

```text
running 152 tests
test result: ok. 152 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 167 tests
test result: ok. 167 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 13 tests
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 36 tests
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

合計は **369 passed / 0 failed / 0 ignored / 0 measured** である。

## 静的実装確認

- `src/main.rs:1090-1118` の `complete_rollout_prefix_len` は、secure-open時の長さを境界に末尾を調べ、最後の改行までを parser へ渡す。EOF直前の未改行tailは除外され、同じdevice/inodeへの追記後に次cycleで再読できる構造になっている。
- `src/main.rs:1126-1165` の `read_thread_rollout_path` は、canonical regular fileをopenし、open前後のdevice/inode、symlink、regular file、長さ縮小を検査する。失敗時は `Err(())` で candidate read を失敗させる。
- `src/thread_contract.rs:1229-1254` の strict parser は改行済みJSON不正、known event型不正、invalid UTF-8、oversizeをrejectする。一方、production pathは `src/main.rs:1147-1149` で `parse_rollout_reader_recoverable` を呼ぶ。
- `src/thread_contract.rs:1264-1303` の recoverable parserはbounded readerが改行まで消費した `LimitExceeded` と `Parse` を無条件にスキップし、前後の状態を継続する。既存テスト `recoverable_rollout_parser_keeps_running_state_around_large_tool_output` はこの挙動を固定する。
- `src/thread_contract.rs:1512-1575` は admitted candidateのread/parse失敗を `CycleError` とし、他candidateだけのpartial snapshotを返さない。`src/main.rs` の適用側も失敗時にcurrent active rowsを維持せず、完全な次cycleだけを受け付ける境界を持つ。
- `src/main.rs:9834-9900` の path-level fixture は、EOF末尾のpartial tailと同一inodeへのappendだけを確認する。改行済みmalformed line、EOF以外の部分行、途中縮小、inode差替え、symlink化を実際の `read_thread_rollout_path` で固定するテストは、この監査範囲では確認できなかった。

## 重大所見

### P1: rolloutの完全oversize/invalid-UTF8の正本が実装・設計・文書で不一致

最新版 `DESIGN.md:80` は「完全なoversize/invalid-UTF8ツール記録はその記録だけを隔離」とする一方、`DESIGN.md:86` は「改行済みのmalformed/oversize行…candidate file全体をreject」とする。`DESIGN.md:74` と `DESIGN.md:117` の local JSONL記述も、完全な不正/oversizeレコードを隔離する記述と、完全行をfile rollbackする記述が併存する。

production rollout pathが呼ぶ recoverable parserは、行の種別を検査せず `LimitExceeded` / invalid UTF-8 をスキップするため、少なくとも「tool recordだけを隔離し、その他の改行済みoversize行はrejectする」という読み方には一致しない。`docs/DATA_PROTECTION_POLICY.md:39,52` は oversized/invalid JSON/UTF-8の改行済み単一recordを隔離するため、DESIGN.md:86の全面rejectとも一致しない。どのイベント形状をrecord isolationの対象にするかを一つの正本へ確定するまで、要求適合は未確定とする。

### P1: tail/partialの決定マトリクスが最新版の境界を表現していない

`DESIGN.md:86` は「EOF直前の未改行tailは次cycleへ保留」と「改行済み不正行、EOF以外の部分行、途中縮小、inode差替えはcandidate全体reject」を分離した。対して `docs/LIVE_STATE_DECISION_MATRIX.md:23` は `invalid/partial` を一括してFAIL-CLOSEDとし、EOF直前未改行tailを保留する行と改行済み不正行を分離していない。`docs/DATA_PROTECTION_POLICY.md:39,52-53` は概ねtailを分けるが、rolloutのtool-record例外と完全oversize rejectの差を説明していない。文書間のAND条件・復帰条件をこの境界へ更新し、対応する固定試験名を登録する必要がある。

### P2: 最新境界の自動試験が不足

raw 369件は全て通過したが、確認できたtail fixtureは append + EOF partial の1系統である。最新版の受入条件を完全に証明するには、少なくとも次を個別に固定する必要がある。

- EOF直前の未改行tail（同じdevice/inodeのappendで次cycleに完成）
- 改行済みmalformed JSON行、known eventの型不正、oversize行、invalid UTF-8行
- EOF以外に現れる部分行
- secure-open後の途中縮小、inode差替え、symlink化、I/O失敗
- 失敗candidateが別candidateの完全snapshotへ混入しないcycle atomicity

strict in-memory parserの境界試験は存在するが、production secure-open pathの検証結果とは区別する。

## 実環境証拠の欠落

次の必須証拠は、現行binary SHA `31a523e6797708381deffd6843513e4581c585c54649ae59e2a4674a2b8089fb` に結び付いたものを確認できなかった。

- 実X Windowの最新PID起動・状態別/サイズ別captureと独立目視。既存 `docs/evidence/LIVE_STATE_RUNTIME_2026-08-22.md` は別SHA `124a41aa…` のcaptureで、viewport切れを理由にHOLDとしている。
- Windowsクライアントの現行SHAによるfresh image。`docs/evidence/WINDOWS_ACCEPTANCE_E2E_2026-08-22.md` 自身がhistorical/current HOLDと記録し、既存installer/image世代は現行Linux release binary SHAへ結び付いていない。
- app-server停止→再起動による `active_paths` 変化と、新しい完全snapshotだけの公開trace。
- 複数server/collector同時実行時のadmission分離、および旧epochが現行snapshotを上書きしない実trace。

このため、静的PASSと369テストPASSだけでは `LIVE-001` の実環境受入をPASSにできない。`docs/REQUIREMENTS_LEDGER.md:35,43,45` もDP-001、DP-009、LIVE-001をHOLDとしており、LIVE-001の独立証拠欄はV4を指したままである。

## 最小の戻しタスク

1. oversize/invalid-UTF8の「tool record isolation」と「改行済み不正行のcandidate reject」を、イベント形状まで含めて `DESIGN.md`、`DATA_PROTECTION_POLICY.md`、`LIVE_STATE_DECISION_MATRIX.md`、要求台帳へ一つの契約として整合させる。
2. 上記tail/partial、縮小、inode差替え、symlink、cycle atomicityをproduction pathで固定するテストを追加し、raw件数を再取得する。
3. 現行SHAを固定して実X/Windows fresh image、実停止→再起動trace、複数server traceを取得し、要求台帳の証拠欄と最新独立監査へ登録する。
4. completion/data-protection/regression gateを現行成果物で再実行し、新規Luna独立セッションで再評価する。

監査中のソース変更は行っていない。上記未達が解消されるまで、判定は `INCONCLUSIVE/HOLD` のままとする。

# データ保護要求台帳・完了チェックリスト

この台帳は、ユーザー要求を忘れず、実装・検証・終了判断を同じIDで追跡するための正本である。`open`、`partial`、`missing`、`unverified`、`inconclusive`が1件でも残る場合、完了を宣言しない。横断要求を含む最新監査は[要求実装監査](REQUIREMENTS_AUDIT_2026-08-22.md)である。

## 目的・非目的・依存関係

### 目的

- 既存の利用履歴とマスター可視化データを欠損・改変・重複から保護する。
- app-server停止、複数collector、巨大なログレコード、DB障害、migration失敗でも安全な保持境界を明示する。
- 今後の変更で同じ回帰を再発させない変更管理と独立評価を固定する。

### 非目的

- Codex Info全プロセス停止中の未取得値を捏造すること。
- 明示要求なしに常駐daemonを追加してCPU負荷を増やすこと（本件では過去要求によりdaemonが明示要求されているため、別IDで実装・検証する）。
- 3か月pruneをmigrationと偽って自動実行すること。

### 依存関係DAG

```text
要求台帳/保護規約
  -> 入力bounded validation
  -> cycle/auth/epoch admission
  -> UsageStore transaction/upsert
  -> backup/prune/migration boundary
  -> reload/graph/REST/Windows presentation
  -> static gate + tests + independent evaluation
```

## 要求から証拠への台帳

| ID | 要求（原文の要約） | 契約・境界 | 失敗時の保持 | 実装 | 独立した証拠 | 状態 |
| --- | --- | --- | --- | --- | --- | --- |
| DP-001 | デグレードを100%許容しない | 有効値を旧値より後退させず、失敗cycleを公開しない。live inventoryとlivenessを混同しない | 旧完全snapshot/DBを保持 | `src/main.rs`, `DESIGN.md` | `cargo test --locked`, live-state matrix、独立監査 | HOLD |
| DP-002 | 巨大tool出力で履歴を欠損させない | 1レコード単位でoversize/invalid JSON/UTF-8を隔離 | 前後の有効recordを保持。I/O等はfile rollback | `src/main.rs:read_recoverable_session_line`, `src/thread_contract.rs` | oversized collector、recoverable rollout、security JSONL tests | verified |
| DP-003 | app-server停止中の変動を可能な範囲で記録 | persisted reset hintがありcollectorが動作中ならbackfillを1回実行 | 未認証中は公開せず、DB再読込後に表示 | `src/main.rs:recovery_period` | persisted backfill、one-shot latch test、[実動作証跡](evidence/DATA_PROTECTION_RUNTIME.md) | verified |
| DP-004 | 常時負荷を増やさない | 常時全session scan/即時再起動ループを禁止 | 障害期間の復旧はbounded one-shot | `src/main.rs:recovery_requested`, `poll` | one-shot latch test、gate実行、[実動作証跡](evidence/DATA_PROTECTION_RUNTIME.md) | verified |
| DP-005 | 複数server/clientの二重登録とprofile/account間混在を防ぐ | logical partition `(ProfileScopeId,AccountScopeId,StorageEpoch)`を確定し、`(partition_id,reset_at,timestamp)`一意、transaction upsert、remaining COALESCE、cost/token MAX | identity不一致またはbusy/commit失敗は全batch rollbackし旧DB/rootを保持 | `src/usage_store.rs`（新partition契約は未実装） | cross-profile/account fixture、concurrent writer、same-operation no-duplicate、row/hash oracle | HOLD / PRODUCT_PENDING |
| DP-006 | DBバックアップを数世代保持 | prune前にSQLite online backupを3世代作成し、各世代をprivate modeで保持 | backup失敗時はprune禁止。元DBは置換しない | `UsageStore::backup_generations`, `UsageHistory::startup_maintenance` | quick_check/reload付きbackup test、gate実行、[実動作証跡](evidence/DATA_PROTECTION_RUNTIME.md) | verified |
| DP-007 | 過去データをmigrationで壊さない | 旧schemaは暗黙変換せず拒否。明示migrationは別DB検証後atomic switch | 旧DB・旧backup・旧memoryを保持 | `UsageStore::open`, `UsageStore::migrate_verified`, policy §4/§5 | old-schema rejection、candidate migration success/failure、corrupt DB preservation tests | verified |
| DP-008 | daemonは過去要求どおり常駐記録部として導入し、暗黙の無制限負荷にはしない | 明示されたdaemonを専用プロセスとして起動し、interval・singleton・停止・更新・監視手順を固定する | daemon停止時に未取得を捏造せず、既存DBを保持する | `src/daemon.rs`, `src/main.rs`, `scripts/record_daemon_e2e.sh` | daemon unit tests、REST+daemon live PID/lock/追記/CPU trace、[実動作証跡](evidence/DATA_PROTECTION_RUNTIME.md) | verified |
| DP-009 | 要求を忘れず最後に漏れをなくす | データ保護だけでなくWindows parity、導入、daemon、実画面、live-state matrixを同じ台帳で追跡する | open/partial/missing/unverified/inconclusiveを完了扱いしない | policy §5、要求実装監査、CI gate | Windows acceptance gate、DB E2E、live-state matrix、fresh image hashes、独立サブエージェント監査 | HOLD |
| DP-010 | 前回の原因を記録し再発防止する | 原因・境界・禁止事項・確認コマンドを文書化 | 「今回だけ実行」は不合格 | policy §6、gate script | policy presence check、独立評価 | verified |
| LIVE-001 | DBの想定外状態で停止済みスレッドを実行中として表示しない | DBはinventory、live判定は同一cycleのactive path + rollout terminal stateの二重証明。root/child/empty/mixed/DB欠落・重複・cycle・process再起動・複数server・RPC/stale epochを表形式で判定し、矛盾は旧完全snapshot保持 | 停止済み行を公開しない。判定不能cycleは部分結果を公開しない | `src/main.rs`, `DESIGN.md` L0, `DATA_PROTECTION_POLICY.md` §2-11 | `docs/LIVE_STATE_DECISION_MATRIX.md`、`docs/evidence/LIVE_STATE_MATRIX_INDEPENDENT_AUDIT_2026-08-22_V11.md`、production secure-open境界・root/child active-path・候補部分読取atomic拒否・失敗→完全snapshot復帰テスト | HOLD |

## 工程ガバナンス原子契約

次の3行は製品226 ID・旧96 ID・製品依存DAGへ加えない工程専用namespaceである。ただし完了判定では
製品要求とAND結合し、1行でも未達ならterminal PASSを0にする。チャット基盤やAPI turnの終了後もprocessが
生存するという主張はせず、観測できるactive goal、agent snapshot、tracker、continuation eventだけを証拠にする。

| ID | actor | trigger / precondition | exact state / transition | acceptance | negative / failure | retention / idempotence | owner / dependencies | independent evidence / oracle | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| GOV-THREAD-END | extraction coordinator、orchestrator supervisor、独立評価者 | unresolved countが1以上、または未完了child、HOLD、FAIL、INCONCLUSIVE、timeout、disconnect、response loss、interruptが存在する | `ACTIVE_GOAL→HOLD_OR_INCONCLUSIVE→REASSIGNMENT_RECORDED→ACTIVE_GOAL→VERIFIED→TERMINAL`。platform/tool強制終了では`turn_end_unobserved`と`next_turn_required=true`を記録し、実reassignment観測前に再配置済みとしない | unresolved>0ならterminal_pass=0、forced_end>0ならcontinuation_record=1、active_owner<=1、terminal_pass=1ならunresolved=0かつindependent_pass=1 | timeout/切断/API turn終了をPASSへ変換、実eventなしの継続・再配置・liveness主張、旧ownerのlate callbackによるPASSを禁止 | goal/work/subagent ID、owner、source/freeze SHA、last completed ID、reason、event UTC、continuation_epochを保持。同一work/event/source/epochはno-op | owner=`/root`; governance joins=`AGENTS.md`,`COMPLETION_PROTOCOL.md`,orchestrator live state,tracker; product hard DAGには非参加 | live agent snapshot、goal state、tracker、raw turn-boundary/continuation event、source SHAを別評価者が再計算。terminal偽装0、4つのacceptance式一致が必要 | REQUIREMENTS_PASS / CONTINUOUS_GATE |
| GOV-NO-INPUT-END | extraction coordinator、orchestrator、独立評価者 | authority、decision、evidence不足により入力・確認・判断を求めたくなるが完了条件未成立 | `OPEN_DECISION→DETERMINISTIC_CONTINUE_OR_HOLD→REASSIGNMENT_RECORDED→ACTIVE→DECISION_CAPTURED→VERIFIED`。`WAITING_FOR_INPUT`はnonterminal表示だけで、terminal stateではない | 最終eventを質問/入力待ちだけにせず、未決事項・根拠・影響・仮定禁止理由・last IDを保持し、read-only抽出・矛盾整理・証拠要件化を継続。回答到着を観測なしに推測しない | synthetic decision、暗黙user approval、`WAITING_FOR_INPUT→TERMINAL`、入力なしをPASS根拠化、API turn/subagent生存の捏造を禁止 | source、question reason、attempt、last step、continuation_epochを保持。同一未決eventはno-op、後着回答は同一decision IDへ1回だけjoin | owner=`/root`; governance joins=`GOV-THREAD-END`,`REQUIREMENTS_INTAKE_POLICY.md`,`COMPLETION_PROTOCOL.md`,tracker | waiting_terminal_count=0、unresolvedならpass=0、回答なしでopen_decisionが消えない、raw turn boundaryと次回再開記録一致を独立監査 | REQUIREMENTS_PASS / CONTINUOUS_GATE |
| GOV-ESCALATION-100X | extraction coordinator、ledger maintainer、fresh independent extractor/evaluator、machine gate | `EXTRACTION_COMPLETE`承認後に未記載の要求・状態・境界・失敗・保持・証拠を発見、または誤記/漏れ判定不能 | `EXTRACTION_COMPLETE→INVALIDATED_OR_HOLD→ESCALATION_RECORDED→RAW_REEXTRACTING→INDEPENDENT_RECONCILIATION→VERIFIED`。approved_N=226ならtarget=226000、approved_N>226ならtarget=max(226000,N×100) | discovery source/time、old N、式、target、影響ID、split理由、preventionを保持し、各work unitへunique ID、根拠、境界、failure/retention、UX理由、試験、raw証拠、独立判定式を割当。fresh extractor PASS前はimplementation_resume=0 | target削減、誤記扱いによる除外、重複水増し、synthetic product ID、旧PASS再利用、独立評価なし再開を禁止 | approved N/source/freeze SHA、omission event、旧ledger、raw source manifest、last-good extraction、escalation_epochを保持。同一source/omissionはno-op、別発見はepoch+1 | owner=`/root`; governance joins=`RC-172,RC-173`,`REQUIREMENTS_INTAKE_POLICY.md`,baseline,canonical,tracker; product hard DAGには非参加 | omission_after_approvalならstate=HOLD、式の再計算、invalid_row>0ならPASS=0、independent_pass=falseならimplementation_resume=0。product_id_set=226と条件発火後governance_work_unit_targetを区別 | REQUIREMENTS_PASS / CONTINUOUS_GATE |

## 終了前チェック

- [x] 変更が対象ファイル範囲を越えていない（横断監査で対象を要求台帳に固定）
- [ ] 台帳の全IDが`verified`で、`open`/`partial`/`missing`/`unverified`/`inconclusive`がない（LIVE-001の表形式・実機・独立証拠待ち）
- [ ] source→validation→state→persistence→presentationの各境界を確認した（live inventory→liveness境界を追加監査中）
- [x] malformed、empty、multiple writer、server unavailable、daemon停止、restart、auth switch、schema mismatchを確認した
- [x] DB本体のrow count/hashが、許可された3か月prune以外で減少していない
- [x] backup世代をquick_checkし、元DBを自動復元・再生成していない
- [x] `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo build --release --locked`が成功した（データ保護変更時点）
- [ ] `scripts/data_protection_gate.sh`が最新の統合台帳で成功した（現行成果物は独立監査HOLDのため未達）
- [x] 最新runtime traceと、必要なUI変更なら最新画像を取得した
- [ ] 独立サブエージェントが統合台帳を実装者の結論なしで評価し、全ID PASSした（現行成果物はHOLD）

このチェックは2026-08-22の変更に対する記録である。次回、対象ファイルを変更する場合は全項目を未確認へ戻し、再実行してから再度`[x]`へ更新する。

## 完了報告に必ず記載するもの

1. 変更した保護規約・実装・台帳ID
2. DB保護の実測（row count/hash、backup世代、quick_check）
3. 実行した全コマンドと結果
4. 独立評価のID別結果
5. 未確認事項（1件でもあれば完了扱いにしない）

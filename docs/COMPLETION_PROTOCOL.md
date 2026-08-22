# 完了判定プロトコル

要求を一つでも未実装・未確認のまま「完了」と報告しないための固定手順。

## スレッド終了禁止（必須）

要求抽出・要件検証・実装・テスト・独立評価・監視のいずれかが未完了、未確認、`HOLD`、
`INCONCLUSIVE`、FAILである間は、主スレッド、監視スレッド、サブエージェントを自主終了しない。
タイムアウト、切断、応答欠落、割り込み、セッション終了は完了の根拠にならず、直ちに
`HOLD/INCONCLUSIVE` として追跡台帳へ記録し、別担当へ再配置して継続する。固定完了文言を
使用できる状態になるまで、処理を完了終了してはならない。

未完了のままユーザーへ入力・確認・判断を求め、その返答待ちを理由に処理を終了することも
禁止する。判断不能な項目は台帳へ `open`/`INCONCLUSIVE` として残し、既存資料から確定できる
抽出・矛盾整理・証拠要件化・独立突合を継続する。

最終応答を `100%要求を満たしました。完了しました。` 以外の文言で終えてはならない。この
固定文言を使用できない間は、ターンを完了扱いで終了せず、要求抽出・実装・検証・独立評価・
監視を継続する。チャット基盤のターン終了など実行環境による停止は完了やPASSと扱わず、
`HOLD/INCONCLUSIVE` を記録し、次回実行時に質問を返さず未完了台帳から再開する。

1. `docs/REQUIREMENTS_LEDGER.md`、`docs/WINDOWS_CLIENT_REQUIREMENTS.md`、要求監査表を更新する。
1a. 新規変更は先に `docs/REQUIREMENTS_INTAKE_POLICY.md` と `scripts/requirements_intake_guard.sh` で要求抽出・試験割当を登録する。抽出未完了なら実装を開始してはならない。
2. `bash scripts/completion_guard.sh` を実行する。`open`、`partial`、`unverified`、`inconclusive` が一つでもあれば終了コード1で停止する。
3. データ保護対象の変更では、`bash scripts/data_protection_gate.sh` も実行する。DP-009を含むfail-closed判定を無視しない。
4. `bash scripts/regression_guard.sh` を実行し、過去に受入した固定契約がソースから消えていないことを確認する。FAILなら完了扱いにしない。
5. 実画面・実プロセス・実DB・実Windows導入が要求された項目は、静的コード確認だけでverifiedへ変更しない。
6. 主担当の自己判定とは別に、独立サブエージェントへ実装者の結論を渡さず、最新の要求台帳・差分・raw証拠との突合を依頼する。独立判定がFAILまたはINCONCLUSIVEなら完了扱いにしない。

## 必須・無視禁止のサブエージェント追跡ゲート（GOV-SUBAGENT-01）

これは任意のレビュー、今回限りの確認、担当者の善意ではない。要求追跡と独立評価を
サブエージェントで実施することを、今後すべての変更に適用する強制制約とする。

### GOV-THREAD-END / GOV-NO-INPUT-END 状態機械

工程状態は`REQUIREMENTS_LEDGER.md`の3つのGOV原子行を正本とする。未解決中の正常な遷移は
`ACTIVE_GOAL→HOLD_OR_INCONCLUSIVE→REASSIGNMENT_RECORDED→ACTIVE_GOAL→VERIFIED→TERMINAL`である。
`unresolved_count>0`、未完了agent、HOLD、FAIL、INCONCLUSIVEのいずれかがある間は
`terminal_pass=0`である。強制的なplatform/tool/API turn境界は完了ではなく、
`turn_end_unobserved,next_turn_required=true,continuation_epoch=n`を記録する。turn終了後にもagent/processが
生存するというliveness claimを作らず、実際のorchestrator eventを観測する前に`REASSIGNMENT_RECORDED`へ進めない。

判断不足時は`OPEN_DECISION→DETERMINISTIC_CONTINUE_OR_HOLD→REASSIGNMENT_RECORDED→ACTIVE`を使う。
`WAITING_FOR_INPUT`はnonterminalな表示状態だけで、`WAITING_FOR_INPUT→TERMINAL`を禁止する。実回答なしの
synthetic decision、暗黙のuser approval、入力なしをPASS根拠にすることは禁止する。read-only抽出、矛盾整理、
証拠要件化、独立突合を継続し、同一未決eventの再観測はidempotent no-op、後着回答は同じdecision IDへ1回だけ結合する。

独立oracleはgoal/agent snapshot、tracker、source/freeze SHA、raw turn-boundary/continuation eventを再計算し、
`waiting_terminal_count=0`、`unresolved_count>0ならterminal_pass=0`、`active_owner_count<=1`、
実eventなしの再配置/liveness主張0を同時に満たす必要がある。これはRC-172/RC-173へ1:1 joinする。

### 並列分担（加速）

独立領域（要求抽出、UX、ライフサイクル、データ保護、Windows受入など）は、所有範囲を
重複させない複数スレッドへ並列分担できる。各スレッドを台帳へ登録し、raw結果と判定を
保存する。スレッドの部分PASSを統合PASSへ丸めず、全スレッドの完了・同一SHA突合・独立判定が
揃うまで主担当は継続する。ここでいう同一SHAは同じartifactに対する検査間の一致を意味し、
X/Linux binaryとWindows installerのように内容が異なるartifact同士は、artifact別SHAを同一source
commit・release manifest・fixture/data hashへ連結する。途中終了・タイムアウト・証拠欠落は当該行を即時HOLD/INCONCLUSIVE
として再配置し、ユーザー入力待ちで終了しない。

- `docs/REQUIREMENTS_LEDGER.md` と `docs/WINDOWS_CLIENT_REQUIREMENTS.md` の全行を、担当・実装・証拠・独立判定ID付きで追跡する。
- 変更開始時に `docs/AGENT_REQUIREMENTS_TRACKER.md` へ担当エージェント、対象ID、所有ファイル、受入ゲートを登録する。未登録の作業は開始してはならない。
- 実装者とは別の新しいサブエージェントが、実装者のPASS結論を見ずに最新差分とrawログを再評価する。エージェントが途中終了、タイムアウト、証拠欠落した場合は `INCONCLUSIVE` とし、完了を禁止する。
- 最新SHA・最新画像・最新テスト結果に紐づく独立判定が `PASS` になるまで、リリースマニフェストは `RELEASE HOLD / 提出不可` を維持する。
- `scripts/completion_guard.sh` は `docs/INDEPENDENT_AUDIT_LATEST.md` の最新判定が `status: PASS` であることを必ず検査する。この検査を削除・迂回・手動で置換してはならない。
- `docs/AGENT_REQUIREMENTS_TRACKER.md` の成果物SHAは独立監査の `artifact_sha256` と一致しなければならない。不一致・未登録・途中終了は `INCONCLUSIVE` として完了を禁止する。
- 台帳に `HOLD` または `INCONCLUSIVE` の担当行が残る場合、completion guardは停止し、同じ要求を新しい担当へ再割当する。
- `docs/TEST_GAP_REGISTER_2026-08-22.md` のギャップを要求台帳へ登録し、未確認項目を残したまま完了へ進めてはならない。
- 継続監視が必要な変更では `scripts/requirements_watchdog.sh` を起動する。監視は `completion_guard`、要求抽出、回帰、データ保護の全ゲートが同一作業ツリーでPASSになるまで終了しない。`--once` のFAIL、SIGTERM/SIGINT、サブエージェントの途中終了、タイムアウト、ログ欠落はすべてHOLD/INCONCLUSIVEとして扱い、完了の根拠にはできない。監視は状態確認だけを行い、製品データ・入力デバイス・既存プロセスを操作しない。
- 100%要求達成以外で実装・検証・監視を停止することは最大レベルの規約違反である。途中終了・応答欠落・タイムアウト・セッション終了は即時に新しい担当へ再配置し、処理を継続する。停止状態を完了として扱わず、`HOLD/INCONCLUSIVE` を解除してはならない。

上記のどれか一つでも欠けた場合、コードが動作していても完了・納品・PASSを報告してはならない。

このプロトコルは今回限りの注意書きではなく、今後の変更でも必ず再実行する完了制約である。現行成果物は独立判定とWindows受入が未完了のため `RELEASE HOLD / 提出不可` である。いずれかのゲートがFAILまたはINCONCLUSIVEになった時点で出荷を停止する。

# Codex Info 開発ガイド

- ユーザーの変更と無関係な差分を戻さない。依頼なしにcommit、push、PR作成をしない。
- 製品要件の正本は`docs/PRODUCT_REQUIREMENTS.md`とし、wire、データ、UIの詳細は同文書が参照する仕様へ置く。
- 監査版ごとのEvidence、文書SHA一覧、サブエージェント作業台帳をrepositoryへ追加しない。再現可能なtestと必要最小限の仕様を残す。
- 要件漏れを理由にN倍、N二乗、N階乗、全直積へ展開しない。同じ観測結果は既存要件へ統合し、因果関係のある有限caseだけを追加する。
- 文書ごとのSHAを完了ブロッカーにしない。製品artifactを一意に識別するhashは記録できるが、内容評価の代用にしない。
- 外部authority値を推測しない。publisher、certificate、対応OS、保証値が未指定なら、公開・対応表明・mutationを行わないfail-closed動作を要件化する。
- lockfileと既存package managerを守る。検索は`rg`を優先する。
- 変更後は対象に最も近いformatter、check、testを実行する。testが0件の結果をPASSにしない。
- UI変更は実画面、Windows固有動作は実Windowsで確認する。環境上確認できない項目をPASSと報告しない。
- 画面キャプチャを取得した事実だけをUI評価の証拠にしない。各画像では可視色・文言・状態・配置を意味単位で棚卸しし、正本または他platformとの対応を説明できない色・表示差を1件でも残したままPASSにしない。可能な項目はpixel/UI Automationで機械判定し、その結果と同じ最新画面をレビューする。

## 共有repositoryのbranch・worktreeガバナンス

### 正本、用語、branchの責務

- `origin`は共有GitHub repositoryを指し、remote headは`origin`上の`refs/heads/*`を指す。Codexは他利用者が所有するbranch、worktree、変更、PRを推測で変更または削除してはならない。
- `/home/salty/code/codex_info_v2`をユーザー通常worktreeとする。`feat/next`はユーザーが確認・統合するbranchであり、Codexにとってread-onlyとする。
- `main`は本番・Releaseの正本とし、Codexは直接pushしてはならない。`feat/next`から`main`へのPR、承認、merge、close、Release判断はユーザーが管理する。
- Codexが書込み可能なbranchは、宣言済みの完全な`origin/feat/next` SHAから作る一時branch`codex/<task>`だけとする。PRの方向は`codex/<task> -> feat/next -> main`に固定し、Codexは`main`向けPRを作らない。
- `active task`は、ユーザーが宣言内容を明示許可してからcleanupが完了するまでを指す。`at rest`はactive taskが0件の状態を指し、Codexが作成したlocal/remoteの`codex/*` branchと追加worktreeが残ってはならない。未知のbranchは他利用者所有として保持し、Codexは削除せず報告する。

### worktreeを使う理由と安全境界

- Codexによる編集、test、build、formatter、生成物作成は、許可済み一時branchをcheckoutした一時worktree内でだけ行う。ユーザー通常worktreeでは`status`、`diff`、`show`、`log`、`worktree list`、`branch list`、`ls-remote`等の読取りだけを許可する。
- Codexはユーザー通常worktreeでcheckout、switch、add、commit、push、reset、rebase、merge、cherry-pick、stash、clean、update-ref、Git config変更、test、build、formatter、生成物作成を行ってはならない。`fetch`も共有Git状態を変更するため、許可前は行わない。
- worktreeには、ユーザー通常worktreeを汚さない、branch切替を不要にする、固定SHAで検証できる、独立したbuild出力を持てる、Git objectを共有して高速・省容量になる利点がある。
- worktreeは`.git`、object database、refs、config、hooks、remoteを共有するため完全なsandboxではない。branch責務、明示許可、path所有権、fail-closed停止を安全境界とし、これらを省略してworktreeを使用してはならない。
- 書込み作業で一時worktreeを使用する利点がない場合、Codexはユーザー通常worktreeへfallbackせず、その書込み作業を開始しない。

### 作成前のpreflightと明示許可

- Codexはbranchまたはworktreeを作る前に、`git status`、`git worktree list --porcelain`、local/remote branch、Open PR、対象pathの所有・dirty状態を読取り確認する。
- baseは`git ls-remote origin refs/heads/feat/next`が返す単一の40桁object IDとする。不存在、複数、取得不能、malformedの場合は停止し、localの`feat/next`、`main`、`FETCH_HEAD`、古いremote-tracking refで代替してはならない。
- Codexは次の全項目をchatで宣言し、宣言後のユーザー本人による明示許可を待たなければならない。宣言自体や過去タスクの許可を、今回の許可と解釈してはならない。

```text
Worktree使用申請
目的:
必要性と利点:
代替手段では不足する理由:
canonical worktree path:
一時branch名:
origin/feat/nextの完全なbase SHA:
owned files / paths:
変更しない範囲:
許可を求める操作（edit/test/commit/push/PR等）:
予定時間と完了予定時刻:
検証方法:
統合方法とPR target:
cleanup条件と削除予定:
```

- 許可は宣言したbranch、path、base SHA、owned scope、操作、期限にだけ有効とする。base、scope、path、操作、統合方法、予定時間が変化または超過した場合、あるいはユーザーが取消した場合は直ちに停止し、変更点を宣言して再許可を待つ。
- 許可後のfetchは、宣言した`origin/feat/next` refと完全SHAの取得に限定する。fetchが`FETCH_HEAD`と共有object database等を変更することを前提とし、作成直前にremote SHAが宣言値と同一であることを再確認する。
- 一時branch名は一意な`codex/<task>`、canonical worktree pathは`/home/salty/code/codex_info_v2-wt-<task>`とする。既存local/remote branch、既存path、symlink、別worktreeと衝突する場合は作成せず停止する。

### ownership、実装、検証、統合

- 1 taskにつき一時branch 1本、一時worktree 1個とする。同じfile/pathにwriterを1人だけ割り当て、owned pathsと変更禁止範囲を宣言する。
- `AGENTS.md`、workflow、lockfile、共通仕様、要件台帳等のcross-cutting fileは排他的所有とし、同時に別のwritable taskを走らせてはならない。ユーザー通常worktreeの同じpathにdirty/untracked変更がある、ownershipが重複する、または一時worktreeに不明差分が現れた場合は停止する。
- サブエージェントは宣言済み一時worktreeとowned pathsだけを扱う。サブエージェントによるbranch/worktreeの作成・削除、ref/config/remote操作、commit、push、PR操作を禁止し、管理は主担当だけが行う。
- Codexは宣言した最小のformatter、check、testを実行し、0件のtestをPASSにしてはならない。既往障害、security、cross-cutting governance等で独立判断が必要な場合だけfresh evaluatorを使う。
- commitはユーザーが明示許可した場合に限り、owned filesだけをstageして行う。pushとPR作成も宣言に含まれ明示許可された場合だけ行い、Codexが作るPRのbaseは`feat/next`に限定する。
- CodexはPRを勝手にapprove、ready化、merge、close、auto-merge設定してはならず、workflowをapprove、rerunしてはならない。これらは、ユーザーがexact targetと操作を別途明示許可した場合だけ実施できる。
- pushまたはPR作成が許可されていない作業を「統合済み」または「完了」と報告してはならない。実装済み、local検証済み、未統合を区別して報告する。
- pushまたはPR作成直前に`origin/feat/next`の完全SHAを再確認する。宣言baseから進んでいる場合は、旧SHA、新SHA、競合し得るowned pathsを報告して停止し、rebase、merge、reset、cherry-pick、stash、force pushを行わず再許可を待つ。
- repositoryのworkflow契約が別途変更されない限り、required acceptance、version準備、CodeQL、Release前gateは`feat/next -> main` PRが所有する。`codex/<task> -> feat/next`では宣言済みlocal gateとreviewを行い、versionまたはRelease mutationを行わない。`feat/next`向けCIを追加する場合はRelease経路から分離した別設計・別許可とし、`main`向けtriggerを単純に広げてはならない。

### race、cleanup、復旧、報告

- cleanupは主担当だけが、宣言済みbranchとcanonical pathに対して行う。merge/fast-forwardではcommit ancestryを確認し、squash、rebase、cherry-pick相当ではdeclared diffがtargetへ反映されたことと受入checkを確認する。PRの状態だけを統合証拠にしてはならず、同等性が曖昧な場合は削除しない。
- cleanup前に一時worktreeのtracked/untracked差分と生成物を確認する。不明差分、未commit変更、未統合のunique commitがある場合は保持し、完全SHA、対象path、PR状態、復旧方法を報告してユーザーのintegrate/discard判断を待つ。
- cleanupにはforceなしの`git worktree remove`、統合済みbranchに対する`git branch -d`、許可済みremote一時branchの削除だけを使用する。`rm -rf`、`git branch -D`、`git worktree remove --force`、`git clean`、force pushを禁止する。
- task開始時と終了時に、時刻、canonical worktree path、branch、完全なbase/HEAD SHA、dirty状態、owned/non-owned scope、許可された操作と期限、check結果、commit、push、PR URL/state、残存worktree/local/remote ref、cleanup結果、復旧手段を、terminalで再現できるコマンドとともにchatで報告する。repositoryへagent台帳やEvidence文書を追加してはならない。

## サブエージェントのコスト・速度ガバナンス

- 目的は総Codex消費量とwall-clock timeを同時に削減することであり、サブエージェント使用自体を成功指標にしてはならない。短期・決定論的・逐次的な作業は主担当がlocal commandで処理し、サブエージェントを使わない。
- 委譲は、独立した並列作業、専門的な独立判断、または低コスト監視によって、調整・待機・再読込を含むend-to-end総コストか完了時間が改善すると事前に説明できる場合だけ行う。固定のmodel familyを理由に委譲してはならない。
- 主担当SOLがサブエージェントの完了だけを待つactive turnや短周期pollingを継続してはならない。SOL側に有用な並行作業がなく、passive waitがSOL消費を発生させないと確認できない場合は、そもそも委譲しない。
- 委譲時は`fork_turns = "none"`、最小のtask-local context、限定owned scope、1つの有用なvalidation gate、最小出力を使用する。raw logや会話全履歴を渡さず、同じrevision・入力・役割・modelでの重複実行や、timeoutだけを理由とする再試行を禁止する。
- 主担当はサブエージェントと同じ調査・実装・監視を重複して行わない。利用可能なterminal結果を保持し、raw command evidenceをagent summaryより優先する。証拠と矛盾するverdictは無効とする。
- コスト削減効果は比較可能なend-to-end実測がある場合だけ主張する。利用量が取得できない場合は推測せず`unavailable`とし、wall-clockだけをtoken/cost削減の証拠にしない。
- 継続計測は製品deliveryと分離した、ユーザーが別途許可する評価作業として行う。計測のためだけにmodel、route、subagentをprobeせず、通常作業から自然に得られるtask分類、revision、solo/delegated、agent数、利用可能なusage、wall-clock、SOLのwait/poll回数、再作業、gate結果だけを最小量で収集する。
- 計測記録をこのrepositoryへ追加せず、製品taskのcritical pathへ入れない。比較可能なsampleが蓄積するまで一般的な節約効果を断定せず、評価方法自体のコストが便益を上回る場合は計測を停止してユーザーへ報告する。

## 設計整合性と実装方針

- 症状ごとに既存コードをコピー＆ペーストし、条件分岐・例外処理・別実装を継ぎ足す増改築を禁止する。変更前に正本、責務、状態遷移、データフロー、不変条件、失敗時の所有者を特定し、その全体設計に沿って実装する。
- 同じ製品機能を複数プラットフォームへ提供する場合、データ解釈、計算、表示意味、操作契約は一つの正本から導出する。UIフレームワーク固有コードは描画と入力のadapterに限定し、ユーザーの明示承認なしに独自仕様・独自画面・並行する計算ロジックを作らない。
- 既存の共通モデルまたは正本を拡張すれば解決できる問題に、第二のsource of truth、互換用コピー、場当たり的fallbackを追加しない。重複が既にある場合は、さらに分岐を足すのではなく責務境界を整理して収束させる。
- 不具合修正は、表示された症状だけを隠すパッチではなく、原因となった設計境界を修正する。例外的な分岐が必要な場合は、適用範囲と終了条件を有限の受入テストで固定する。
- reviewと完了判定では、変更行の局所的な正しさだけでなく、正本から最終表示・操作までの経路が一貫し、類似機能との不要な差異や新しい重複を生んでいないことを確認する。

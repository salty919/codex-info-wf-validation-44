# Codex Info 製品要件

この文書は、WindowsクライアントとLinux側collector/APIの実装判断に必要な要件だけをまとめた正本である。
監査履歴、作業経過、文書SHA一覧は製品要件ではないため含めない。詳細なwire schemaは
`REST_API_V1.md`、データ保持は`DATA_PROTECTION_POLICY.md`、画面仕様は`WINDOWS_UX_SPEC.md`を参照する。

## 1. 製品境界

- WindowsクライアントはLinux側の利用状況、残量、履歴、実行中threadを読み取り表示する。
- 製品の通信は固定loopback API、利用者が選択したWSL、またはOpenSSH configのliteral `Host` aliasに限定する。
- password、token、private key、展開済みSSH接続情報を保存しない。保存できるのは再接続に必要な非秘密selectorだけである。
- telemetryは送信しない。更新確認だけは固定GitHub repository
  `salty919/codex_info_v2` の公開Release API/assetを読み取れる。これ以外の未登録の外向き通信、
  暗黙のsupport upload、raw diagnostic uploadを行わない。

## 2. 状態と表示

- Setup、未認証、通常、警告、取得失敗を区別する。取得失敗を未認証や正常emptyへ変換しない。
- 更新candidateが不完全、不正、世代不一致の場合は画面の一部だけを更新せず、最後の完全な表示を保持する。
- 同じ事実の表示所有者は1か所とし、残量、reset countdown、status、connection情報を言い換えて重複表示しない。
- 定期的なquota再取得は不完全な中間結果であり、次のローカル使用量取得が完了するまで前回コミット済みのモデル・履歴・thread表示を保持する。認証主体の変更または明示的なログアウト以外で、主画面を空の初期状態へ戻してはならない。
- 最小viewport、対応locale、keyboard、UIA、高contrast、text scaleで主要操作を失わない。root scrollで欠落を隠さない。
- Back、Close、Escape、再入、遅延callbackは世代tokenで一度だけ処理する。古いPID/HWND/generationへfocus、message、route変更を行わない。
- 起動モードは固定する。引数なし/`--port PORT`はdaemon+RESTのみ、`--ui`はdaemon+REST+X UI、`--ui --port PORT`は指定ポートで同じ動作をする。待受アドレスは常に127.0.0.1へ固定する。
- `--stop`は同一profileの完全に検証できたlock ownerだけへTERMを1回送り、lock解放を最大5秒待つ。lockが無ければ成功とし、lockがあるのにowner identityを証明できない場合、別ownerへの交代、timeout、signal失敗は何も削除せず失敗する。SIGKILLへ昇格しない。
- 公開引数は上記、`--stop`、`--help`/`--h`/`-h`だけとする。旧・未知・誤記・重複・欠落・逆順・余分な引数、範囲外portを、daemon、REST、UI、DB、lockを作る前に拒否する。
- CLIヘルプを含む利用者向け固定メッセージは、画面本体と同じ対応localeのi18nカタログから導出する。起動スクリプトへ単一言語の製品文言を複製しない。

## 3. 収集・API・live判定

- 同一profileのwriter、recorder、REST publisherはそれぞれ有効ownerを1つだけ持つ。旧lease、旧epoch、旧cycleの結果は公開しない。
- DBは履歴inventoryであり、実行中判定の単独根拠にしない。同一cycleで検証したprocess identityとrollout terminal stateの両方を用いる。
- live rolloutでUTF-8、JSON、event kind、task stateを検証できないrecordがあるcycleは拒否し、最後の完全snapshotを保持する。
- RESTはread-onlyである。未知route/method、不正header/schema、oversize requestからDB、settings、cursor、processを変更しない。
- GUIなしserverはwindow、Slint component、display backendを生成せず、明示したservice lifecycleで起動・停止・復旧する。

## 4. データ保護

- 履歴DB、verified backup、設定、Linux側履歴は、install、update、rollback、uninstall、restore失敗で削除しない。
- migration、restore、updateはcandidateを完全検証してからatomic switchする。検証またはswitch失敗時は旧世代だけをcurrentとして保持する。
- cursorはsource identityと結合し、rotate、truncate、replaceを区別する。古いoffsetによるskipと二重登録を防ぐ。
- backup作成または検証に失敗した場合は既存のverified世代をpruneしない。
- crash、reboot、再実行はjournalの同一operationを再開し、commit、publication、deleteを各1回以下にする。

## 5. Windows導入・更新・削除

- アプリ起動後の更新確認は通知だけを生成し、download、Setup起動、既存payload変更を行わない。
  新版がある場合だけ状態帯に更新操作を表示し、利用者がその操作を明示実行した後に限ってdownloadと
  標準GUI Setupを開始する。常設の更新ボタン、silent install、unattended apply、自動再起動を行わない。
- 更新候補は公開済み・非prereleaseの`windows-vX.Y.Z` Releaseだけから選び、同Releaseのexact-name
  installerとmanifestについてversion、URL authority、byte size、SHA-256を完全検証する。不一致、
  redirect逸脱、途中download、oversize、起動失敗は既存payloadを変更せず、部分fileを公開しない。
- install、update、rollback、uninstallは別transactionとして扱い、stage中のfile、shortcut、HKCU、Apps登録を成功状態として公開しない。
- update失敗時は旧payload、shortcut、registry、versionを起動可能な状態で保持する。初回install失敗時は未公開状態へ戻す。
- uninstallは設定と履歴を保持する。途中失敗は完全復元または再開可能なjournal状態のどちらかにし、部分削除を成功と表示しない。
- 同一install rootの同時操作は単一leaseで直列化する。PID再利用、foreign owner、reparse差替え、token変化を検出した操作はmutation 0とする。
- interactive/silent等のmode、exit code、対応Windows、architecture、署名者、version policyはrelease authority inputで決める。
  署名者authorityが未設定なら署名済みと表明せず、設定済みauthorityと不一致なら更新候補を拒否する。
  利用者が開始するunsigned OSS buildは、exact GitHub repository、release tag、manifest、size、SHA-256の
  検証を満たす場合だけ標準GUI Setupへ渡し、Windowsが示すpublisher警告を隠さない。

## 6. 配布・顧客向け表明

- Windows製品版とX版は単一のstable `X.Y.Z`を共有する。製品変更を含むPRはmajor/minorを変更せず、mergeごとに
  自動採番処理がpatchを十進整数としてちょうど1増やす。patchからminorへ桁上がりさせず、`1.0.9`の次は
  `1.0.10`とする。major/minorは利用者の明示指示を要する別変更でだけ更新し、自動採番処理は変更しない。
  `Cargo.toml`、root packageの`Cargo.lock`、`windows-client/Directory.Build.props`が開始時点で同値でない場合、
  または期待元versionとmainが一致しない場合は、3ファイルを一つも変更せず停止する。
- `main`向けPRの変更pathが`AGENTS.md`、`README.md`、`docs/**`、`.github/ISSUE_TEMPLATE/**`だけで構成される場合を
  「非製品のみ」とする。rename/copyは変更前後のpathを両方判定する。これ以外を1件でも含むmixed PR、未知path、
  workflow・CI script・製品sourceの変更は製品変更として完全な採番・品質・配布経路を通す。PR/API identity不一致、
  changed-files件数と全page取得結果の不一致、重複・空・malformed data、分類器未対応時は非製品扱いへfallbackせず、
  mutation前に失敗する。ただしtrusted baseに分類器がまだ存在しない導入時の1境界だけは製品変更として完全経路を通す。
- 製品変更PRの作成前は単一のローカル入口からnative deterministic test、data protection契約、Windows unit/contract testを
  各1回だけ実行する。必須Rust testは1回の全target実行結果から名前と成功を確認し、個別に再実行しない。
  PRではrelease artifactに必要なnative buildとCLI/recorder実行、mainの実適用merge rule、実Windows installer/UIを
  各1回だけ確認し、ローカルと同じtest/gateを別job、別workflow、acceptance、Releaseで再実行しない。非製品のみPRは
  これらの製品build/test/artifact producerをすべてskipし、分類とrequired check集約だけを短時間で実行する。
  単一のrequired acceptanceは常に生成する。製品変更ではversion準備、先行jobの成功、検証対象tree、source SHA、
  各artifact証拠の対応を検査し、非製品のみではversion未変更、全artifact producerが`skipped`、artifact 0件を検査する。
  分類欠落、期待外のjob結果、失敗・未完了・古い証拠では明示的に失敗してmergeを許可しない。
- 製品変更PRだけ、品質確認を開始する前にPR branch上のversion 3ファイルをexact next patchへ自動更新する。その
  version commitを含む最新mainとの合成treeを品質確認し、mergeによって採番を確定する。採番commit自身では
  高コスト品質jobを開始せず、新しいheadに対する次のworkflow runで一度だけ確認する。非製品のみPRはversion 3ファイルを
  変更せず、`version-prepared` required jobを分類だけで成功完了させる。
- 製品変更のmerge後だけ、eventとPR APIの全identity（PR番号、head/base repository・ref・SHA、merge SHA）および
  `merged_at`の一致を確認する。ready_for_review等で同じheadの正当な品質runが複数存在し得ることを前提に、quality run APIを
  statusで絞らず全status・全page取得し、同一identityのexact候補がすべて`created_at <= updated_at <= merged_at`を
  満たすことを検証する。そのうち`run_number`が最大で一意のcandidateだけを選び、`completed`/`success`のときだけ
  受理する。最新candidateの失敗・未完了、tie、候補zero、malformed、post-merge run、pagination不完全・異常は
  artifact取得前にfail-closedとし、older successへfallbackしない。`pull_requests`はauthorityにせず、検証済みtreeとの
  一致を確認し、quality test/buildを再実行せずmanifest生成とRelease公開を行う。同時mergeと公開は直列化する。
  非製品のみのmerge後jobは分類だけで成功完了し、quality run解決、artifact download、binary build、manifest、tag、
  GitHub Releaseを一切生成しない。
- PR由来のcheckout、script、workflow、artifactを、repository contents・checks・Releaseへのwrite権限を持つjobで実行しない。
  変更分類はPR/file APIの完全なidentityとpaginationをdefault branchまたはPRのtrusted baseにある分類器で判定する。
  自動採番はdefault branchのtrusted workflow/toolだけを実行し、PRのversion 3ファイルをdataとして検証した後、
  same-repository headへexact 1 commitを原子的に追加する。head/baseの競合、fork、不正version、対象外file mutationでは
  書き込まない。post-merge jobはdefault branchのmainだけをcheckoutし、分類不成立またはeventのmerge SHAと不一致の場合は
  Release mutationを行わない。
- 製品変更PRではCodeQLをmerge必須gateとし、critical/high findingをdismissやworkflow無効化で通過させない。
  非製品のみPRではCodeQL AnalyzeとAutobuildを実行せず、active code-scanning rulesetの設定は維持する。外部AI findingsが
  provider側の未対応modelで継続失敗する場合は、そのAI機能だけをrepository単位で無効化できるが、製品変更PRのCodeQL、
  code-scanning alerts、required acceptanceは維持する。
- Codex code reviewはPRの変更が確定した最新headに対して`@codex review`を1回だけ起動する補助レビューとする。
  古いheadの結果や未解決かつnon-outdatedのP0/P1をready判定へ流用せず、独自API key workflowを追加しない。
  Codex reviewはCodeQL、required acceptance、必要な承認の代替にしない。
- `windows-vX.Y.Z` tagは原子的に新規作成する。Setupとmanifestは非公開Draft上でexact 2資産の名前・size・
  状態・commit SHAを検証し終えてから公開し、既存tag/Releaseへ上書きして継続しない。
- release artifactはsource、lockfile、実payload、license/notice、署名、version、対象platformを一つのrelease identityで追跡する。
- publisher名、certificate、対応OS build、RPO/RTO、accessibility適合、support窓口を根拠なしに推測しない。
- authority inputがないclaimは「保証なし」「未対応」とし、認証済み、対応済み、測定済みと表示しない。
- recovery journalと顧客共有support bundleを分離する。support exportは明示操作、allowlist、owner-only ACL、秘密0を必須とし、自動送信しない。
- customer guideとdeveloper READMEを分離し、顧客手順にrepository clone、Cargo build、`run.sh`を通常導線として要求しない。

## 7. 有限の検証規則

- 要件は重複を作らず、同じ観測結果は同じ要件へ統合する。
- 各境界軸の値を最低1回検査する。複数軸は、共有状態または既知の因果関係がある組だけを有限のrisk-based caseにする。
- 全直積、N倍、N二乗、N階乗のcase生成を行わない。
- 文書ごとのSHA一致を合否条件にしない。合否は観測結果、失敗時保持、副作用数、参照整合で決める。
- 製品artifactのhashは評価対象を一意に識別するために1つ記録できるが、内容評価の代用にしない。

## 8. 完了条件

- 上記要件と参照先仕様の間に、同じ入力へ異なる必須結果を要求する矛盾がない。

## グラフ表示の正本と受入境界

グラフの値の意味、期間境界、欠測・未使用区間、残量とモデル使用量の
独立性は `docs/WINDOWS_UX_SPEC.md` のグラフ意味論を正本とする。X版と
Windows版の実装は同じ `tests/fixtures/graph_delayed_quota.json` を入力に
し、fixture内の固定期待値（期間数、累積SOL、遅延して届く残量、未観測区間）
をそれぞれ独立に検証する。片方の描画結果や補間ヘルパーをもう片方の
期待値生成に使うことは禁止する。仕様・fixture・実装のいずれかが不一致、
または実機証跡が欠ける場合は合格ではなく保留とする。
- 自動検査は実行されたtestが0件でないことを確認する。UI変更は実画面、Windows固有動作は実Windowsで確認する。
- 未確認の外部authority値を創作してPASSにしない。値がない場合のfail-closed動作を検証する。

## 「正しい表示」の判定（必須不変条件）

次の条件をすべて満たした状態だけを正しい表示と定義する。

1. 同一の認証主体でquotaを定期更新している間は、前回の完全なモデル使用量・履歴・threadを保持する。更新途中の欠測を空、0、未取得へ置き換えない。
2. `reset_at`がサービスのrolling値として移動しても同一期間を維持する。実際の期間切替は、quotaの回復または期間境界を示す観測がある場合だけ新期間へ移す。
3. モデル使用量（ドル・token）と残量（%）は別観測として扱う。残量観測がない時間帯をモデル使用量から逆算せず、遅れて届いた残量観測はその時刻へ反映する。
4. 同一観測時刻に異なる`reset_at`の行が存在する場合、100%かつ使用量0の単独行だけをrolling更新の候補とし、それ以外の残量を選択中期間へ混在させない。行順で値を上書きせず、期間境界も同一時刻の単独行だけで短縮しない。
5. 過去期間はその期間に属する全モデル系列を累積値で描画し、未使用区間は専用の未使用帯として表示する。SOL/TERRA/LUNAのいずれかを0や欠測へ黙って変換しない。
6. 明示的なログアウトまたは認証主体変更だけが可視状態を消去する。通信失敗・quota更新中・local収集中は最後の完全表示を保持し、失敗状態は別途表示する。
7. X版とWindows版は同一fixtureの固定oracle（期間数、期間境界、累積SOL、遅延残量、未観測区間）を独立に満たす。どれか一つでも不一致、実機証跡欠落、またはテスト未実行なら合格ではなく保留とする。
8. 製品バージョンはメイン画面に一度だけ表示し、子ウインドウのタイトルやボタンへ重複表示しない。値はX版・Windows版とも同じリリースversion authorityから導出する。
9. Windows版の初回起動では、health・status・detailsの最初の完全な世代が揃うまで内容領域を表示せず、固定レイアウト上にスピナーを表示する。途中の部分世代を順番に描画して画面をばたつかせてはならない。初回取得が失敗した場合はスピナーを解除し、失敗状態と再試行手段を表示する。
10. X版の初回起動でも、認証済み状態へ遷移した後のquota・local usage・threadの最初の完全な世代が揃うまで主画面の内容領域を公開せず、ヘッダー（製品バージョンを含む）を固定したままスピナーを表示する。local収集またはapp-serverが失敗した場合はスピナーを解除し、最後の完全表示または失敗状態を表示する。
11. X版の起動ウィンドウは主モニターの可視デスクトップ内へ配置し、別モニターや負座標へ出して利用者から見えない状態にしてはならない。起動成功は、可視範囲内の実ウィンドウと内容の実画面で確認する。
12. `--ui` のdaemon/REST起動に失敗しても、X版のGUIを消失・即時終了させず、接続失敗と再試行手段を表示する。

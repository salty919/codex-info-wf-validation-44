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

- Windows製品版は単一の`X.Y.Z`を正本とする。PRでは現行`main`より上がる変更だけをrelease候補として
  検証し、PRからReleaseを書き換えない。`main`へのmergeで同版が上がり、全Windows gateがPASSした時だけ
  `windows-vX.Y.Z` tag、Setup、update manifestを同じGitHub Releaseへ公開する。版が不変または後退なら
  Releaseを作らない。HTTP 404だけを不存在と認め、tagは原子的に新規作成する。Setupとmanifestは
  非公開Draft上でexact 2資産の名前・size・状態・commit SHAを検証し終えてから公開し、既存tag/Release、
  通信障害、5xx、部分uploadへ上書きして継続しない。
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
4. 過去期間はその期間に属する全モデル系列を累積値で描画し、未使用区間は専用の未使用帯として表示する。SOL/TERRA/LUNAのいずれかを0や欠測へ黙って変換しない。
5. 明示的なログアウトまたは認証主体変更だけが可視状態を消去する。通信失敗・quota更新中・local収集中は最後の完全表示を保持し、失敗状態は別途表示する。
6. X版とWindows版は同一fixtureの固定oracle（期間数、期間境界、累積SOL、遅延残量、未観測区間）を独立に満たす。どれか一つでも不一致、実機証跡欠落、またはテスト未実行なら合格ではなく保留とする。

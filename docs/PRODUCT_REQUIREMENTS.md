# Codex Info 製品要件

この文書は、WindowsクライアントとLinux側collector/APIの実装判断に必要な要件だけをまとめた正本である。
監査履歴、作業経過、文書SHA一覧は製品要件ではないため含めない。詳細なwire schemaは
`REST_API_V1.md`、データ保持は`DATA_PROTECTION_POLICY.md`、画面仕様は`WINDOWS_UX_SPEC.md`を参照する。

## 1. 製品境界

- WindowsクライアントはLinux側の利用状況、残量、履歴、実行中threadを読み取り表示する。
- 製品の通信は固定loopback API、利用者が選択したWSL、またはOpenSSH configのliteral `Host` aliasに限定する。
- password、token、private key、展開済みSSH接続情報を保存しない。保存できるのは再接続に必要な非秘密selectorだけである。
- telemetryは送信しない。未登録の外向き通信、暗黙のsupport upload、raw diagnostic uploadを行わない。

## 2. 状態と表示

- Setup、未認証、通常、警告、取得失敗を区別する。取得失敗を未認証や正常emptyへ変換しない。
- 更新candidateが不完全、不正、世代不一致の場合は画面の一部だけを更新せず、最後の完全な表示を保持する。
- 同じ事実の表示所有者は1か所とし、残量、reset countdown、status、connection情報を言い換えて重複表示しない。
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

- install、update、rollback、uninstallは別transactionとして扱い、stage中のfile、shortcut、HKCU、Apps登録を成功状態として公開しない。
- update失敗時は旧payload、shortcut、registry、versionを起動可能な状態で保持する。初回install失敗時は未公開状態へ戻す。
- uninstallは設定と履歴を保持する。途中失敗は完全復元または再開可能なjournal状態のどちらかにし、部分削除を成功と表示しない。
- 同一install rootの同時操作は単一leaseで直列化する。PID再利用、foreign owner、reparse差替え、token変化を検出した操作はmutation 0とする。
- interactive/silent等のmode、exit code、対応Windows、architecture、署名者、version policyはrelease authority inputで決める。未指定・未署名・不一致なら新規公開、更新、対応表明を行わず旧verified版を保持する。

## 6. 配布・顧客向け表明

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
- 自動検査は実行されたtestが0件でないことを確認する。UI変更は実画面、Windows固有動作は実Windowsで確認する。
- 未確認の外部authority値を創作してPASSにしない。値がない場合のfail-closed動作を検証する。

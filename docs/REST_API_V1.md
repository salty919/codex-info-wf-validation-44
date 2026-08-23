<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# イントラネット REST API v1

## 目的と境界

Linux / WSL 上で起動する既存の Codex Info に、Windows クライアント向けの読み取り専用 API を追加する。
`codex-info-server`はSlint/X11/Wayland runtime dependencyを持たないheadless release binaryである。
`RecorderDaemon`はその`record --interval 60` modeとしてsource JSONLを検証してSQLiteへ書く独立writerであり、HTTP listenerを持たない。
`SnapshotPublisher`がcommit済みの完全な`DataGeneration/DataHash`と現行 admission tuple
`(ProfileScopeId, AccountScopeId, StorageEpoch, SupervisorLeaseIdentity, CollectorEpoch, CycleSeq)`からimmutableなstatus/details
`PublishedPair`を構築し、native UIとREST workerへ同じpairをread-onlyで渡す。HTTP要求からCodex
app-server、認証 URL、セッションファイル、SQLite、Slint / X11へ直接到達する経路は持たない。

この v1 はインターネット公開、LAN への直接公開、ブラウザー向け CORS、書込み
操作、ログイン操作を対象外とする。既存の Linux / X11 UI は引き続きローカルで
動作する。

Windows clientの接続設定はREST resourceではなく、`language`、`setupCompleted`、
`connectionConfigured`、`timeZoneId`、`connectionProfile`、`connectionSelector`の6 keyだけを
ローカルに保存する。profileは`none|wsl|sshConfigAlias`、selectorはWSL exact distribution tokenまたは
literal OpenSSH Host alias grammarに限定し、secret、展開済み値、raw host/user/pathを保存・送信しない。
saved selectorのauto reconnectは`ArgumentList`＋`BatchMode=yes`で起動し、auth argvもsaved profileから作るが、
起動成功とstatus再確認は別stateとする。4-key recoveryはMain disconnected＋Settingsだけであり、製品判定は`PRODUCT_PENDING`。

| 要望 | v1での対応 |
| --- | --- |
| Linux / WSL をサーバー化する | headless `codex-info-server serve --listen 127.0.0.1:8787`として実装する。Slint/X11/Waylandへ依存しない。 |
| Windowsから監視する | SSHローカルポート転送先の固定JSONを、`windows-client/` の Windows 監視クライアントが表示する。 |
| Linuxネイティブ環境を残す | APIは未設定なら起動せず、既存のUI更新経路を変更しない。 |
| イントラネットだけを対象にする | loopbackだけへ束縛し、SSHを暗号化・認証境界にする。 |
| インターネット経由は別設定にする | v1の設定や認証を再利用せず、今回の対象外として分離する。 |

## 起動と SSH トンネル

API は既定で無効である。user-systemd unitsがinstalledなprofileでは
`codex-info-api.service`の`codex-info-server serve --listen 127.0.0.1:8787`だけが開始する。
fallbackでは同じserve commandを明示的に実行した場合だけ開始する。`0.0.0.0`、`::`、LAN アドレス、
ホスト名は受け付けない。

```bash
codex-info-server serve --listen 127.0.0.1:8787
```

`serve`は REST 専用のサイレント起動となり、Linux のネイティブUIは表示しない。UIを使う場合は
native UIを別プロセスで起動する。`codex-info-recorder.service`は別に
`codex-info-server record --interval 60`を実行し、API/REST/UIはrecorderを暗黙spawnしない。
`codex-info-server.target`はこのrecorder serviceとAPI serviceを束ねる。
installer、update、rollback、uninstallでのunit/binary保持順序はrunbook-ownedであり、runbook証拠がない間は`PRODUCT_PENDING`とする。

REST専用workerは`SnapshotPublisher`のread-only consumerであり、recorder leaseやmaintenance
ownerを取得しない。units installed時は`codex-info-recorder.service`とsystemdがrecorderを所有し、
UI/REST/`run.sh`はrecorderをspawnしない。fallbackでは明示的なrecord commandだけがleaseを取得し、
後続起動は既存lease発見時にno-opとなる。UI/RESTの終了はrecorderを停止しない。daemonが単独で動く間に
HTTP listenerを暗黙生成せず、REST listenerとrecorderのlifecycleを分離する。

Windows からは SSH のローカルポート転送を使う。

```powershell
ssh -N -o BatchMode=yes -L 8787:127.0.0.1:8787 <connectionSelector>
```

上記は接続関係を示す概念的なshell例であり、Windowsクライアントの実行argv順序を定義しない。
Windowsのcanonical `ArgumentList`はValue AuthoritiesおよびWIN-E-006..010の
`[ssh.exe,-o,BatchMode=yes,-N,-L,8787:127.0.0.1:8787,<validated alias>]`に従う。

このとき Windows クライアントの接続先は
`http://127.0.0.1:8787/v1/status` と `/v1/details` である。HTTP は Linux 側の loopback と
SSH トンネルの端点の間だけで使用し、端末間の暗号化・相手認証は SSH が担当する。
そのため v1 では HTTPS 証明書を扱わない。

## リソース

すべての応答は `Content-Type: application/json; charset=utf-8` と
`Cache-Control: no-store` を返し、response header aggregateは8 KiB以下とする。既知の固定bodyでは
`Content-Length`をUTF-8 body bytesと一致させ、未知長の受信側もstream中の上限超過で停止する。
`Set-Cookie`、`Location`、`Content-Encoding`、`WWW-Authenticate`、proxy/authentication headerは
返さない。response header allowlistはこの`Content-Type`/`Cache-Control`と固定body時の`Content-Length`だけで、
その他のapplication/proxy headerを追加しない。許可する成功メソッドは `GET` だけである。自動解凍、redirect、cookie、proxyは使用しない。

| Request | Result |
| --- | --- |
| `GET /v1/health` | API プロセスが応答可能であることを示す。 |
| `GET /v1/status` | 最新の安全な状態・利用枠スナップショットを返す。 |
| `GET /v1/details` | モデル別ドル内訳、履歴期間・サンプル、実行中Threadsを返す。 |

未定義のパスは JSON の `404`、既知パスへの非 `GET` は JSON の `405` で返す。
応答に email、認証 URL、認証トークン、raw error、ローカルパス、セッション内容を
含めない。

path照合はcase-sensitiveかつ完全一致であり、URL decode/normalizationを行わない。case-altered path、
末尾slash追加、query付きknown path、未定義prefixはunknown pathとして扱う。拒否bodyは`api_version="v1"`と固定error codeだけを持つ
JSON objectとし、未知キー・raw error・秘密値を含めない。404/405を含む全responseは
SQLite transaction、WAL/SHM、migration、prune、backup、DB row/hash、PublishedPairを変更しない。

### Response・read-only matrix

| request | method | result | response / side effect |
| --- | --- | --- | --- |
| `/v1/health` | `GET` | `200` | JSON health object、required `Content-Type`/`Cache-Control` headers、DB write/transaction=0 |
| `/v1/status` | `GET` | `200` | current `PublishedPair` status、同上header、DB write/transaction=0 |
| `/v1/details` | `GET` | `200` | current `PublishedPair` details、同上header、DB write/transaction=0 |
| 上記known path | `HEAD/POST/PUT/PATCH/DELETE/OPTIONS`等全non-GET | `405` | 固定JSON error、同上header、DB/WAL/SHM/migration/prune/backup=0 |
| unknown、case-altered、末尾slash、query付きpath（methodを問わない） | any | `404` | 固定JSON error、同上header、DB/WAL/SHM/migration/prune/backup=0 |

RESTのtransfer body上限はstatus `64 KiB`、details `32 MiB`、response header `8 KiB`である。
detailsの配列上限はhistory periods `128`、history samples `100,000`、confirmed history gaps `4,096`、threads `256`、models `3`。
これらはinternal validated snapshot `1 MiB`とは別resourceであり、どの上限もdecode後の推測値へ
置換しない。上限超過、malformed、unknown/case key、duplicate key、domain errorは該当resourceの
直前完全pairを保持し、部分候補を公開しない。現行 admission tupleのいずれかがstale・欠落・不一致の
candidateも同じ扱いとし、DB、memory、REST、UIを変更しない。

`GET /v1/status` の形は次のとおりである。`quota` は利用枠がまだ確定していない
場合に `null` になる。`input_tokens` はキャッシュ分を除いた入力トークンである。

```json
{
  "api_version": "v1",
  "state": "ready",
  "observed_at": 1780000000,
  "authenticated": true,
  "plan_label": "Pro",
  "quota": {
    "remaining_percent": 98.0,
    "reset_at": 1780400000,
    "window_seconds": 604800,
    "monthly": false
  },
  "models": [
    {
      "name": "SOL",
      "input_tokens": 1200,
      "cached_input_tokens": 300,
      "output_tokens": 400
    }
  ],
  "active_thread_count": 1
}
```

`state` は `initializing`、`ready`、`auth_required`、`error` のいずれかである。
`error` は接続先ではなく Codex 情報取得の失敗を表す。詳細な失敗内容は API に
公開しない。Windows クライアントは HTTP 接続エラーと `state` を別々に表示する。

wireに `ready` boolean keyは存在しない。接続後の利用可能判定は、完全schemaを受理した同じ
status/details rootについて `state == "ready" && authenticated == true` の論理積だけである。
`state="ready",authenticated=false` と `state="auth_required",authenticated=true` はdomain不整合として
candidate pair全体をrejectし、直前の完全pairを保持する。`/v1/health`の200、process生存、listener存在、
認証開始processのexit codeだけをreadyへ読み替えない。fixture、Windows state、文書でも架空の
`ready=true` fieldを作らず、必ず上記2実在fieldを別々に記録する。

### `/v1/details` 完全schema

`/v1/details` はcontract revision `rest-v1-details-gap-20260823`の次のトップレベル13キーだけを持つ。先頭7項目と
`active_thread_count`は同じ`PublishedPair`からpublishされた`/v1/status`の同名値と一致し、
`models`はstatusのtoken 4項目へdollar 3項目を加えた形である。片側の取得・検証が失敗した
場合はstatusだけを先行更新せず、直前の完全pairを保持する。wire上に`version`、
`snapshot_epoch`、`billing_period`、`thread.status`、`is_orphan`は存在しない。

| key | 型・境界 |
| --- | --- |
| `api_version` | 文字列`v1` |
| `state` | `initializing` / `ready` / `auth_required` / `error` |
| `observed_at` | `null`またはUnix秒整数`1..253402300799` |
| `authenticated` | boolean |
| `plan_label` | `null`または本節「PlanTypeから公開値への写像」のexact canonical label |
| `quota` | `null`またはstatusと同じ4必須キーのobject |
| `models` | 0..3件、`SOL`/`TERRA`/`LUNA`重複なし。各行は下記7必須キー |
| `active_thread_count` | JSON非負整数`0..UInt64.MaxValue` |
| `history_periods` | 0..128件。各行は下記5必須キー |
| `history_samples` | 0..100,000件。各行は下記9必須キー |
| `history_gaps` | 0..4,096件。`recorder_gap_ledger`のconfirmed rowだけを下記5必須キーへredactしたprojection |
| `threads` | 0..256件。各行は下記12必須キー |
| `estimated_cost_label` | control/bidi formattingなしの1..160 Unicode scalar。表示所有権は別途DESIGNで決め、schemaに存在するだけで重複表示を許可しない |

各model行は`name`、`input_tokens`、`cached_input_tokens`、`output_tokens`、
`input_dollars`、`cached_input_dollars`、`output_dollars`だけを持つ。tokenはJSON非負整数、
dollarは有限かつ0以上のJSON numberである。ドルはcreditや為替へ変換しない。

各history period行は`id`、`start_at`、`end_at`、`label`、`current`だけを持つ。`id`は
1..512 Unicode scalarで集合内一意、時刻はUnix秒整数`1..253402300799`、
`end_at >= start_at`、`label`はcontrol/bidi formattingなしの1..512 Unicode scalar、
`current=true`は集合内で最大1件である。

`label`はLinux/X側が同じperiod groupの表示に用いたreference labelであり、selection keyでもWindowsの
日時parse入力でもない。serverは`DESIGN.md`のcanonical period ID、start/end、起動時timezone、DST offset、
重複期限suffix、current suffixから一意に生成し、`label -> id`対応が1対1でない候補をpublishしない。
Windowsは選択・保持を`id`と受理array indexだけで行い、文字列をparseしない。Windows画面のperiod labelは、
同じ`start_at/end_at/current/id`を保存済み`timeZoneId`、locale、登録済みsuffix mappingで再renderする
presentation ownerとし、wire labelと異なる場合もcanonical ID、両端instant、offset、suffix roleの全てが
一対一に一致しなければならない。server reference labelをそのまま表示する実装も、選択中locale/timezoneの
mapping結果と完全一致する場合だけ許可する。

### PlanTypeから公開値への写像

`plan_label`と`quota.monthly`は任意文字列/任意booleanではなく、同じvalidated account/quota cycleの
PlanTypeからserverが生成する。外部正本は`DESIGN.md`記載のPlanType schema hashとexact enumであり、
trim、lowercase、prefix/substring一致、schema外aliasを使わない。wireへPlanTypeやPlanFamily keyを追加せず、
次の関係だけを公開する。

| exact PlanType | canonical `plan_label` | `quota.monthly` |
| --- | --- | --- |
| `free` | `無料` | `false` |
| `go` | `Go` | `false` |
| `plus` | `Plus` | `false` |
| `pro` | `Pro` | `false` |
| `prolite` | `Pro Lite` | `false` |
| `team` | `Team` | `false` |
| `self_serve_business_prolite` / `self_serve_business_usage_based` / `business` | `Business` | `false` |
| `ent26` / `enterprise_cbp_automation` / `enterprise_cbp_usage_based` / `enterprise` | `エンタープライズ` | `true` |
| `edu` | `教育` | `false` |
| schema-valid `unknown` | `プラン未設定` | `false` |

`ready/authenticated` rootでaccount PlanTypeがある場合、`plan_label`は上表の非null値である。quotaが非nullなら
`monthly`も同じrowに一致する。空、大小文字差、schema外値、account/rate-limitのknown family不一致、
label/monthly不整合はcycle全体をrejectし、旧完全pairを保持する。Windowsは自由文字列からfamily/monthlyを
推測せず、server内部のredacted PlanType、schema hash、公開label/monthlyを同一cycle evidenceへ結合する。

各history sample行は`timestamp`、`reset_at`、`remaining_percent`、`sol_dollars`、
`terra_dollars`、`luna_dollars`、`sol_tokens`、`terra_tokens`、`luna_tokens`だけを持つ。
`timestamp`は有効なUTC event秒を`floor(event_epoch / 60) * 60`へ変換したminute-startであり、
`(reset_at,timestamp)`は集合内一意、時刻は上記Unix秒範囲、`remaining_percent`は`null`
または有限な0..100、dollarは有限かつ0以上、tokenはJSON非負整数である。

各history gap行は`gap_id`、`reset_at`、`start_at`、`end_at`、`reason`だけを持つ。`gap_id`は
ASCII lowercase hex 32文字で集合内一意、3時刻はUnix秒整数で同じhistory period内、
`start_at<=end_at`とする。reasonは`daemon_stop_unrecoverable`、`reset_hint_expired`、
`auth_epoch_tombstoned`だけである。配列は`(reset_at,start_at,end_at,gap_id)`昇順、区間の重複・交差0。
pending/recovered/rejected ledger、raw cursor/path/process/ownerはwireへ出さない。server/client/release manifestの
details contract revisionが一致しない場合はpair全体をrejectし、旧完全pairを保持する。

各thread行は`id`、`title`、`parent_thread_id`、`model`、`model_label`、`total_tokens`、
`context_usage_tokens`、`context_window_tokens`、`created_at`、`last_user_message_at`、
`is_subagent`、`depth`だけを持つ。`id`は集合内一意の1..512 Unicode scalar、titleは
1..512、modelは1..128、model_labelは1..24 Unicode scalarで、全てcontrol/bidi
formattingを含まない。parentは`null`または1..512のID、3つのtokenは`null`またはJSON
非負整数、2つの時刻は`null`または上記Unix秒、`is_subagent`はboolean、depthは`null`
または整数0..1024である。Windowsのorphan表示は、完全に受理した同一threads集合に
`parent_thread_id`が存在しない場合だけ派生し、API fieldとして受け取らない。

`threads`配列はserver側canonical active snapshotの`updatedAt desc, id desc`順でpublishする。
`updatedAt`自体はwire fieldへ追加しない。Windowsは受理した配列indexをcanonical rankとして使い、
rootとsiblingの相対rankを保ったまま親先行depth-first・subtree-contiguousへpresentation投影する。
存在しない`updatedAt`をclientで推測したり、title・受信時刻・IDだけで別順へ再sortしたりしない。

全objectは上記キーを全て必須とし、未知、大小文字違い、同一object内の重複、型違い、
配列上限超過が1件でもあればcandidate全体を拒否する。status/detailsはサーバー側で一つの
write lockにより同じ`PublishedPair`からatomic publishする。Windowsが別々のHTTP requestで
同一cycleの表示を更新する場合は、現行`(ProfileScopeId, AccountScopeId, StorageEpoch, SupervisorLeaseIdentity, CollectorEpoch, CycleSeq)`、
共通core field、`DataGeneration`、canonical fingerprint、`RootHash`が一致した組だけをcommitする。
片側失敗、更新競合、一致しない組、stale lease/epoch/cycleは架空の世代番号を補わず、DB、memory、REST、UIを
変更せず両方のlast-good pairを保持して次cycleで再取得する。statusだけS1へ進みdetails D0を
残す混合世代、detailsだけを先行更新する混合世代、片側のDB read/writeは許可しない。

## 段階的な移行

v1 は Linux ネイティブ UI と別プロセスの headless `codex-info-server serve --listen 127.0.0.1:8787` で起動する監視 API である。
Windows 側には `windows-client/` の Avalonia / .NET 10 クライアントを用意し、Visual
Studio Community から solution を開いて、この固定 JSON 契約を表示できる。詳細な
接続・検証・表示仕様は[Windows クライアント](WINDOWS_CLIENT.md)を参照する。

インターネット経由の利用を将来追加する場合は、v1 の bind 設定を緩めず、別の
設定・認証・脅威モデルとして設計する。

## DATA/REST 横断 OPEN 境界（RC-139..149 / DP-REST-001..011）

現行conflict集合はRC-001..174である。この節が直接所有するのはRC-139..149であり、RC-001..138と
RC-150..171は参照入力として扱う。
RC-150..171の意味を再登録しない。RC-139..149を、RESTとデータ保護境界の同じ状態・入力・保持・証拠へ1:1で結ぶための
未決authority conflictを原子化する。全項目の状態は
`OPEN_AUTHORITY_CONFLICT`であり、具体的な数値、HTTP status、schema key、generation順、
許可side effect、負荷profileを推測して確定しない。`docs/DATA_PROTECTION_POLICY.md`にも同じIDと
用語を記載し、IDの意味を文書ごとに分岐させない。

本書はDP-REST-001..011のwire path/method/status/header/body/request limitを所有し、
`docs/DATA_PROTECTION_POLICY.md`はdata state、last-good、retention、writer/maintenance identityを所有する。
同じ事実を両書で別値に決めず、cross-document joinが不一致なら`OPEN_AUTHORITY_CONFLICT`を保持する。

対応は`RC-139↔DP-REST-001`、`RC-140↔DP-REST-002`、`RC-141↔DP-REST-003`、
`RC-142↔DP-REST-004`、`RC-143↔DP-REST-005`、`RC-144↔DP-REST-006`、
`RC-145↔DP-REST-007`、`RC-146↔DP-REST-008`、`RC-147↔DP-REST-009`、
`RC-148↔DP-REST-010`、`RC-149↔DP-REST-011`に固定する。既存RC overlap欄は
`RC-064..080`、`RC-095..097`、`RC-106..107`、`RC-129`を監査した結果であり、
隣接するだけで意味が同一でないRCは重複扱いにしない。

共通語は次のとおりとする。

- `DR-AdmissionTuple`は既存の`(ProfileScopeId, AccountScopeId, StorageEpoch, SupervisorLeaseIdentity, CollectorEpoch, CycleSeq)`である。
- `DR-LastGoodPair`は、同じ現行admissionで検証済みのstatus/details `PublishedPair`である。wireへ新しい世代fieldを追加する意味ではない。
- `DR-SourceCheckpoint`はsource fingerprint、file identity、cursor、AuthEpoch/nonce、DB commitとの関係を表す未決境界である。
- `DR-GenerationNamespace`は`DataGeneration`、backup generation、`CollectorEpoch`、service/bootstrap generation、`CycleSeq`を混同しないためのnamespace・parent・順序規則である。
- `DR-DataRestLineage`はsource→DB transaction→publisher pair→HTTP response→Windows表示を同一操作として再結合する証拠identityである。encodingと全fieldは未決である。
- `DR-ReadOnlyEffectSet`はRESTがSQLite以外へ及ぼし得るlog、metric、cache、temp、filesystem、process、network副作用の許可・禁止集合である。

### RC-139 / DP-REST-001 — health body schema / limit

- **根拠**: `docs/REST_API_V1.md:80-91,93-108,138-147`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:22,27-33,83`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:74,79`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:13`。
- **状態/入力**: `HealthCandidate→HealthValid/HealthRejected`を固定し、exact path/method、listener owner、body key/type/size、healthが認証・ready・snapshot更新を意味するかを決定する。値は後段「DP-REST wire authority」のhealth定義へ移管済みである。
- **失敗/保持/idempotence**: malformed、oversize、owner不一致、non-JSONはhealth成功を0にし、health表示・connection stateと`DR-LastGoodPair`の保持関係を決める。status/detailsをhealth失敗で更新せず、同一candidateの再入はpublish/writeを増やさない。
- **identity/oracle/targets**: `DR-AdmissionTuple`、health schema generation、request identityを結び、canonical body/status/header/byte counter、listener owner、DB副作用0を再計算する。対象=`WIN-E-012`, `WIN-I-001`, `WIN-I-006..007`, `WIN-J-001`, `WIN-K-001`, `GLOBAL:AUD-011`。RC overlap=`RC-076,079`（共通limit/headerの重複監査対象）。採用値は後段「DP-REST wire authority」が所有する。

### RC-140 / DP-REST-002 — server non-2xx / error envelope

- **根拠**: `docs/REST_API_V1.md:78-108,138-140`、`docs/DATA_PROTECTION_POLICY.md:169-176`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:20-23`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:87`。
- **状態/入力**: publisher欠落、DB read fault、serialization fault、stale owner、内部timeoutを、`PublisherAvailable→Success`または`PublisherFault→ErrorResponse/Retained`へ分類する。canonical non-2xx/未commit切断の採用値は後段Server error responseへ移管済みである。
- **失敗/保持/idempotence**: error responseのschema/header/raw body・statusを固定し、raw error/秘密を出さず、失敗時は`DR-LastGoodPair`とDB rootを保持する。error再入でretry loop、DB再生成、二重publishを起こさない。
- **identity/oracle/targets**: request、`DR-AdmissionTuple`、publisher generationを結び、wire status/body/header、pair hash、DB trace、error classificationを再計算する。対象=`WIN-J-001`, `WIN-J-009`, `WIN-J-016`, `WIN-I-014`, `WIN-I-016`, `WIN-E-012`, `GLOBAL:AUD-011`。RC overlap=`RC-079,080,107`（route/status、DB副作用、ready predicateの重複監査対象）。error envelopeは後段Server error responseが所有する。

### RC-141 / DP-REST-003 — request line/header/body/connection boundary

- **根拠**: `docs/REST_API_V1.md:69-76,88-108`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:83,86`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:79`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:13,29`。
- **状態/入力**: `ConnectionAccepted→RequestBounded→RouteDecision→Response/Rejected`を固定し、request line、request header aggregate、GET body、同時接続、keep-alive/idle、cancel、shutdownの境界値と適用時点は後段Request resource contractへ移管済みである。
- **失敗/保持/idempotence**: bound超過・malformed・途中切断はmaterialize前に拒否し、DB、memory、pair、設定を変更しない。拒否の再入は同じrequestを二重処理せず、shutdown中は新規admissionを決められた状態へ遷移させる。
- **identity/oracle/targets**: connection/request identity、listener generation、`DR-AdmissionTuple`を結び、raw wire byte、同時connection数、timeout/keep-alive、CPU/RSS、pair/DB before-afterを採取する。対象=`WIN-I-001`, `WIN-I-006`, `WIN-J-001`, `WIN-J-016`, `WIN-K-001`, `GLOBAL:AUD-021`, `GLOBAL:DP-008`。RC overlap=`RC-076,079,080`（response/local inputとDB副作用の重複監査対象）。request ownerは後段Request resource contractである。

### RC-142 / DP-REST-004 — read-only non-SQLite effect set

- **根拠**: `docs/REST_API_V1.md:11-14,88-91,93-108`、`DESIGN.md:130-134`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:22,32-33`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:28`。
- **状態/入力**: `ReadOnlyAdmission→EffectAudit→Response/Retained`を固定し、成功・404・405・errorごとのSQLite外副作用（access/error log、metric、cache/temp、atime、filesystem journal、process、network）を`DR-ReadOnlyEffectSet`へ分類する。allow/deny値は後段Read-only effect setへ移管済みである。
- **失敗/保持/idempotence**: allowlist外の副作用を黙って許可せず、決定されたreject/holdへ進め、`DR-LastGoodPair`とDB/rootを保持する。同一requestの再入で未登録副作用や二重publishを発生させない。
- **identity/oracle/targets**: request、listener/publisher identity、effect category、generationを結び、filesystem/process/network/SQLite/WAL/SHM/row/hashの全before-afterを再計算する。対象=`WIN-I-001`, `WIN-J-001`, `WIN-J-016`, `WIN-E-012`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-080`（DB側read-onlyとの重複監査対象）。effect ownerは後段Read-only effect setである。

### RC-143 / DP-REST-005 — DB profile/account/AuthEpoch storage identity

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:35-43,69,103,127-143`、`docs/REST_API_V1.md:11-14,215-219,235-242`、`docs/LIVE_STATE_DECISION_MATRIX.md:34-37`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:15-17`。
- **状態/入力（検出履歴）**: `StorageIdentityCandidate→PartitionAdmitted`または`IdentityConflict→Rejected/Quarantined`を固定し、canonical DB path、profile、account、AuthEpoch/nonceとusage keyのscopeが未決だった。採用値は§「DP-REST wire authority」と`DATA_PROTECTION_POLICY.md` §8.6のlogical partition、`(partition_id,reset_at,timestamp)`である。
- **失敗/保持/idempotence**: identity不一致、同一keyの異なるaccount、profile alias collisionではwrite/upsert/publishを0にし、旧DB、旧root、旧accountの非表示境界を保持する。同一storage identity・同一operationの再入は1回を超えるrow/publishを作らない。
- **identity/oracle/targets**: storage-binding generation、profile/account/AuthEpoch、lease ownerを結び、同一path・同一keyを用いたcross-profile/account fixtureでpartition、row count、column MAX/COALESCE、visible occurrencesを再計算する。対象=`WIN-J-003..005`, `WIN-J-012..013`, `WIN-J-016`, `WIN-I-016`, `GLOBAL:DP-008`, `GLOBAL:LIVE-001`。RC overlap=`RC-068,069,096,106`（writer/lease/publisher identityの重複監査対象）。partition ownerは後段DP-REST-005である。

### RC-144 / DP-REST-006 — cursor + SQLite atomic commit

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:53-59,96-105,133-146,188`、`DESIGN.md:79,123`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:16-17,85,89`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:23-24`。
- **状態/入力**: `SourceRead→Candidate→DBAndCheckpointCommit→PairCandidate→Published`または`Rollback/Hold`を固定し、source fingerprint/file identity/cursor/AuthEpoch/nonce、row batch、DB transactionのcommit関係は後段DP-REST-006へ移管済みである。
- **失敗/保持/idempotence**: DB commit後のcursor失敗、cursor先行後のDB rollback、crash、busy/full/I/Oではcursor、DB、rootを同じ旧世代へ保持し、重複・欠損・推測gapを作らない。同一source checkpointとoperationの再入はidempotentにする。
- **identity/oracle/targets**: `DR-SourceCheckpoint`、operation generation、`DR-AdmissionTuple`を結び、cursor before/after、transaction id、row/hash、restart trace、publish countを同じevidenceへ入れる。対象=`WIN-J-010..012`, `WIN-J-016`, `WIN-I-016`, `GLOBAL:DP-008`。RC overlap=`RC-065,067,068,074`（cursor/gap/writer/faultの重複監査対象）。commit ownerは後段DP-REST-006である。

### RC-145 / DP-REST-007 — generation namespace / parent / monotonicity

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:41-43,112-115,153-159,166-167`、`docs/REST_API_V1.md:11-14,235-242`、`DESIGN.md:132-133`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:19,23`。
- **状態/入力**: `GenerationCandidate→NamespaceValidated→Published`または`Stale/Regression/Collision→Retained`を固定し、`DataGeneration`、backup generation、`CollectorEpoch`、service/bootstrap generation、`CycleSeq`のnamespace、parent、順序、restore/rollback後の関係は後段DP-REST-007へ移管済みである。
- **失敗/保持/idempotence**: namespace不明、parent不一致、generation回帰・衝突・stale candidateでは新pairを公開せず`DR-LastGoodPair`を保持する。同一operation/同一candidateの再入はgeneration/publicationを二重化しない。
- **identity/oracle/targets**: `DR-GenerationNamespace`、base DB hash、operation id、`DR-AdmissionTuple`、RootHashを結び、generation ledgerとDB/backup/pair hashを時系列で再計算する。対象=`WIN-I-016`, `WIN-J-014..016`, `WIN-L-016`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-071,073,078,096,106`（各世代・pair・ownerの重複監査対象）。namespace ownerは後段DP-REST-007である。

### RC-146 / DP-REST-008 — explicit restore crash journal / re-entry

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:150-161,165-167`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:18-20,88`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:26-27`。
- **状態/入力**: 明示restoreを`RestoreIdle→AdmissionClosed→CandidateAudited→CurrentQuarantined→Replaced→PairChecked`または`RestoreFailed→Reconcile`として固定し、operation id、owner、phase、current/candidate path・inode・hash、crash/re-entryを入力にする。restore journal schemaはOPENである。
- **失敗/保持/idempotence**: 退避後・replace前・reload/pair検証中のcrashや第二restoreではwriter/publishを0にし、現DB、全verified backup、old memory/rootを復元可能に保持する。同一operationのresumeは一回だけ、foreign operationはtakeover/rewriteしない。
- **identity/oracle/targets**: MaintenanceOwner、operation generation、journal identity、DB generationを結び、各crash boundaryのjournal replay、current DB count、candidate/current hash、switch/publish count、pair検証を再計算する。対象=`WIN-J-006`, `WIN-J-014..015`, `WIN-I-016`, `WIN-L-016`, `GLOBAL:DP-008`。RC overlap=`RC-071,072,073`（backup/migrationまたはrestore順序の重複監査対象）。restore journal ownerは後段DP-REST-008である。

### RC-147 / DP-REST-009 — host reboot daemon re-entry

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:119-142`、`docs/REST_API_V1.md:35-56`、`DESIGN.md:111`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:14-16,83-86`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:19,22-23`。
- **状態/入力**: `HostBoot→RecoveryAdmissionClosed→LeaseJournalCursorRevalidated→NewCollectorEpoch→Starting→Running`または`Hold`を固定し、boot/power-loss identity、persisted lease/journal/cursor/hint、DB/root/generationを入力にする。systemd boot orderingとre-entry authorityは後段DP-REST-009へ移管済みである。
- **失敗/保持/idempotence**: stale lease、未回収journal、cursor/hint不一致、旧publisherの再入は採用せず、旧DB/root/pairを保持する。同一boot/operationの再入はactivation、lease取得、backfill、publishを二重化しない。
- **identity/oracle/targets**: boot identity、new supervisor/collector epoch、owner nonce、`DR-SourceCheckpoint`、`DR-AdmissionTuple`を結び、reboot/power-loss前後のprocess/lease/journal/cursor/DB/pair traceを再計算する。対象=`WIN-J-010`, `WIN-J-011`, `WIN-J-013`, `WIN-J-016`, `WIN-I-016`, `GLOBAL:DP-008`, `GLOBAL:LIVE-001`。RC overlap=`RC-064,066,071,073`（通常crash/ownerとmaintenance recoveryの重複監査対象）。boot recovery ownerは後段DP-REST-009である。

### RC-148 / DP-REST-010 — source→DB→pair→HTTP→Windows lineage

- **根拠**: `docs/DATA_PROTECTION_POLICY.md:7-15,110-115,133-146,156-167`、`docs/REST_API_V1.md:11-14,235-242`、`docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:20-23,89`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:89`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:26-28`。
- **状態/入力**: `SourceCandidate→DBTransaction→DataGeneration/RootHash→PublishedPair→HTTPResponse→WindowsAcceptedRoot`を一つのlineage stateとして固定し、source fingerprint/cursor、AuthEpoch、transaction id、DB generation、request id、artifact SHA、表示rootのjoin fieldsは後段DP-REST-010へ移管済みである。
- **失敗/保持/idempotence**: 任意edgeの欠落、foreign generation、時刻逆転、別artifactでは受理・表示・顧客証拠を0にし、`DR-LastGoodPair`、旧DB/root、失敗evidenceを保持する。同じlineageの再入はrow/transaction/publish/displayを二重化しない。
- **identity/oracle/targets**: `DR-DataRestLineage`、`DR-GenerationNamespace`、`DR-SourceCheckpoint`、`DR-AdmissionTuple`を全edgeへ付与し、全hash・timestamp・before/after・request/response・Windows acceptanceを一つのDAGとして再計算する。対象=`WIN-J-010..016`, `WIN-I-007`, `WIN-I-016`, `WIN-L-016`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-078,096,106`（pair atomicityとowner identityの重複監査対象）。lineage ownerは後段DP-REST-010である。

### RC-149 / DP-REST-011 — combined low-load scope

- **根拠**: `docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md:83,85-86`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:22`、`docs/DATA_PROTECTION_POLICY.md:58-59,133-135,188`、`docs/REST_API_V1.md:69-108`。
- **状態/入力**: `LoadProfileDeclared→Measured→InScope/Unsupported`を固定し、daemon-only idleか、REST polling、複数collector/server、maintenance、同時requestを含むcombined profileかを決める。supported scopeと上限は後段DP-REST-011へ移管済みである。
- **失敗/保持/idempotence**: 宣言profile外の実測を製品負荷保証へ昇格せず、超過・測定不能時はデータ/root/pairを変更しない。同じprofileの再測定は追加writer、scan、publishを発生させない。
- **identity/oracle/targets**: host、artifact SHA、各process owner、`DR-AdmissionTuple`、operation generationを結び、daemon/API/maintenance/collectorのCPU/RSS、request/connection、scan/write/retry、DB/pair hashをcombined traceで再計算する。対象=`WIN-J-010`, `WIN-J-013`, `WIN-J-016`, `WIN-I-001`, `WIN-J-001`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。RC overlap=`RC-075`（daemon finite boundsとの重複監査対象）。supported profile ownerは後段DP-REST-011である。

## DP-REST wire authority（RC-139..142の採用値）

本節は前節のREST所有OPEN値を一意化する。data state、partition、checkpoint、generation、restore、boot、
lineage、load profileは`DATA_PROTECTION_POLICY.md` §8を正本とする。本節の追加は製品実装・実機・出荷PASSを
意味せず、状態は`REQUIREMENTS_SELECTED / PRODUCT_PENDING / HOLD`である。

### Health response

`GET /v1/health`の200 bodyはUTF-8 JSON objectでexact key集合を`api_version,service`、値を
`api_version="v1"`、`service="codex-info"`へ固定する。unknown、missing、duplicate、case-altered key、
別値、control/bidi、trailing non-whitespace、depth追加を拒否する。transfer-decoded bodyは1 KiB以下である。
healthはlistener到達性だけで、認証、ready、DB健全性、PublishedPair更新を意味しない。

全JSON responseのproducer headerは次のexact意味を持つ。

- `Content-Type`は`application/json; charset=utf-8`。parameter追加、charset欠落、別charsetを生成しない。
- `Cache-Control`は`no-store`。
- fixed bodyでは`Content-Length`をUTF-8 bytesと一致させる。
- response header aggregateは8 KiB以下。`Set-Cookie`、`Location`、`Content-Encoding`、
  `WWW-Authenticate`、authentication/proxy headerは0件。

clientはContent-Typeをcase-insensitive tokenとしてparseするが、media type=`application/json`かつ
唯一のparameter charset=`utf-8`を両方要求する。charsetなしを受理しない。body key順やJSON insignificant
whitespaceはidentityに使わず、parse後のexact key/value集合とcanonical再serialization SHAをoracleにする。

### Server error response

error bodyはexact key集合`api_version,error`、`api_version="v1"`、errorは次のclosed enumだけで、
transfer-decoded bodyは1 KiB以下とする。

| HTTP status | error | 適用条件 |
| ---: | --- | --- |
| 400 | `bad_request` | parse可能だがrequest line/target/header/body契約不正 |
| 404 | `not_found` | exact known pathでない |
| 405 | `method_not_allowed` | exact known pathに対するGET以外 |
| 408 | `request_timeout` | request header/body deadline超過 |
| 413 | `request_body_not_allowed` | GETにnon-zero bodyまたはTransfer-Encoding |
| 429 | `too_many_requests` | connection admission上限超過 |
| 431 | `request_headers_too_large` | header count/field/aggregate上限超過 |
| 500 | `internal_error` | response commit前のserialization/invariant failure |
| 503 | `snapshot_unavailable` | publisher/DB root/stale owner/internal read timeout、またはshutdown中 |

route/publisher faultを200や`state=error`へ丸めない。valid PublishedPair自身の`state=error`だけはschema-valid
200 snapshotであり、server transport faultと別物である。serialization failureがheader/body commit後に起きた場合は
connectionをabortし、追加JSONやpartial-success markerを送らない。clientは全non-200、切断、長さ不一致で
status/details candidate全体をrejectし、直前pairを保持する。

### Request resource contract

製品endpointはHTTP/1.1だけを受け、request targetはorigin-formのexact
`/v1/health`、`/v1/status`、`/v1/details`である。percent decode、path normalization、query、fragment、
absolute-form、authority-form、asterisk-formを許可しない。

| resource | 採用上限・規則 |
| --- | --- |
| request line | CRLF込み2,048 bytes以下。method tokenは8 bytes以下 |
| header count | 32以下 |
| header field | name 64 bytes以下、value 1,024 bytes以下、aggregate CRLF込み8 KiB以下 |
| Host | exactly one、productでは`127.0.0.1:8787`。duplicate/欠落/別authorityは400 |
| request body | GETは0 byte。Content-Lengthは欠落またはexact 0、Transfer-Encodingは0件 |
| active connections | listener generationあたり16以下。17件目以降は429またはparse前close |
| request per connection | 1。response後closeし、pipeline/upgrade/connectは拒否 |
| deadline | acceptからheader完了3.000秒、header完了からrequest完了1.000秒、全体3.000秒 |
| shutdown | 新規admissionを即時停止し、既存requestを最大3.000秒drain後cancel |

request header allowlistは`Host,Accept,User-Agent,Connection,Content-Length`だけで、各fieldは最大1件、
ただしContent-Lengthは欠落可とする。`Authorization,Cookie,Proxy-Authorization,Forwarded,X-Forwarded-For,
X-Forwarded-Host,X-Forwarded-Proto,Upgrade,Expect,TE,Transfer-Encoding`は常に拒否する。obs-fold、NUL、CTL、
bare LF、invalid UTF-8を値として解釈せず400またはparse前closeにする。拒否requestはbodyを無制限drainせず
socketをcloseし、DB/pair/settings/checkpointを0変更とする。

### Read-only effect set

全route・全statusで許可するproduct effectはrequest lifetime内heap、bounded in-memory counter、loopback socket
read/write、read-only open/statだけである。persistent log/Event Log/metric/cache/temp、registry、child process、
非loopback socket/DNS、file create/write/rename/delete/fsync、SQLite transaction、DB/WAL/SHM、backup、migration、
checkpoint、PublishedPair mutationは禁止する。OS-managed atimeはproduct successの根拠にせず、content/inodeと
product syscall traceを検査する。同一request再入で副作用countが増えた場合はFAIL/HOLDである。

### Atomic status/details client admission

client cycleはhealth受理後にstatusとdetailsを各1回取得し、両方のschema/domain/common coreが一致した場合だけ
同じroot generationとして一括commitする。statusだけvalid、detailsだけvalid、片側timeout/non-200/invalid、
common field不一致では両方をdiscardし、直前の完全pairを保持する。wireにgeneration fieldを追加せず、client内部の
request cycle IDとcanonical common-core hashで同一候補を結ぶ。`WIN-I-016`を含む具体契約はこの規則と異なる
status-only commitを許可しない。
ただし`auth_required`のsecurity visibility transitionはdata pairのcommitではない。
schema-validな`state=auth_required,authenticated=false`を受理した場合、detailsがinvalidでも
旧accountの可視値だけを一回のroot updateで空にし、status/details store、DB、pair generationは変更しない。
これはstatus-only data commitではなく、認証世代の可視性を閉じる失敗安全遷移である。

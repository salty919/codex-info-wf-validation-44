<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# イントラネット REST API v1

## 目的と境界

Linux / WSL 上で起動する Codex Info のdaemonと、Windows クライアント向け読み取り専用 APIを、
`codex_info`の1プロセスで所有する。このdaemon modeはSlint WindowやX event loopを生成せず、
X UIを表示しない。`RecorderDaemon`はservice process内のbounded workerとしてsource JSONLを検証しSQLiteへ書く。
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
| Linux / WSL をサーバー化する | 引数なしまたは`codex_info --port 8787`でdaemon+RESTを1プロセスとして起動する。Windowは生成しない。 |
| Windowsから監視する | SSHローカルポート転送先の固定JSONを、`windows-client/` の Windows 監視クライアントが表示する。 |
| Linuxネイティブ環境を残す | 引数なし/`--port PORT`はdaemon+RESTのみ、`--ui`はdaemon+REST+X UI、`--ui --port PORT`は指定portで起動する。 |
| イントラネットだけを対象にする | loopbackだけへ束縛し、SSHを暗号化・認証境界にする。 |
| インターネット経由は別設定にする | v1の設定や認証を再利用せず、今回の対象外として分離する。 |

## 起動と SSH トンネル

user-systemdを使うprofileでは`codex-info.service`が
`codex_info --port 8787`を開始する。手動時も同じcommandを使う。待受アドレスは`127.0.0.1`に固定する。

```bash
codex_info --port 8787
codex_info
codex_info --ui
codex_info --ui --port 9876
```

引数なしまたは`--port`はWindowを表示せず、recorder lockとREST listenerを同じprocess lifetimeで所有する。
`--ui`は既存serviceを再利用し、無ければ同じloopback addressのserviceを一つだけ開始する。
引数なしはdaemon+RESTのみであり、UIを追加する場合は`--ui`を明示する。待受addressは指定できず、`--port PORT`でloopbackのportだけを変更できる。
systemd自動起動の解除は`bash scripts/install_systemd_recorder.sh --remove`で行い、DB/historyを保持する。

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
SQLiteの保持期間は過去3暦月である。一方、1回のDB取得と`details`応答が扱う履歴は観測時刻で終わる
最長1暦月の半開区間 `(one_month_before(observed_at), observed_at]` に限定する。history samples上限は
31日分の1分bucketに相当する`44,640`、history periods `128`、confirmed history gaps `4,096`、threads `256`、models `3`である。

### 応答時間SLOと容量条件

warm-up後、loopback、in-flight 1でrequest送信開始からresponse body全受信までを測る。
`/v1/health`、`/v1/status`、全4xxはP90 25 ms以下・P95 50 ms以下、`/v1/details`は
7日相当10,080 samplesでP90 50 ms以下・P95 100 ms以下、契約最大1暦月44,640 samplesで
P90 100 ms以下・P95 150 ms以下とする。各route/profileを30回以上測定し、client hard timeoutは
1秒、timeout・欠測・上限超過はPASSへ丸めない。DB読出しはtimestamp/reset複合indexを使い、
3暦月を保持したDBから1暦月窓かつ44,640行以下だけを一度materializeする。full table scan、無上限読出し、UI threadでの
行単位publishを禁止し、candidate失敗時はlast-good publicationを保持する。
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

`/v1/details` はcontract revision `rest-v1-details-reset-at-20260823`の次のトップレベル13キーだけを持つ。先頭7項目と
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
| `history_periods` | 0..128件。各行は下記6必須キー |
| `history_samples` | 0..44,640件。観測時刻までの最長1暦月。各行は下記9必須キー |
| `history_gaps` | 0..4,096件。`recorder_gap_ledger`のconfirmed rowだけを下記5必須キーへredactしたprojection |
| `threads` | 0..256件。各行は下記12必須キー |
| `estimated_cost_label` | control/bidi formattingなしの1..160 Unicode scalar。表示所有権は別途DESIGNで決め、schemaに存在するだけで重複表示を許可しない |

各model行は`name`、`input_tokens`、`cached_input_tokens`、`output_tokens`、
`input_dollars`、`cached_input_dollars`、`output_dollars`だけを持つ。tokenはJSON非負整数、
dollarは有限かつ0以上のJSON numberである。ドルはcreditや為替へ変換しない。

各history period行は`id`、`start_at`、`end_at`、`reset_at`、`label`、`current`だけを持つ。`id`は
1..512 Unicode scalarで集合内一意、時刻はUnix秒整数`1..253402300799`、
`start_at <= end_at <= reset_at`、`label`はcontrol/bidi formattingなしの1..512 Unicode scalar、
`current=true`は集合内で最大1件である。

`reset_at`はperiod groupのcanonical reset境界であり、sampleの所属判定に使う。`end_at`は現在期間では
観測時刻、途中で次期間へ切り替わった過去期間では次期間開始へclipできるため、`end_at`をcanonical
reset境界として代用してはならない。clientは`id`をparseせず、sampleの`reset_at`がperiodの
`reset_at - 60 <= sample.reset_at <= reset_at`に入るものだけを同periodへcanonicalizeする。

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

v1 は Linux ネイティブ UI と別プロセスの、引数なしまたは`codex_info --port 8787`で起動する監視 API である。
Windows 側には `windows-client/` の Avalonia / .NET 10 クライアントを用意し、Visual
Studio Community から solution を開いて、この固定 JSON 契約を表示できる。詳細な
接続・検証・表示仕様は[Windows クライアント](WINDOWS_CLIENT.md)を参照する。

インターネット経由の利用を将来追加する場合は、v1 の bind 設定を緩めず、別の
設定・認証・脅威モデルとして設計する。


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

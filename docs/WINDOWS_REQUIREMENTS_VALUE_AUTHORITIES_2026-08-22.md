# Windows要求 固定値・意味論の正本マップ（2026-08-22）

状態: `REQUIREMENTS_AUTHORITY / EXTRACTION_INCOMPLETE`

## 目的

226件の具体契約を作る際、検査用の例示値を製品仕様へ昇格したり、既存値を担当者判断で
変更したりしないための正本参照表である。各atomic contractの固定値・列挙値・座標・
閾値は、この表の正本または同表から辿れるDecisionへ一致しなければならない。

oracle専用fixtureの任意値は `fixture_only` と明記し、製品のendpoint、対応言語、版番号、
インストール先、画面絶対座標、保持世代、周期へ転用してはならない。根拠が見つからない値は
発明せず `OPEN_VALUE_AUTHORITY` として抽出FAILにする。

## Window geometry/DPI/non-scroll authority（抽出専用追加、実装現況を正本にしない）

この節の値はWindows要求の正本であり、現在の実装、既存の画像、またはfixtureの都合から
逆算して変更してはならない。状態は引き続き `REQUIREMENTS_AUTHORITY / EXTRACTION_INCOMPLETE`
であり、ここで記録する証拠計画は実装後に初めて実行する。

### Exact Window matrix

寸法はすべてOS frameを含まないlogical client sizeで記録する。`initial`、`min`、`max`は
それぞれ幅×高さであり、`unbounded`はGraphの上限が製品契約で拘束されないことを表す。
HelpはMain内の情報surfaceで、WindowでもHWNDでもない。

| surface | registered top-level surface | runtime open HWND | logical client initial | logical client min | logical client max | resize | Window controls |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Main | yes | 1 | 900×480 | 900×480 | 900×480 | fixed | minimize, close |
| Setup | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed | minimize, close |
| Settings | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed | minimize, close |
| Graph | yes | 0..1 (singleton) | 940×640 | 700×480 | unbounded | resizable | minimize, maximize/restore, close |
| Threads | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed | minimize, close |
| Legal | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed | minimize, close |
| Main内 Help | no (owner=Main) | 0 additional | Main client 900×480内 | Main client 900×480内 | Main client 900×480内 | Mainに従う | Mainの操作を使用、独自Window controlsなし |

したがって登録top-level surface inventoryは正確に6個（Main、Setup、Settings、Graph、Threads、
Legal）であり、Helpを第7 Windowへ分離しない。runtime open HWNDはMain=1＋open child subset=0..5、
合計1..6で、5 childを全て開いた時だけ6となる。各childはsingletonであり、runtime cardinalityを6へ
固定しない。固定Windowのsupported work
areaは少なくとも `900×480 logical`、Graphのsupported work-area minimumは少なくとも
`700×480 logical`である。固定Windowの境界未満をsupported matrixとして扱わず、font縮小、
clip、scroll、PASS値の捏造で逃げない。境界未満のwork areaは製品の新しいfailure classや
第7 Windowを発明せず、topology manifestの `unsupported_scope` に記録する。

surface/monitor/DPI/sizeのsupported predicateは次のANDで固定する。

```text
supported = client_threshold AND frame_fit
client_threshold(fixed Main/Setup/Settings/Threads/Legal) = logical >= 900×480
client_threshold(Graph) = logical >= 700×480
frame_fit = DPI変換後のDWM.visible_frame全体が対象MONITORINFO.rcWork内へ完全包含
```

fixed Main/Setup/Settings/Threads/Legalのclient thresholdはlogical `>=900×480`、Graphは
logical `>=700×480`。`frame_fit`はDPI変換後に取得したDWM `visible_frame`全体が対象
`MONITORINFO.rcWork`内へ完全包含されること（left/top以上、right/bottom以下）である。
client thresholdは必要条件であり、thresholdだけでsupportedとはしない。thresholdとframe-fitの
ANDがsupportedの十分条件である。

### DPI、座標field、配置、入力の不変条件

- DPI authorityは`GetDpiForWindow`相当のOS-reported integer `dpi`、`scale=dpi/96`とする。
  100%/150%/200%（`96/144/192`）は必須fixtureであるが、対応domainをこの3値へ限定しない。
- logicalの正の幅・高さをphysicalへ変換する場合は各値を独立に
  `floor(logical*dpi/96 + 0.5)`で丸める。丸め前後の値を混同しない。
- geometry evidenceのfieldは少なくとも `logical_client`、`physical_client`、
  `visible_frame`（DWM visible frame bounds）、`work_area`（`MONITORINFO.rcWork`）を別々に
  持ち、各fieldのorigin、width、height、unit（logicalまたはphysical）を明示する。
- Main fresh launchは起動直前のforeground Windowが属するmonitorを対象とし、無効ならprimary
  monitorを使う。child Windowの初回openはowner Windowのmonitorを使う。fresh Main/初回の
  user-openだけをcenter対象とする。reopenは、現在のいずれかの`rcWork`へvisible frame全体が
  包含される時だけlast stable OS rectを復元する。monitor除去/解像度縮小で無効ならMainは
  foreground monitor→primary、childは有効owner monitor→primaryの順で一度だけ
  `topology_recovery center`し、reasonをraw記録する。どのmonitorもsupported predicateを満たさない
  場合は`unsupported_scope`とし、PASSを捏造しない。
- centerは`MONITORINFO.rcWork`とDWM visible frame boundsから、physical座標で
  `origin=work_origin+floor((work_size-frame_size)/2)`を一度だけ計算する。各軸のdouble-coordinate
  residualは`abs((2*frame_origin+frame_size)-(2*work_origin+work_size))<=1`とする。
  通常のtimer、poll、reopen、drag後のrecenter、`Window.Position` loop、cursor合成は0件とし、
  無効reopen時の一度だけの`topology_recovery center`だけを例外とする。
- 登録された6 surfaceのtitle領域のnative moveは1 gestureにつき1回だけ開始し、control hitは0件とする。
  全Windowはminimize/closeを持ち、native maximize/restoreとnative resizeはGraphだけが持つ。
  Graph maximizeは現在monitorのwork areaへ適用し、fullscreenはproduct actionにしない。

### 必須topology cellsと証拠境界

同一DPI monitor crossing、異なるDPI monitor crossing、negative/nonzero monitor origin、
taskbarで縮んだwork areaを独立cellとして扱う。対象monitorは上記supported boundary以上で
なければならない。境界未満は`unsupported_scope` manifestへ記録し、supported PASSへ混ぜない。

この変更の目的は、client/frame/work-areaの単位混同、monitor中心のずれ、control押下をmoveへ
誤変換する回帰、HelpのHWND増殖、minimum sizeの誤適用を抽出段階で閉じることである。理由は、
Windows固有のgeometryはX版の表示値を変えずにOS境界だけを明示する必要があり、実装の現状を
正本へ昇格させると未確認の値を仕様として固定してしまうためである。X版から継承するのは
値、期間、状態、所有権、失敗時保持であり、logical/physical frameの表現、native move、
chapter/page導線はWindows固有の表現として追加する。証拠計画は、同一artifact SHAで各matrix
cellをfresh起動し、OS-reported DPI/monitor/frame/work-area、round式、center residual、
HWND数、native input/resize/drag counters、foreground/cursorのraw traceを保存してから、
実装担当とは別の評価者が独立再計算する。これは証拠取得済みを意味せず、状態は抽出中のまま
保持する。

| 領域 | 正本値・関係式 | 権威資料 | 禁止する勝手な差分 |
| --- | --- | --- | --- |
| REST listen | Linux/WSL、Windows clientともHTTP endpointはloopback `127.0.0.1:8787` | `docs/REST_API_V1.md` §起動・SSH、`docs/WINDOWS_CLIENT.md` §起動・通信境界 | 8765/8000等への変更、`localhost` DNS、LAN/public bind |
| SSH forward | `ssh.exe` をshell経由でなくsaved `connectionSelector`から作る引数配列 `[-N,-o,BatchMode=yes,-L,8787:127.0.0.1:8787,<connectionSelector>]` で直接起動する。selectorはliteral OpenSSH Host alias grammar（1..255）だけで、WSLは別profileのexact distribution tokenを使う。host/user/path/alias展開値は永続化・argvへの追加をしない | `docs/WINDOWS_CLIENT.md` §Windows側の接続導線・SSH境界、`UX-20260822-SSH-001` | remote port分離、password/token埋込み、shell展開、raw host/user/pathの保存、aliasをclient側でHostName/User/IdentityFileへ展開 |
| 認証対象 | `connectionProfile`とsaved `connectionSelector`からprofile-specificな引数配列を作る。WSLはexact `[wsl.exe,-d,<connectionSelector>,--,codex,login]`、remote SSHはexact `[ssh.exe,-o,BatchMode=yes,<connectionSelector>,codex,login]`の固定argvとする。認証開始とstatus再確認は別stateで、認証完了はその同じprofileの`/v1/status`が`state=ready,authenticated=true`になった場合だけ | `docs/WINDOWS_UX_SPEC.md` Setup/Auth decision、`docs/WINDOWS_CLIENT_REQUIREMENTS.md` WIN-PAR-02/WIN-SET-01、`UX-20260822-SSH-001` | remote接続なのに常にローカルWSLでlogin、shell command、開始直後に認証済み扱い、別connectionSelector/epochの結果流用 |
| REST readiness | wireに`ready` boolean keyはなく、同じ完全受理rootの実在2 fieldについて`state == "ready" && authenticated == true`だけがreadyである。`health=200`、listener/process生存、auth child exitは別事実。wireの`state=ready`は入力事実であり、client canonical UI state IDではなく、17-state projection内の`normal`/`quota_warning`/`quota_danger`/`reset_warning`を残量・reset境界から導出する | `docs/REST_API_V1.md` status schema/readiness、`UX-20260822-SSH-001`、`UX-20260823-FULL-STATE-001` | `ready=true` keyの発明、health/process/exitだけでready化、`ready,false`または`auth_required,true`の受理、wire readyをUI state IDへ直接表示 |
| REST surface | read-only `GET /v1/health`、`GET /v1/status`、`GET /v1/details`だけ。`api_version="v1"`とstatus/detailsの完全schemaを受理し、既知pathの非GETは405、未知pathは404 | `docs/REST_API_V1.md`、`src/server.rs` public DTO、`docs/WINDOWS_CLIENT_REQUIREMENTS.md` | endpoint追加、redirect/cookie/proxy、unknown/case-altered/duplicate key受理、架空401/auth endpoint |
| Plan公開値 | exact schema PlanTypeをtrim/alias/prefix一致なしで検証し、serverが`REST_API_V1`の表どおりcanonical `plan_label`と`quota.monthly`を同一cycleで生成する。wireへPlanType/PlanFamily keyは追加せず、Windowsも自由文字列からfamilyを推測しない | `DESIGN.md` PlanType/PlanFamily、`docs/REST_API_V1.md` PlanType写像 | 任意plan label、Enterprise以外のmonthly=true、Business部分一致、unknownへのschema-invalid値の吸収、label/monthly世代混在 |
| REST details thread | wire keyは `id,title,parent_thread_id,model,model_label,total_tokens,context_usage_tokens,context_window_tokens,created_at,last_user_message_at,is_subagent,depth` の12個。`is_orphan`はwire keyではなく、完全に受理した同一threads集合内に非null parent IDが存在しない場合のWindows派生値。native DB/rollout収集段階のdangling edgeによるcycle全体rejectとは段階を分離する | `src/server.rs::PublicThread`、`LoopbackStatusClient.cs::ThreadProperties`、`DetailsContracts.cs::ApiThreadDetails`、`DESIGN.md` Threads収集/presentation境界 | `parent_id/context_tokens/context_limit/snapshot_epoch/status/is_orphan`をwire fieldとして発明、部分/rejected集合からorphan推測、完全受理REST orphanをnative収集danglingと混同してreject |
| REST thread順序 | serverはcanonical active snapshotを`updatedAt desc,id desc`順で`threads`配列へpublishするが、`updatedAt`はwire fieldにしない。Windowsは受理array indexをcanonical rankとしてroot/sibling相対順を保ち、parent-first depth-first/subtree-contiguousへ投影する | `DESIGN.md` canonical snapshot/presentation、`docs/REST_API_V1.md` details thread | Windowsで架空updatedAtを要求・推測、title/受信時刻/IDだけで再sort、server canonical sibling順を破壊 |
| Windows response上限 | response headerは8 KiB。`/v1/status`本文は64 KiB、`/v1/details`本文は32 MiB。detailsはさらにhistory periods 128、history samples 100,000、confirmed history gaps 4,096、threads 256、models 3を独立上限とし、いずれか超過で全candidateを拒否する | `LoopbackStatusClient.cs`の公開契約、`src/server.rs` public bounds、`docs/WINDOWS_CLIENT_REQUIREMENTS.md`、RC-067 | unbounded read、status/detailsを同じ64 KiBに縮退、32 MiB超の部分parse、pending/rejected gap公開、RPC側4 MiBとの混同 |
| Windows poll | 起動直後に1回、各cycle完了から10秒後に次回、各HTTP要求timeoutは3秒、手動更新を含む同時in-flightは最大1。待機中clickはqueueせず無視し、closeでtimer/requestをcancel | `docs/WINDOWS_CLIENT.md` §表示と更新 | daemonの60秒周期との混同、重複poll、完了前から固定intervalを数える、無制限retry |
| 認証消去遷移 | schema-valid `auth_required`、logout、token失効、AccountKey変更は通常のlast-good保持より優先し、旧accountのplan/quota/model/history/thread可視値を1回のroot updateで空にする。Linux側履歴DBは削除せず、認証中は旧account dataへ到達不能にする。消去適用失敗時は旧情報を表示し続けずcontrolled shutdown | `DESIGN.md` AuthEpoch/AuthRequired root、`docs/WINDOWS_CLIENT.md` §表示と更新 | auth_requiredを通信/JSON invalid扱いして旧account値を現在値として表示、DB削除、部分的なfield消去、旧detailsだけ保持 |
| 対応言語 | `ja`, `en`, `zh-Hans`, `ko`, `es`, `fr`, `de`, `pt`, `it`, `ru` の10言語。未知localeは決定的に英語fallback | `DESIGN.md` 情報所有権表、`README.md` 多言語、`src/i18n.rs` catalog | 代表3言語だけを全対応と扱う、surfaceごとの混在fallback |
| 時刻 | persisted Windows `timeZoneId` enumはexact `[local,UTC]`。`local`はprocess起動時のhost IANA zoneへ一度解決し、`UTC`はUTCへ解決する。絶対表示・期間・Graph軸は解決済みzoneの各instantのoffsetを使い、elapsed/countdownはUTC差分とする。任意IANA IDはWindows settingsへ保存しない。無効timestampを推測しない | `RC-059`、`docs/LOCALIZATION.md`、本節の設定永続化・X版意味論不変 | 任意IANA ID保存、起動後のzone再解決、固定JST/UTCだけで全localeを代表、欠測時のnow推測 |
| Graph/モデル表のモデル | Graph系列とモデル利用表は `SOL`, `TERRA`, `LUNA`だけ。Graph系列順はRemaining→LUNA→TERRA→SOL、色はRemaining=`#56b2f5`、LUNA=`#e6a23c`、TERRA=`#5dc98a`、SOL=`#a88cf5`。3モデルは独立toggleで同時表示でき、単一選択式へ変更しない | `DESIGN.md` Graph、`ui/theme.slint`、graph fixture/Decision | unknown modelの別系列化、単一モデルselectorへの縮退、系列順・色の無断変更 |
| Thread概要のモデル分類 | active threadがある場合は`SOL`, `TERRA`, `LUNA`の0件も表示し、validated model labelに既知tokenがちょうど1つ含まれる場合だけ3分類へ入れる。それ以外は`その他`へ集計する。Graph/モデル利用表の3モデル制限とは別ownerである | `DESIGN.md` 情報所有権表「スレッドモデル分類」 | thread unknownをGraph系列へ追加、`その他`の無断削除、unknown threadをactive総数から落とす |
| Graph期間・横軸 | current期間は左端=`period.start_at`、右端=`min(quota.reset_at, now)`。同じaccepted rootのcurrent `period.end_at`は`quota.reset_at`とexact一致を必須とし、不一致rootをrejectする。past期間は保存済み確定`period.end_at`。開始から右端までをplot幅100%へ写像し、未来のresetまでの空白を予約しない | `DESIGN.md` Graph、`WINDOWS_CLIENT_REQUIREMENTS.md` WIN-PAR-14、`UX-20260822-GRAPH-001` | `period.end_at`と`quota.reset_at`の選択的使用、current右端を未来resetへ固定、最初のsampleを左端にする、開始前/右端後の値を期間内へ捏造 |
| Graph期間label | server `history_periods[].label`はLinux/X reference、選択keyはcanonical `id`と受理array index。Windows表示は同じ`id/start_at/end_at/current`を保存済み`timeZoneId`・locale・登録suffixで再renderし、両端instant/offset/suffix roleを1対1に保つ。label文字列の日時/ID逆parseは0 | `DESIGN.md` period label、`docs/REST_API_V1.md` history period、`UX-20260822-GRAPH-001` | wire labelをselection keyにする、locale/timezone不一致の無条件echo、重複label、DST/suffix/current role欠落、文字列parseから期間を推測 |
| Graph縦軸・終端書式 | Remainingはモデル軸と独立した0..100%。ドル軸上限は表示中モデルの期間内最大（0なら描画用1）、token軸上限は期間内3モデルの共通最大（0なら描画用1）。token軸は`<1000`整数、`>=1000`/`>=1M`/`>=1B`をそれぞれ小数1桁K/M/B、ドル軸とドル終端値は`$`＋小数2桁 | `DESIGN.md` Graph、`src/main.rs::graph_paths_for_selection`、`dollar_axis_labels`、`format_token_axis_value` | Remainingをドル/token軸へ載せる、非表示ドル系列をscaleへ含める、token toggleで個別scaleへ変更、小数桁・suffixの無断変更 |
| 残量 | 有限な0..100%。通常の欠測は利用量から推定せずlast-goodまたは未取得。ただし両側の実測残量低下でbracketされたactive interior欠測だけは`DESIGN.md`の線形補間規則を適用する。`RecorderGapLedger`確定gap、expired reset hint、tombstoned AuthEpoch、終端欠測は補間せずgap markerまたはlast-goodを保持する。期間左端から現在/確定終端までを使う | `DESIGN.md` Graph意味論、`GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md` | 終端だけ82%へ補間、アイドル中の変化、確定gap/expired reset hint/tombstoned AuthEpochの補間、100%超への復帰 |
| Window寸法・topology | 登録top-level surfaceはMain/Setup/Settings/Threads/Legal=logical `initial=min=max=900×480` fixed、Graph=`initial=940×640,min=700×480,max=unbounded,resizable`、Help=Main内900×480/additional HWND=0。runtime HWNDはMain=1＋child subset 0..5、合計1..6のsingleton。supportedはclient threshold AND DPI後DWM visible_frameのrcWork完全包含 | 本節のExact Window matrix、supported predicate、`docs/WINDOWS_UX_SPEC.md` geometry authority | 700×480をfixed Windowへ適用、runtime cardinalityを6へ固定、frame-fit未確認でのPASS、unsupported work areaの混入、font縮小/clip/scrollでの回避、Helpの第7 Window化 |
| Windows Window集合・操作 | borderless top-level surface inventoryはMain、Setup、Settings、Graph、Threads、Legalの正確な6個。登録された6 surfaceはnative moveを1 gestureにつき1回、control hit=0で扱う。登録された6 surfaceはminimize/close、Graphだけnative resize/maximize/restore。Graph maximizeはcurrent monitor work area、fullscreenはproduct actionでない | 本節のDPI、配置、入力の不変条件、`docs/WINDOWS_UX_SPEC.md` geometry authority | Help HWND追加、control pressでmove開始、fixed Windowのresize/maximize、fullscreen action、timer/poll/recenter、cursor合成 |
| Windows DPI/geometry field | integer `dpi`=`GetDpiForWindow`相当、`scale=dpi/96`。96/144/192は必須fixtureだがdomainを限定しない。positive sizeは`floor(logical*dpi/96+0.5)`。`logical_client`、`physical_client`、DWM `visible_frame`、`MONITORINFO.rcWork` `work_area`を別fieldで保持 | 本節のDPI、座標field、配置、入力の不変条件、`docs/WINDOWS_UX_SPEC.md` | logical/physical/frame/work-area混同、float DPIの正本化、fixture値のdomain限定、client寸法をframeへ転用 |
| Windows center/reopen | Main freshは直前foreground monitor（無効時primary）、child初回openはowner monitor。centerは`work_origin+floor((work_size-frame_size)/2)`、double-coordinate residual各軸≤1。reopenはvisible_frameが現存いずれかのrcWorkへ完全包含される時だけlast stable rectを復元し、無効時はMain=foreground→primary、child=owner→primaryへ一度だけ`topology_recovery center`してreasonをraw記録。通常のtimer/poll/reopen/drag後recenter、`Window.Position` loop、cursor合成=0（無効reopen時の一度だけのtopology recoveryを除く） | 本節のDPI、supported predicate、配置、入力の不変条件、`UX-20260822-UX-002` | 2px以上の中心差、frame/client混同、frame-fit未確認復元、再center loop、cursor移動、実装のcenter helperを正本扱い |
| Graph座標 | 製品座標は`DESIGN.md`のwindow/content/plot式から算出する。fixture画像座標はそのfixtureだけのoracle | `DESIGN.md` Graphレイアウト、graph fixture contract | `left=40,width=900`等の任意fixture座標を製品固定値にする |
| 非スクロールUX | root/internal `ScrollViewer`だけを到達手段にする旧要求を`UX-20260822-UX-002`で明示supersede。Main/Setup/Settings/Graph/Threads/Legalの主要情報・primary・Back・Closeは同一viewport、長文/一覧はpage/step/chapter/detail/collapseで分割 | `UX-20260822-UX-002` §決定・固定条件・非スクロール影響、`docs/WINDOWS_UX_SPEC.md` §2.1 | root/internal scrollへの依存、primary/Back/Closeのviewport外、Legal scrollbar、700×480を全Windowへ適用 |
| keyboard/focus | `windows-keyboard-v1`の6 Window Tab列、Enter/Escape/Alt routeを使う。focus indicatorは2 logical pixel以上の連続面積、隣接色との変化3:1以上で、通常/high-contrast/100・150・200% DPIを満たす | `UX-20260823-KEYBOARD-001`、`docs/WINDOWS_UX_SPEC.md` §5 | toolkit任せの未定義順、mouse fallback、background focus/cursor奪取、画面外focus、根拠のない別閾値 |
| 設定永続化 | JSONへ保存できるproduct fieldは `language`、`setupCompleted`、`connectionConfigured`、`timeZoneId`、`connectionProfile`、`connectionSelector`の6 keyだけ。profile enumは`none`、`wsl`、`sshConfigAlias`のいずれかで、selectorはWSL exact distribution tokenまたはliteral OpenSSH Host alias grammar。secret、展開値、raw host/user/path/port/key/password/token/API URL/SSH commandを保存しない。saved selectorでauto reconnectし、primaryは同一directoryの一時fileをflush後atomic replaceし、失敗時は旧primaryを保持する。製品判定は`PRODUCT_PENDING` | `docs/WINDOWS_CLIENT_REQUIREMENTS.md` WIN-SET-03/04、`docs/WINDOWS_CLIENT.md` §初回セットアップ、`UX-20260822-SSH-001` | `schema_version`等の未承認field、資格情報・remote target保存、失敗時の旧設定破壊、根拠のないbackup file、saved selector以外の自動接続 |
| 起動routing | 設定fileなしはSetupを開く。valid markerとsaved selectorがありschema検証を通ればprofile-specific `ArgumentList`＋remote `BatchMode=yes`でauto reconnectを開始するが、開始成功とstatus再確認を別stateにする。malformed/empty/truncatedは接続済みを偽らずMainのdisconnected状態を表示し、4-key recoveryはMain disconnected＋Settingsだけで行う。破損時にWelcome/Setupを毎回自動表示しない | `docs/WINDOWS_CLIENT_REQUIREMENTS.md` WIN-SET-03/04、`docs/WINDOWS_UX_SPEC.md` §Settings/Recovery、`UX-20260822-SSH-001` | 破損時Setup loop、破損marker捏造、Mainへ到達不能、last-good Linux DB/history削除、開始直後のready/別selector結果流用 |
| installer版 | `artifact_manifest.version == installed DisplayVersion == shortcut target file version`。更新fixtureは `V_old != V_new` で表す | installer manifest、`windows-client/installer/Program.cs`、artifact evidence manifest contract | 根拠のない2.3.0/2.4.0を製品版として固定 |
| source/artifact SHA | 要求文書はexact pathとhashの関係式を固定し、実際の64-hex SHA-256は要求抽出freeze時または製品release freeze時のmanifestへ取得する。文書編集後は旧hashをPASSへ流用せずmanifestを再生成する。具体契約内へ作業途中のmutable file hashを製品固定値として埋め込まない | canonical index、release manifest、artifact evidence manifest contract | 古いSHAの固定、要求文書自身の編集で即staleになるhash、異なるartifactへ同じSHA要求、未取得hashの捏造 |
| installer payload方式 | 配布対象は`win-x64` self-contained Windows client payloadを内蔵したself-contained single-file `CodexInfo.WindowsClient.Setup.exe`。clean supported Windowsに外部.NET Desktop Runtime/SDKを要求しない。.NET runtimeを含む全runtime/依存通知を同一artifactへ同梱し、不足時は配布FAIL | ユーザーの通常Windows導入要求、`Build-WindowsInstaller.ps1`のpublish境界、WIN-H-001/H-002 | framework-dependent成果物を通常顧客へ配布、別payload folder、SDK/Visual Studio/manual build要求、runtime notice欠落のまま出荷 |
| install root | per-user既定は `%LOCALAPPDATA%\Programs\Codex Info Monitor`相当、Start MenuはユーザーPrograms配下 | `windows-client/installer/Program.cs` の `DefaultInstallDirectory`/`StartMenuDirectory` | versioned想像path、HKLM必須化、System32 cwd |
| uninstall保持 | executable/shortcut/HKCU registrationを除去。通常削除はsettings/server historyを保持し、明示purgeだけ別確認 | `docs/WINDOWS_CLIENT.md`、installer `Uninstall`、data protection policy | 通常uninstallで履歴削除、cancel後の部分削除 |
| daemon周期 | 既定60秒、設定可能5..3600秒、bounded wait。UI/RESTから独立しsingleton leaseを使う | `docs/WINDOWS_CLIENT.md` daemon節、`DESIGN.md` DB節 | busy polling、複数collectorの二重writer、UI終了連動停止 |
| daemon lease | live PID/process identityを確認し、同じlock file identityを再確認できたstale leaseだけを回収。経過時間だけで奪わない | `src/daemon.rs` `lock_is_stale`/`DaemonLock::acquire`、`docs/WINDOWS_CLIENT.md` | 24時間等の任意threshold、live owner上書き、検査後に差し替わったlock削除 |
| DB競合 | canonical DB内のlogical partitionを先に確定し、SQLite transaction、busy timeout 2秒、unique `(partition_id,reset_at,timestamp)`、MAX/non-NULL merge、batch rollbackを使う | `DESIGN.md` DB節、`docs/DATA_PROTECTION_POLICY.md` §8.6 | partitionを含まないcross-account key、lock無視、部分commit、失敗時DB再生成、後退値 |
| backup/migration | prune前online backup 3世代。migrationは別名candidate→全行/件数/hash/境界検証→成功時だけatomic switch | `DESIGN.md` DB節、`docs/DATA_PROTECTION_POLICY.md` | 世代数変更、backup前prune、in-place推測migration、自動破壊復旧 |
| retention/prune | 3暦月境界より古い行だけを、検証済みbackup成功後にpruneする。backup失敗時は0行削除 | `DESIGN.md` Graph履歴/DB節、`docs/DATA_PROTECTION_POLICY.md` | 境界内削除、成功時も永久にpruneしない契約、backup失敗後のDELETE |

## 機械ゲートへ渡す不変条件

1. atomic contract全226行のID集合はbaselineと完全一致する。
2. E/Iのendpoint契約内に `8765` または `8000` があればFAILとする。
3. G-001は上記10言語を全て列挙し、全surface key集合の一致を独立oracleで判定する。
4. Hの版番号はmanifest関係式または明示した `fixture_only` だけを許可する。
5. 画面座標は正本式への参照または `fixture_only` の区別を必須とする。
6. 正本と契約が矛盾する場合、実装の現状に合わせて正本を黙って変更せず、抽出FAILとして止める。
7. Windows pollの10秒/3秒/1本とdaemonの60秒/5..3600秒を同じ周期として扱った行はFAILとする。
8. REST thread行に上記12 wire key以外の架空fieldがある、または`is_orphan`をwire fieldとする場合はFAILとする。
9. installer行がframework-dependentまたは外部.NET runtime必須を許す場合はFAILとする。

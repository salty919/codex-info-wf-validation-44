# UX Decision: Windows full visual state matrix

Decision ID: `UX-20260823-FULL-STATE-001`

状態: `EXTRACTION_DECISION_RECORDED / PRODUCT_PENDING`

## 利用者の課題

正常、warning、errorの一部だけを確認すると、起動中、認証要求、quota境界、空履歴、resource別失敗、
stale保持の画面が未定義のまま実装され、未確認状態で値が0へ置換されたり、復旧操作が消えたりする。
利用者は状態ごとに何が最新で何が保持されたかを推測できない。

## 目的

要求された17 stateを一つの有限集合へ固定し、各stateのowner、適用surface、保持、primary action、
locale/size/DPI/theme/motion dimensionを明示する。通常状態の画像を別stateの証拠へ流用せず、X版の
data/period/threshold/last-good意味論を維持したままWindowsの表示・導線だけを決定する。

## 検討した案と棄却理由

1. `normal`、`warning`、`error`だけを共通fallbackにする案は、initializing、auth、各resource error、
   stale、no-historyの保持と復旧CTAを隠すため棄却する。
2. surfaceごとに別の状態名・優先順位を発明する案は、同じsnapshotがMain、Graph、Threadsで異なる意味に
   見え、last-good境界を追跡できないため棄却する。
3. 17 stateをcanonical state manifestへ登録し、各surface projectionとN/Aを明示して全dimensionへjoin
   する案を採用する。複数失敗を一つの推測classへ丸めず、既存のfailure recovery decisionを参照する。

## 採用案

### 1. 完全state集合

state IDは次の17件だけとする。

`initializing`, `auth_required`, `normal`, `quota_warning`, `quota_danger`, `reset_warning`, `zero`,
`full`, `api_error`, `transport_error`, `status_invalid`, `details_invalid`, `history_error`,
`thread_error`, `db_error`, `stale`, `no_history`

`SETTINGS_SAVE_FAILED`はこの17 stateへ吸収せず、`UX-20260823-ERROR-001`のcanonical failure classとして
Settings instanceへ表示する。SSHのprofile invalid/local port in use/interaction required/process start-or-exit/
health unavailableも、`UX-20260823-ERROR-001`のfailure classであり、OpenSSH generic exitをDNS/key/passwordへ
推測する状態を追加しない。

### 2. state owner、surface projection、保持

略号は`M=Main`、`S=Setup`、`St=Settings`、`G=Graph`、`T=Threads`、`L=Legal`、`H=Main内Help`である。
state manifestは全surfaceに`applicable=true`または明示的な`N/A`を記録し、通常画像をN/Aの代用にしない。

| state | owner / trigger | surface projection | primary / visible result | failure時保持・禁止 |
| --- | --- | --- | --- | --- |
| `initializing` | client startup/reconnect supervisor | M,S,St,G,T,L,H | 初期化中表示。Setup/Settings/各child/Helpへ到達可能 | 既存last-goodがあれば保持し、なければ値を0や正常へ捏造しない。未完了processを隠さない |
| `auth_required` | auth checkが未完了/期限切れ | M,S,St、G/TはN/Aまたは保持、L/H | `action.auth.start`またはSettings recovery | auth clear規則に従いaccount可視値をclearする。秘密値、設定成功marker、DB/historyを捏造しない |
| `normal` | complete valid snapshot | M,S,St,G,T,L,H | 各surfaceのcanonical primary | valid pair、selection、settings、DB/history意味論を変更しない |
| `quota_warning` | X版のwarning境界 | M,G、S/St/T/L/Hはstatus chromeを表示 | warning stateと既存primary | quota/status/dataを丸めない。X版の境界値を再定義しない |
| `quota_danger` | X版のdanger境界 | M,G、S/St/T/L/Hはstatus chromeを表示 | danger stateと既存primary | warningへ弱めず、値・期間・更新所有者を変更しない |
| `reset_warning` | reset boundary warning | M,G、S/St/T/L/Hはstatus chromeを表示 | reset warningとperiod action | reset前後のperiod/elapsed/countdown semanticsを変更しない |
| `zero` | quota/remaining fixture=0 | M,G、S/St/T/L/Hは値を参照する表示 | zeroを0として表示し、status/復旧操作を残す | negative/blank/normalへ置換しない。別resource/last-goodを消去しない |
| `full` | quota/remaining fixture=full | M,G、S/St/T/L/Hは値を参照する表示 | fullを100%相当の既存意味で表示 | fullをwarning/dangerへ推測変換しない。期間・単位を変更しない |
| `api_error` | REST response/endpoint errorを受理 | M,S,St,G,T,L,H | fixed API failure classと1 primary CTA | valid last-good pair、settings、selectionを保持。raw bodyを表示しない |
| `transport_error` | usable HTTP transportなし | M,S,St,G,T,L,H | connection failure classとrecheck/settings CTA | last-goodを保持、process/health境界を分離。DNS/key/passwordをgeneric exitから推測しない |
| `status_invalid` | status schema/domain invalid | M,S,St,G,T,L,H | `action.refresh`とstatus invalid表示 | status last-goodとdetails last-goodを保持。partial snapshotを公開しない |
| `details_invalid` | details schema/domain invalid | M,S,St,G,T,L,H | details recheckとdetails invalid表示 | valid status、details last-good、quotaを保持。statusを巻き戻さない |
| `history_error` | history resource unavailable/invalid | M,G,T、S/St/L/Hはstatus chrome | history recheck。Graph periodは選択不能またはlast-good | quota/status、history last-goodを保持。空値や新規historyを捏造しない |
| `thread_error` | thread resource unavailable/invalid | M,T,G、S/St/L/Hはstatus chrome | thread recheck。Threads Back/Closeを残す | status/quota、thread last-goodを保持。0件へ勝手に置換しない |
| `db_error` | server DB failureをAPI経由で観測 | M,S,St,G,T,L,H | data unavailable classとconnection recheck | Windows direct DB access=0、全last-good/設定を保持。DB再生成・空DB置換をしない |
| `stale` | last-goodは存在するがfresh取得不能 | M,S,St,G,T,L,H | stale indicatorとconnection/recheck CTA | last-good値、selection、page、settingsを保持。freshと表示しない |
| `no_history` | valid snapshotに履歴itemがない | M,G,T、S/St/L/Hはstatus chrome | Graph period/Threadsは「履歴なし」表示、Back/Close/Helpは到達可能 | period/thread選択肢をfabricateしない。空をerrorやzeroへ誤分類しない |

`applicable`のsurface projectionは表示ownerと到達性を分ける。例えば`history_error`でもMainのstatusと
Threads/GraphのBack/Closeは消さず、Legal/Helpのpre-authored contentはresource errorで削除しない。

### 3. 適用dimension

state manifestの一行は次の直積へjoinする。適用されない組み合わせは`N/A(reason=...)`を明示し、別stateの
画像・text・UIA treeを再利用しない。

| dimension | required values |
| --- | --- |
| surface | Main、Setup、Settings、Graph、Threads、Legal、Main内Help（Help additional HWND=0） |
| logical client size | fixed surface `900×480`、Graph `940×640 initial`/`700×480 minimum`、HelpはMain client内 |
| DPI | OS integer dpi、`scale=dpi/96`。`96/144/192`（100/150/200%）は必須fixture、domain全体は限定しない |
| theme | normal、high contrast |
| motion | full motion、reduced motion |
| locale | `ja,en,zh-Hans,ko,es,fr,de,pt,it,ru`、unknown→`en` |
| topology | supported monitor/work-area cells。unsupported boundaryはscope外manifestへ分離 |
| input | mouse-free keyboard Tab/Shift+Tab/Enter/Escape/Alt、UIA activation、native Back/Close |

size/DPI/frame-fitの詳細は既存geometry authorityを参照し、threshold未満・frame-fit不能をこのstate matrixの
supported PASSへ混ぜない。全stateでprimary、Back、Close、focus visual、UIA Name/Description、clipを同じ
viewport条件で記録する。

### 4. エラー・focus・非スクロール境界

state表示のStatus ownerは同時に一つだけで、Cause/Impact/Recovery keyを分離する。`api_error`、
`transport_error`、resource error、`stale`は`UX-20260823-ERROR-001`のfailure class/retentionへjoinし、
generic raw error、secret、path、stderrを表示しない。background pollはfocus/cursorを変更せず、利用者の
primary CTAだけがroute/focusを変更する。

全surfaceの主要情報、primary、Back、Closeは同一viewportに残し、長い集合・Legal/Help本文はpage/章/選択詳細の
semantic IDとtext hashで分割する。root/internal ScrollViewer、font縮小、文字途中切断をstate recoveryに
使わない。Graph/Threadsの個別Back/CloseとMain内Help scopeはそれぞれのDecisionへhard joinする。

## X版との関係

X版のstate owner、quota/reset boundary、epoch/period、duration/countdown、history/thread/data意味論、
last-goodとDB非破壊を変更しない。Windowsはstate manifest、catalog key、UIA、page/viewport表現を追加する
だけで、missing dataを0/full/normalへ変換しない。

## 影響要求

`RC-088`, `RC-089`, `RC-090`, `WIN-G-016`, `WIN-M-014..015`, `WIN-M-021`, `WIN-M-026`, `WIN-M-029`,
`WIN-M-030`, `WIN-C-007..009`, `WIN-F-004..011`, `WIN-I-014..016`, `WIN-J-008..009`。

## 非スクロール影響

17 stateの各applicable surfaceで、主要値、state Cause/Impact、primary CTA、Back、Close、UIA focusを同じ
canonical viewportへ固定する。page/paragraph semantic joinは`missing=0`、`extra=0`、`duplicate=0`、
`clip=0`とし、state変化でScrollViewerを到達手段にしない。N/A stateはmanifestで明示し、正常画像の流用を
受入根拠にしない。

## 証拠計画

同一release artifact SHAで、17 state×7 surface projection×supported size×DPI×theme×motion×10 locale＋
unknown→en×topology×keyboard/UIAのfresh画像とraw manifestを取得する。各rowについてstate ID、owner、
source fixture、applicable/N/A理由、Cause/Impact/CTA key、before/after resource hash、last-good、
UIA AutomationId/Name/Description、focus、route/action count、semantic page hash、bounds、clip、
scroll input、HWND subsetを記録する。`SETTINGS_SAVE_FAILED`と5 SSH canonical classはError Recoveryの
別fixtureとしてjoinし、generic exitの推測分類0を確認する。実装者と異なる担当が三値判定する。

## 未確定

17 state、適用dimension、projection、保持境界はDecisionとして確定した。実artifact、fresh画像、実Windows
UIA/keyboardログ、resource hash、独立製品判定は未取得であり、製品状態は`PRODUCT_PENDING`である。X版の
threshold/period/状態意味論を変更する判断、未登録stateの追加、generic SSH exitの推測分類は未採用である。

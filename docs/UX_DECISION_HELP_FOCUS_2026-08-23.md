# UX Decision: Main HWND Help focus scope

Decision ID: `UX-20260823-HELP-FOCUS-001`

状態: `EXTRACTION_DECISION_RECORDED / PRODUCT_PENDING`

## 利用者の課題

Helpを開いたときに別Windowが増えたり、Tabが背後のMain/childへ抜けたり、`Escape`後に呼出元の
route/focusへ戻れなかったりすると、利用者は説明を読んだあと自分がどこにいたかを再構成しなければ
ならない。Helpの本文を表示できても、UIA name、chapter/page、Back、Close、focus visualがない状態は
キーボード利用者と支援技術利用者にとって到達不能である。

## 目的

- HelpをMain HWND内の独立focus scopeとして表示し、追加HWNDを0にする。
- chapter/page、Back、Close、Tab/Shift+Tab、Enter、Escapeを一意に固定する。
- Helpを開いたentryのcaller route/HWND/focusを記録し、Back/Close/Escape後に同じcallerへrestoreする。
- 10 locale＋unknown→en fallbackで同じAutomationId、focus topology、keyboard actionを保つ。

## 検討した案と棄却理由

1. Helpを第7 top-level Windowとして開く案は、登録top-level inventoryとruntime HWNDの境界を変え、
   owner monitor、lifecycle、close時のcaller restoreを二重化するため棄却する。
2. Mainの通常navigationへHelp本文を混在させ、Tab順をtoolkitへ委ねる案は、本文のpage操作と呼出元focusを
   失い、localeや長文でscopeが変わっても検出できないため棄却する。
3. Main HWND内に明示的な`Main.help` scopeとsemantic page manifestを持たせ、caller tupleとUIA focus列を
   固定する案を採用する。Back/Close/Escapeの実keydownとUIA treeを独立に観測できる。

## 採用案

### 1. Window topologyとentry manifest

登録top-level surface inventoryはMain、Setup、Settings、Graph、Threads、Legalのexactly sixである。
runtime open HWNDは`Main=1 + open child subset=0..5`、合計`1..6`であり、5 childを同時に全て開いた時だけ
6になる。Helpはこのinventoryへ追加せず、Main HWND内の`Main.help` surfaceとして
`additional_hwnd=0`を必須値とする。

Help entryごとに次のcaller tupleを一度だけ記録する。

| field | 契約値 |
| --- | --- |
| `caller_surface` | Helpを開いた既存surface（Main/Setup/Settings/Graph/Threads/Legal） |
| `caller_hwnd` | そのsurfaceの既存singleton HWND。新規HWNDは作らない |
| `caller_window_instance_generation` | 同じ数値HWNDのreuseを区別するPID内単調増加generation |
| `caller_route` | entry直前のrouteとpage/selection識別子 |
| `caller_focus_automation_id` | entryを起こしたcontrolのlocale不変AutomationId |
| `main_return_route` | Help entry直前にMainが所有していた非Help route/page/selection |
| `help_scope` | `Main.help`、Main HWNDに一つだけ |
| `additional_hwnd` | `0` |

Main `nav.Help`、各childの`nav.Help`、SettingsのHelp actionなどHelpへ到達する全entryは同じmanifestへ
joinする。caller tupleを取得できないentry、二重scope、追加HWNDは開く操作を成功扱いにしない。

### 2. chapter/pageと操作

Helpのpre-authored chapter/pageはsemantic IDとparagraph hashで管理し、現在chapter/総chapter、現在page/総page、
前/次、Back、Closeを一つのMain viewportに表示する。page境界では`page.Previous`/`page.Next`をdisabledにし、
disabled actionはpage、route、caller tupleを変更しない。

### 3. focus、keyboard、caller restore

Help scopeの初期focusとforward列は次で固定する。

```text
initial = help.Chapter
forward = help.Chapter
          -> page.Previous(if present)
          -> page.Next(if present)
          -> action.Back
          -> action.Close
reverse = 完全なforwardの逆順（Shift+Tab）
```

本文、装飾、disabled controlはTab列へ入れない。`Enter`はfocused actionを1回だけ実行し、keyup、key repeat、
busy中の再keydownで追加page/close/routeを作らない。`Escape`、visible Back、title Closeはそれぞれ1回だけ
Help scopeを閉じ、entry時に記録したcaller HWNDを前面化し、caller route/page/selectionと
`caller_focus_automation_id`へfocusをrestoreする。callerのsnapshot、DB、settings、last-good、selection、
pageをHelp closeの副作用で変更しない。Help close後に推測でMain monitorへfocusを移すこと、cursorを合成して
callerへ移すこと、background pollでrestoreを再実行することはしない。

restore直前に`(product PID,caller_hwnd,caller_window_instance_generation)`とsingleton registryを再検証する。
callerがclose済み、別instanceへ置換済み、数値HWNDがreuseされた、またはroute/controlが現行UIA treeにない場合は、
旧callerへmessage/focusを送らず、同じMain HWNDの`main_return_route`を1回だけ復元して`nav.Help`へfocusを置く。
Mainが`ShuttingDown`ならHelp scopeを閉じるだけでroute/focus restore、window生成、poll再開を0件とする。
Escape、visible Back、title Close、caller破棄が競合しても一つの`HelpCloseToken`をCASし、close action、route restore、
focus restoreをそれぞれ最大1回にする。late eventはno-opで、snapshot/settings/DB/cursorを変更しない。

### 3.1 caller lifecycleとmulti-caller re-entry（RC-099/100）

Help lifecycleは`Closed → Opening(entry token) → Open(first caller tuple, HelpScopeGeneration) →
Closing(HelpCloseToken) → Validate → RestoreCurrent/FallbackMain/ShutdownDrop → Closed`とする。入力は
Main/Setup/Settings/Graph/Threads/Legalの各`nav.Help`、`Alt+H`、UIA Invoke、Back/Close/Escape、callerのdestroy・
recreate・同一数値HWND reuse、Main `ShuttingDown`を含む。`Opening`中の同一entryは同じtokenへjoinし、既に`Open`なら
同じcallerからの再entryも追加scope/HWND/callbackを作らず、異なるcallerからの再entryも最初のcaller tupleを上書きせず
既存Help scopeへjoinする。従ってClose後の戻り先はfirst caller tupleで一意である。

`HelpCloseToken`を取得できた最初のBack/Close/Escapeだけが`Closing`へ遷移し、keyup、key-repeat、同時double-close、
遅延callbackはno-opとする。restore直前と`Opening`の再entry時に`(product PID, PID start token, caller HWND,
WindowInstanceGeneration, MainGeneration, HelpScopeGeneration)`およびsingleton registryを再検証する。一項目でも
不在・不一致・取得失敗なら旧callerへmessage/focus/routeを送らず、Mainが生存している場合だけ`main_return_route →
nav.Help`を一回復元し、Main `ShuttingDown`ではroute/focus restore、window生成、poll再開を0件とする。scope、route、focus、
close actionは各1回以下、snapshot/settings/DB/cursor bytesは不変である。

### 4. UIA、visual、locale

Helpの各controlはlocale不変の`AutomationId`、catalog由来の非空`Name`、`HelpText`/`Description`、
`AcceleratorKey`、visible boundsを持つ。focus indicatorはcontrol perimeterに連続する2 logical px以上、
隣接色差3:1以上とし、通常theme、高コントラスト、100/150/200% DPI、reduced motionでcontentを隠さず、
viewport端clipを0にする。

localeは`[ja,en,zh-Hans,ko,es,fr,de,pt,it,ru]`とunknown→`en` fallbackを使用する。resolved localeが
変わってもHelpのchapter/page order、AutomationId、focus列、shortcut、Back/Close/Escape action、caller
restoreは変えない。翻訳catalogのsemantic item/paragraphはlocaleごとに一度だけpageへ割り当てる。

## X版との関係

X版のデータ、時刻、期間、状態、全paragraphの意味論と所有権は変更しない。HelpはWindows固有の
server/API、WSL、remote SSH、auth、設定/復旧、update/uninstall、診断の説明surfaceを追加するだけであり、
Helpを第7 Windowへすること、本文削除・要約・並べ替えでviewportへ合わせることは採用しない。

## 影響要求

`RC-084`, `RC-085`, `RC-098`, `RC-099`, `RC-100`, `WIN-G-014..015`, `WIN-K-008`, `WIN-K-010..012`,
`WIN-M-011..013`, `WIN-M-025`, `WIN-M-028..030`。

## 非スクロール影響

Helpのchapter/page、Back、Close、primary page action、focus indicatorはMain client viewport内に完全表示する。
root/internal ScrollViewerを到達手段にせず、長い本文はparagraph境界でpage分割する。pageごとにsemantic IDの
missing/extra/duplicateと文字clipを0にし、Tabはscope外へ抜けない。Helpのadditional HWNDは常に0である。

## 証拠計画

同一release artifact SHAのfresh processで、全Help entry（6 surfaceのcaller route）、全chapter/page、
10 locale＋unknown→en、17 state、supported size/DPI/theme/motionについて、caller tuple、HWND集合、
`additional_hwnd`、UIA tree、AutomationId/Name/Description/shortcut、initial/forward/reverse focus、
Enter/Escape/Back/Close action count、caller restore route/focus、page/paragraph hash、bounds、clipをraw記録する。
Main HWND内Helpの`additional_hwnd=0`と、追加top-level Window/HWNDが0であることを別のOS観測で確認する。
caller alive/closed/replaced/HWND-reused、Main ShuttingDown、Escape+Back+Close同時競合を別caseにし、
instance generation、HelpCloseToken、old HWND message count、Main fallback route/focus、snapshot/settings/DB/cursor hashを記録する。
さらに6入口の同一caller再entry・異caller再entry・同時double-entry/double-closeを各2回以上実行し、
`HelpScopeGeneration`、first caller tuple保持、scope/HWND/subscription数、focus owner、route/action count、late event no-opを
rawで結合する。実装、fresh画像、UIA操作ログ、独立製品判定は未取得であり、製品PASSを導出しない。

## 未確定

Help scope、topology、caller restore、keyboard/UIA、locale、page完全性の契約は確定した。実artifact、物理Windows
hostのfresh画像・操作ログ、focus contrast測定、独立判定は未取得であり、製品状態は`PRODUCT_PENDING`である。
追加failure class、第7 Window、Main以外のfallback Windowは導入しない。caller消失/reuseとMain shutdownは上記の
決定的fallback/idempotent closeで処理し、製品実装・実機証拠なしに完了へ昇格しない。

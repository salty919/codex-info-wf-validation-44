# UX Decision: Windows keyboard navigation and focus

Decision ID: `UX-20260823-KEYBOARD-001`

状態: `EXTRACTION_DECISION_RECORDED / PRODUCT_PENDING`

## 利用者の課題

マウスを使えない利用者、キーボードを主に使う利用者、画面拡大や高コントラストを使う利用者が、
どこにフォーカスがあり、次の `Tab` がどこへ進み、`Enter`、`Escape`、`Alt` が何を行うかを
画面ごとに推測しなければならない状態を許容しない。単にcontrolへ `Focusable=true` を付けるだけでは、
欠落、循環、画面外focus、マウスfallback、意図しない終了を検出できない。

## 目的

- Main、Setup、Settings、Graph、Threads、Legalの登録top-level 6 surfaceと、Main HWND内Helpの同じ規則の
  キーボード導線を提供する。Helpは独立focus scopeだがtop-level Window/HWNDを追加しない。
- フォーカス位置を色だけに依存せず、通常theme、高コントラスト、100/150/200% DPIで識別可能にする。
- キーボード操作でマウスポインタを移動せず、background更新で利用者のfocusを奪わない。
- 主要操作、戻る、閉じるをroot scrollなしで完了できるようにする。

## 検討した案

1. UI toolkitの自動Tab順と既定focus visualだけへ委ねる。
   動的control、custom title bar、ページング、locale変更で順序や可視性が変わっても契約で検出できず、
   過去の移動不能・focus不明を再発させるため棄却する。
2. 各Windowが独自のshortcutとEscape動作を持つ。
   同じキーが画面ごとに保存、終了、戻るへ変化し、誤操作を生むため棄却する。
3. 共通navigation manifest、Window別のexact Tab列、共通Enter/Escape規則、測定可能なfocus visualを固定する。
   実keydown/keyupとUI Automationで独立再現できるため採用する。

## 採用案

### 1. 共通navigation manifest

manifest IDは `windows-keyboard-v1` とする。全localeで次のrouteを変えず、各メニュー項目の
表示label、tooltip、accessible descriptionへshortcutを併記する。

| 入力 | exact route |
| --- | --- |
| `Alt` keydown | 現在routeに対応するnavigation itemへfocusを移し、keyboard cueを表示する。route自体は変えない |
| `Alt` keyup | action 0、route変更0。keydownで確定したfocusを保持する |
| `Alt+M` | Monitor/Mainを開く |
| `Alt+G` | Trends/Graphを単一instanceで開く |
| `Alt+T` | Threadsを単一instanceで開く |
| `Alt+S` | Settingsを単一instanceで開く |
| `Alt+L` | Legalを単一instanceで開く |
| `Alt+H` | Main内Help information surfaceを開く。第7 top-level Windowを作らない |

`Alt+Space`、`Alt+F4`などOS所有の組合せをproduct routeが横取りしない。manifest外の `Alt+key` は
action 0とし、route、focus owner、設定、表示値を変更しない。

### 2. Window別Tab列

表記順がforward `Tab`、逆順が `Shift+Tab` である。非表示controlおよび無効controlはfocus列へ
入れず、その前後を直結する。状態により存在する `StatusPrimary`、ページ操作、rowは、表中の位置へ
挿入し、同じfixtureでは列と初期focusが毎回一致しなければならない。drag領域と装飾iconはfocus不可とする。

| Window | 初期focus | exact forward Tab列 |
| --- | --- | --- |
| Main | `nav.Monitor` | `nav.Monitor→nav.Trends→nav.Threads→nav.Settings→nav.Legal→nav.Help→action.Refresh→action.StatusPrimary(if present)→window.Minimize→window.Close` |
| Setup | 現在stepの最初の入力、入力がなければ最初のaction | `nav.Monitor→nav.Trends→nav.Threads→nav.Settings→nav.Legal→nav.Help→profile→ssh.User→ssh.HostOrAlias→ssh.AliasSelector→action.StartForward→action.CheckApi→action.StartAuth→action.CheckAuth→action.Back→action.Continue→action.Cancel→window.Minimize→window.Close` |
| Settings | `settings.Language` | `nav.Monitor→nav.Trends→nav.Threads→nav.Settings→nav.Legal→nav.Help→settings.Language→settings.TimeZone→action.ConnectionRecheck→action.Authenticate→action.ReopenSetup→action.Legal→action.Help→action.Save→action.Cancel→window.Minimize→window.Close` |
| Graph | `graph.Period` | `nav.Monitor→nav.Trends→nav.Threads→nav.Settings→nav.Legal→nav.Help→graph.Period→graph.Metric→graph.Remaining→graph.LUNA→graph.TERRA→graph.SOL→page.Previous(if present)→page.Next(if present)→action.Back→window.Minimize→window.MaximizeRestore→window.Close(title)` |
| Threads | `action.Refresh` | `nav.Monitor→nav.Trends→nav.Threads→nav.Settings→nav.Legal→nav.Help→action.Refresh→page.Previous(if present)→page.Next(if present)→thread.Row1..RowN→action.Back→window.Minimize→window.Close(title)` |
| Legal | `legal.Chapter` | `nav.Monitor→nav.Trends→nav.Threads→nav.Settings→nav.Legal→nav.Help→legal.Chapter→page.Previous(if present)→page.Next(if present)→action.Back→window.Minimize→window.Close` |

### Graph/Threads固有のBack/Close（RC-083）

GraphとThreadsは共通navigationの省略だけではなく、各surface自身にvisible `action.Back`とtitle
`window.Close(title)`を持つ。両controlは同一viewport内のbounded `bounds`とUIA `AutomationId`を持ち、
forward/逆Tab列と`Enter`/`Escape`の対象を別々にmanifest化する。
ここでいうtitle Closeは埋め込みtitle領域のcontrolであり、native title barやOSの`Alt+F4`所有権を置換しない。

| surface | Back | title Close | 成功時 | 保持・禁止 |
| --- | --- | --- | --- | --- |
| Graph | `graph.action.back` | `graph.window.close` | 利用者起因の1 actionで既存Mainへ戻り、Graph singletonを閉じる | selection/page/metric/toggle/plot、Main last-good、DB、settingsを変更しない。EscapeはBack相当、window CloseはClose相当 |
| Threads | `threads.action.back` | `threads.window.close` | 利用者起因の1 actionで既存Mainへ戻り、Threads singletonを閉じる | selection/page/refresh result、Main last-good、DB、settingsを変更しない。EscapeはBack相当、window CloseはClose相当 |

BackとCloseは同じrouteへ戻っても同じ意味の文字列へ丸めず、`action`（Back）とtitle control
（Close）のUIA name/shortcut/eventを分離する。primary actionの実行中は再keydownとkeyupで追加close、
新しいchild、DB/settings書込みを作らない。個別surfaceのbounds、UIA tree、route traceは共通
`WIN-M-012`へhard joinするが、共通行だけでGraph/Threadsの個別導線を代替できない。

### Main HWND内Helpのfocus scope（RC-084/085）

Helpは`Main.help`としてMain HWND内に描画される独立focus scopeであり、top-level inventoryの6件や
runtime HWND集合へ追加しない。scopeが開いている間、Main/呼出元childのscopeへTabが抜けず、Helpの
semantic controlだけを巡回する。詳細なchapter/page、caller restore、追加HWND 0、AutomationProperties
は`UX-20260823-HELP-FOCUS-001`をjoin先とする。

Helpのforward列は
`help.Chapter→page.Previous(if present)→page.Next(if present)→action.Back→action.Close`、
`Shift+Tab`はこの完全逆順である。初期focusは`help.Chapter`、ページ操作が存在しない場合も
`action.Back→action.Close`を省略しない。`Enter`はfocused actionを1回だけ行い、`Escape`、Back、
CloseはいずれもHelpを閉じて保存済みのcaller route/HWND/focusへ一度だけ戻す。Help close後の初期focusを
推測で`nav.Monitor`へ置き換えず、entry manifestのcallerをrestoreする。

各Help controlは`AutomationId`（locale不変）、catalog由来の非空`Name`/`HelpText`/shortcutを持ち、
focus indicatorは通常・高コントラストとも2 logical px以上、隣接色差3:1以上、clip 0である。

### Help re-entryとdouble-close（RC-100）

Helpは`Closed → Opening(entry token) → Open(first caller tuple, HelpScopeGeneration) → Closing(HelpCloseToken) →
Closed`の一つのscopeだけを持つ。同一callerからの再entryは既存scopeへjoinし、異なるcallerからの再entryも
first caller tuple、route、focusを上書きせず、追加HWND、scope、subscription、foreground移動を0とする。最初に
CASを取得したBack/Close/Escapeだけがclose actionとcaller restoreを行い、keyup、key-repeat、同時double-close、
遅延callbackはno-opである。entry/closeの各境界で`(PID, PID start token, HWND, WindowInstanceGeneration,
HelpScopeGeneration)`を再検証し、不一致はMain pre-Help route/`nav.Help`への一回限りfallback（Main生存時）または
ShuttingDown時のrestore 0へ送る。UIA route、scope/HWND/subscription数、focus owner、action count、caller tuple hashを
同一event traceから採取する。

### Setup profile×step applicabilityとstale completion（RC-101、RC-110対象）

Setup行62のTab列は全controlの上限集合であり、実際のTab列は`connectionProfile`（none、wsl、sshConfigAlias）と
現在stepの直積から、visibleかつenabledなcontrolだけを投影する。`profile`はprofile selector、`wsl.Distribution`は
WSLの`connectionSelector`、`ssh.AliasSelector`はliteral Host alias、`ssh.User`/`ssh.HostOrAlias`はone-session raw
recoveryだけの一時入力である。invalid selectorでは該当Continue/primaryをdisabledにし、disabled controlはTab列へ入れない。

| profile | step | visibleかつenabledなTab列 | 初期focus | Back | Cancel / title Close | busy中focus | stale completion後focus |
| --- | --- | --- | --- | --- | --- | --- | --- |
| none | profile | `profile`, `action.Cancel`, `window.Close` | `profile` | disabled/no-op | first-launch=Main disconnected＋Settings recovery、reopen=Settings | — | `profile` |
| WSL | profile | `profile`, `wsl.Distribution`, `action.Continue`, `action.Cancel`, `window.Close` | `profile` | disabled/no-op | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`profile` |
| WSL | prepare | `wsl.Distribution`, `action.StartForward`, `action.Back`, `action.Cancel`, `window.Close` | `wsl.Distribution` | `profile` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`profile` |
| WSL | health | `action.CheckApi`, `action.Back`, `action.Cancel`, `window.Close` | `action.CheckApi` | `action.StartForward` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`action.StartForward` |
| WSL | auth-start | `action.StartAuth`, `action.Back`, `action.Cancel`, `window.Close` | `action.StartAuth` | `action.CheckApi` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`action.CheckApi` |
| WSL | auth-check | `action.CheckAuth`, `action.Back`, `action.Cancel`, `window.Close` | `action.CheckAuth` | `action.StartAuth` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`action.StartAuth` |
| WSL | ready | `action.Continue`, `action.Back`, `action.Cancel`, `window.Close` | `action.Continue` | `action.CheckAuth` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | — | `action.Continue` |
| SSH alias | profile | `profile`, `ssh.AliasSelector`, `action.Continue`, `action.Cancel`, `window.Close` | `profile` | disabled/no-op | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`profile` |
| SSH alias | prepare | `ssh.AliasSelector`, `action.StartForward`, `action.Back`, `action.Cancel`, `window.Close` | `ssh.AliasSelector` | `profile` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`profile` |
| SSH alias | health | `action.CheckApi`, `action.Back`, `action.Cancel`, `window.Close` | `action.CheckApi` | `action.StartForward` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`action.StartForward` |
| SSH alias | auth-start | `action.StartAuth`, `action.Back`, `action.Cancel`, `window.Close` | `action.StartAuth` | `action.CheckApi` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`action.CheckApi` |
| SSH alias | auth-check | `action.CheckAuth`, `action.Back`, `action.Cancel`, `window.Close` | `action.CheckAuth` | `action.StartAuth` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | `action.Cancel` | cancel/close=`settings.Language`、Back=`action.StartAuth` |
| SSH alias | ready | `action.Continue`, `action.Back`, `action.Cancel`, `window.Close` | `action.Continue` | `action.CheckAuth` | first-launch=Main disconnected＋Settings recovery、reopen=Settings | — | `action.Continue` |
| one-session raw recovery | recovery | `ssh.User`, `ssh.HostOrAlias`, `action.StartForward`, `action.Back`, `action.Cancel`, `window.Close` | `ssh.User` | `settings.Language` | Settings recoveryへ戻り、raw入力を破棄（durable保存0） | `action.Cancel` | `settings.Language` |

上表のCancelとtitle Closeは同じ一回の`SetupCloseToken`を消費し、first-launchでは製品が生成した一時childだけを
reapしてSettings recoveryを残し、reopen-from-Settingsでは未保存candidateを捨ててSettingsへ戻る。first stepのBackは
no-op、intermediate stepのBackは上表の前stepへ一度だけ戻る。busy中は`action.Cancel`だけをenabledにし、Cancel/Closeで
`SetupOperationGeneration`をinvalidateする。遅延completionが後から到着してもroute advance、settings bytes、process再公開、
focusの奪取を0とし、Cancel/Closeなら`settings.Language`、Backなら表の前step初期focusを保持する。全遷移は
`(Setup PID, PID start token, Setup HWND, WindowInstanceGeneration, SetupOperationGeneration, owned child PID/start token)`を
照合し、旧generationのイベントはno-opにする。

### 3. EnterとEscape

- `Enter` keydownはfocused actionをexactly 1回だけ実行する。keyup、key repeat、busy中の再keydownは
  追加actionを作らない。入力control内ではcontrol標準の確定だけを行い、別routeを暗黙起動しない。
- Mainの `Escape` はprocessを終了せず、popup/keyboard cueがあればそれだけを閉じて `nav.Monitor`へ戻す。
- Setupの `Escape` はvisible `Cancel` と同じで、Settingsから再表示した場合は未保存入力を捨ててSettingsへ、
  初回起動なら製品が生成した一時processだけをcancel/reapしてMain disconnected＋Settings recoveryを残す。
  title Closeも同じ`SetupCloseToken`を使い、設定完了を捏造しない。
- Settingsの `Escape` はvisible `Cancel` と同じで未保存変更を捨て、呼出元へ戻る。
- Graph、Threads、Legalの `Escape` はその単一instanceを閉じてMainへ戻り、Mainのsnapshot、選択中の
  account非依存Graph control、設定、DBを変更しない。
- `Escape` keyupとkey repeatは追加route/actionを作らない。

### 4. focus visual

- keyboard focus indicatorはcontrol perimeterに対して2 logical pixel以上の連続した面積を持ち、
  indicatorと直に接する色の変化は3:1以上とする。通常、hover、pressed、disabled、busy、error、
  high-contrast、DPI `96/144/192`とtext scale `100/125/150/175/200/225%`の直積で測定する。
- indicatorはcontentや文字を隠さず、layout sizeを変化させず、viewport端でclipしない。
- 高コントラストではOS system colorを尊重し、custom themeがOS focus visualを不可視化しない。
- focus ownerは常に1個。Window非表示、close、route changeで旧focusを破棄し、次のcanonical初期focusを設定する。
- background poll、error表示、子Windowの既存instance更新はfocus/cursorを変更しない。利用者がroute/CTAを
  実行した場合だけ、表の次focusへ移せる。

### locale/UIAとstate dimension（RC-085/086/088）

keyboard/UIA topologyは次の10 catalog IDとunknown fallbackで同一にする。
`[ja,en,zh-Hans,ko,es,fr,de,pt,it,ru]`をsupported setとし、unknown、不正、`C`、`POSIX`は
resolved locale=`en`へ一度だけ解決する。各surface（Main、Setup、Settings、Graph、Threads、Legal、
Main内Help）の同じ`AutomationId`、focus owner、Tab列、Shift+Tab逆順、Alt chord、Enter/Escape action、
route、action countをlocaleで変更しない。

UIA manifestの各controlは`AutomationId`、catalog由来の非空`Name`、`HelpText`/`Description`、
`AcceleratorKey`、`bounds`を持つ。locale別に翻訳文が変わってもID/topology/actionは変えず、Name/descriptionの
欠落、未解決key、文字clip、重複IDは0とする。Setupはtitle、profile、step、primary、Back、Cancel、error
keyを単一resolved localeからjoinし、locale混在を0とする。

state matrixは`UX-20260823-FULL-STATE-001`を正本とし、keyboard evidenceの適用集合を
`initializing, auth_required, normal, quota_warning, quota_danger, reset_warning, zero, full,
api_error, transport_error, status_invalid, details_invalid, history_error, thread_error, db_error,
stale, no_history`とする。未適用stateを通常画像で代用せず、surface/stateのN/Aをmanifestへ明示する。

2 logical pixel相当の面積と3:1の変化は、W3C WCAG 2.2 Focus Appearanceの測定法を、製品の
回帰判定を曖昧にしない下限として採用する。Windows固有のfocus可視性とキーボード操作はMicrosoftの
Windows app accessibility/visual feedback guidanceへ整合させる。これは規格認証済みという主張ではなく、
後段の実artifactを測るための契約である。

- W3C: <https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html>
- Microsoft: <https://learn.microsoft.com/en-us/windows/apps/develop/accessibility>
- Microsoft: <https://learn.microsoft.com/en-us/windows/apps/develop/input/guidelines-for-visualfeedback>

## X版との関係

X版のdata、状態、期間、Graph、Threads意味論は変更しない。Windows版で追加するのはWindowsの
navigation、keyboard、focus表現であり、shortcutやfocus移動を理由に取得値、表示所有者、並び順、
last-good、DBへ変更を加えない。

## 影響要求

`RC-083`, `RC-084`, `RC-085`, `RC-086`, `RC-088`, `RC-099`, `RC-100`, `RC-101`, `WIN-E-001`, `WIN-E-010..016`,
`WIN-F-001..007`, `WIN-G-013..016`, `WIN-K-002..004`, `WIN-K-008`, `WIN-K-010..012`, `WIN-K-015`, `WIN-M-002..010`,
`WIN-M-004`, `WIN-M-011`, `WIN-M-013..015`, `WIN-M-018`, `WIN-M-025`, `WIN-M-028`, `WIN-M-030`, `GLOBAL:AUD-020`。

## 非スクロール影響

focusされたcontrol、primary action、Back、Closeは各canonical viewport内に完全表示し、Tabによって
root scroll、内部scroll、画面外controlへ移動しない。Graph/Threadsの個別Backとtitle Close、Main内
HelpのBack/Closeも同じviewportへ固定する。長い集合は既定のページング/章切替を使い、ページ変更後は
現在pageのcanonical初期focusへ移す。Helpの追加HWNDは0である。

## 証拠計画

実装後、同一release manifestとartifact SHAに対して、隔離したWindows hostで実keydown/keyupを入力し、
UI Automationのfocus owner、route、action count、bounds、accessible name、cursor座標、HWND集合を記録する。
7 surface projection（6 top-level surface＋Main内Help）×17 state×10 locale＋unknown→en×DPI `[96,144,192]`×
text scale `[100,125,150,175,200,225]%`×通常/高コントラスト×full/reduced motion×supported sizeのmatrixで、
Tab/Shift+Tab/Enter/Escape/Altとmanifest外入力を実行する。DPIとtext scaleを一つのpercent軸へ丸めず、
各Window generationでDPI適用回数=1、text-scale適用回数=1、二重scale=0を記録する。
Graph/ThreadsのBack/Close、Helpのcaller restore、UIA name/description/AutomationId、semantic bounds、
focus ringのfresh画像とpixel/contrast測定rawを結合し、実装者と異なる担当が三値判定する。
Setupはnone/WSL/SSH alias/one-session raw recovery × 全step × 初回/reopen × Back/Cancel/Close × busy/stale completionを
別caseにし、visible+enabled control集合、初期focus、前step、`SetupCloseToken`、`SetupOperationGeneration`、
owned child PID/start token、settings bytes、route/action/focus traceを同一artifact SHAへ結合する。
マウスevent countとcursor deltaは0でなければFAILとする。

## 未確定

要求・route・閾値は本Decisionで確定した。実装、artifact SHA、物理Windows hostでの操作ログ、fresh画像、
contrast測定、独立製品評価は未取得であり、製品状態は`PRODUCT_PENDING`である。X版のdata/状態/意味論を
変更する判断は含めず、Helpを第7 Windowへする解釈、locale別のTab差、未登録stateの推測は未採用である。

# UX Decision: Non-scroll primary surfaces

Decision ID: `UX-20260822-UX-002`

状態: `EXTRACTION_INCOMPLETE / PRODUCT_PENDING`

## 利用者の課題

長い画面で残量、状態、復旧、戻る、閉じるがviewport外へ消え、利用者がスクロール位置を探す問題を
許容しない。

## 目的

主要情報と主要操作を一つのviewportで判断でき、長い集合は内容を欠落・重複させず段階表示で到達できる
ことを目的とする。

## 代替案と棄却理由

1. Window全体を縦scrollする案は、状態と戻る/閉じるが消えるため棄却する。
2. 内部ListViewだけをscrollする案は、4件目以降や長文の到達性を見た目で隠し、比較とkeyboard導線を
   不安定にするため棄却する。
3. page/章/選択詳細へ意味単位で分割し、固定操作を同一viewportへ残す案を採用する。

## 採用案（決定）

登録top-level surface inventoryはMain、Setup、Settings、Graph、Threads、Legalの正確な6個に固定する。
runtime open HWNDはMain=1＋open child subset=0..5、合計1..6で、各childはsingletonとする。HelpはMain内
900×480 logicalの情報surfaceで、additional HWND=0である。5 childを全て開いた時だけruntime HWNDが6となり、
inventory件数をruntimeの常時個数へ固定しない。Main、Setup、Settings、Graph、Threads、Legalの各surfaceは、
主要情報・主要操作・戻る・閉じるへ到達するためのページ全体スクロールを持たない。長い一覧・本文は、
ページング、章切替、選択詳細、折りたたみへ分割する。rootまたは内部ScrollViewerだけで高さ不足を隠す
旧要求は明示的にsupersedeし、不合格とする。

## geometry/DPI/topology authority（UX-002の抽出追加）

このDecisionは非スクロール導線だけでなく、その導線を測定するWindow geometryの境界も固定する。
値は実装現況や既存画像から昇格せず、抽出中の要求正本として扱う。

| surface | registered top-level surface | runtime open HWND | logical client initial | logical client min | logical client max | resize |
| --- | --- | --- | --- | --- | --- | --- |
| Main | yes | 1 | 900×480 | 900×480 | 900×480 | fixed |
| Setup | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed |
| Settings | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed |
| Graph | yes | 0..1 (singleton) | 940×640 | 700×480 | unbounded | resizable |
| Threads | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed |
| Legal | yes | 0..1 (singleton) | 900×480 | 900×480 | 900×480 | fixed |
| Main内 Help | no (owner=Main) | 0 additional | Main client 900×480内 | Main client 900×480内 | Main client 900×480内 | Mainに従う |

`700×480`はGraph minimumだけであり、Main/Setup/Settings/Threads/Legalのsupported work areaは
少なくとも`900×480 logical`とする。Graphのsupported work-area minimumは`700×480 logical`
とする。各境界未満はsupported matrix外として`unsupported_scope` manifestへ記録し、font縮小、
clip、scroll、PASS捏造へ逃げない。これは新しいproduct failure classや第7 Windowを追加する
根拠ではない。

surface/monitor/DPI/sizeのsupported predicateは次のANDで固定する。

```text
supported = client_threshold AND frame_fit
client_threshold(fixed Main/Setup/Settings/Threads/Legal) = logical >= 900×480
client_threshold(Graph) = logical >= 700×480
frame_fit = DPI変換後のDWM.visible_frame全体が対象MONITORINFO.rcWork内へ完全包含
```

`frame_fit`はphysical rectのleft/top/right/bottomを全て判定する。logical client thresholdは必要条件で
あり、thresholdだけでsupportedとはしない。thresholdとframe-fitのANDがsupportedの十分条件である。

DPIは`GetDpiForWindow`相当のOS-reported integer `dpi`、`scale=dpi/96`を正本とする。
`96/144/192`（100/150/200%）は必須fixtureだがdomain全体を限定しない。positive sizeの丸めは
`floor(logical*dpi/96+0.5)`である。evidence fieldは`logical_client`、`physical_client`、
DWM visible frame boundsの`visible_frame`、`MONITORINFO.rcWork`の`work_area`を別に持ち、origin、
size、unitを明示する。

Main freshは起動直前foreground Window monitor（無効時primary）、child初回openはowner monitor
へ置く。fresh Main/初回user-openだけをcenterする。reopenは、現在のいずれかの`rcWork`へvisible frame
全体が包含される時だけlast stable OS rectを復元する。monitor除去/解像度縮小で無効なら、Mainは
foreground monitor→primary、childは有効owner monitor→primaryの順で一度だけ`topology_recovery center`
し、reasonをraw記録する。どのmonitorもsupported predicateを満たさない場合は`unsupported_scope`
として記録し、PASSを捏造しない。centerは`MONITORINFO.rcWork`とDWM visible frame boundsに対し
`origin=work_origin+floor((work_size-frame_size)/2)`を一度だけ計算し、各軸のdouble-coordinate
residualを`abs((2*frame_origin+frame_size)-(2*work_origin+work_size))<=1`とする。通常のtimer、poll、
reopen、drag後recenter、`Window.Position` loop、cursor合成は0件とし、無効reopen時の一度だけの
`topology_recovery center`だけを例外とする。

登録された6 surfaceはnative moveを1 gestureにつき1回だけ行い、control hit=0とする。全surfaceは
minimize/closeを持ち、native resize/maximize/restoreはGraphだけとする。Graph maximizeはcurrent
monitor work area、fullscreenはproduct actionにしない。same-DPI crossing、different-DPI crossing、
negative/nonzero origin、taskbar-shrunk work areaを必須topology cellとし、対象monitorはsurfaceの
supported boundary以上にする。

この追加の目的は、主要操作を同一viewportへ置いたまま、client/frame/work-area単位、DPI変更、
monitor移動、native入力の受入境界を曖昧にしないことである。理由は、Graph minimumをfixed
Windowへ誤適用したり、frame-fitを欠いた復元や実装のcenter処理を正本化したりすると、非スクロール判定
そのものが異なるgeometryで評価されるためである。X版の値、期間、全item/paragraph、Graph/Threads順序、
状態、所有権、失敗時保持は不変であり、ここで変更するのはWindowsのviewport/OS表現だけである。
証拠計画は、同一artifact SHAのfresh processで全surface×state×supported size×DPI×topologyを
取得し、OS raw rect/DPI、round式、center residual、runtime HWND subset/singleton、frame-fit、
reopen fallback/reason、move/resize/control-hit、scroll input、page/paragraph hash、Back/Close boundsを
保存し、実装担当とは別の評価者が独立再計算する。
証拠未取得のため製品状態は`PRODUCT_PENDING`のままとする。

## 理由

- 監視者が残量、接続状態、リセット、更新、詳細入口を一目で判断するため。
- Windowsアプリとして、操作の所在を固定し、スクロール位置によって戻る/閉じる/エラー復旧が
  消える状態を防ぐため。
- 文字を極端に縮小し、カードを詰め込む場当たり設計を防ぐため。
- X版の長文/一覧表示をそのまま複製するのではなく、データ意味論と所有権を維持しつつ、Windowsの
  viewportとキーボード操作へ適合させるため。

## 固定条件

- Main: 残量、状態、期間ゲージ、実行中概要、モデル利用量、更新、メニューを一画面で到達可能。
- Setup: 現在手順、入力/結果、次へ、戻る、キャンセルを一画面で到達可能。
- Settings: 編集、保存、取消、復旧、戻るを一画面で到達可能。
- Graph: 期間、metric、series、plot、現在値を同時に認識可能。popup開閉でplotを押し下げない。
- Threads: 0件は空状態、1件は詳細card、2〜3件は比較cardを同一viewportへ置く。4件以上は
  canonical presentation orderを3件ずつのpageへ分け、page数は`ceil(thread_count/3)`、各IDの
  出現回数は全page合計で1とする。親が前pageにあるchildも親ID/親title/roleを失わず、選択詳細から
  親子関係を確認できる。更新、現在page/総page、前/次、戻る、閉じるを常時表示する。
- Graph period: 0件は「履歴なし」で選択不能、1〜4件は同一popup page、5件以上はcanonical period
  orderを4件ずつへ分け、page数は`ceil(period_count/4)`とする。popup pageを変えてもplot、toggle、
  metric、選択済みcanonical period IDを移動・消去しない。
- Legal: `GPL/無保証`、第三者、font、API schema、dependency/runtime、distributionのpre-authored章を
  1 pageずつ表示する。章順と全paragraph hashをnotice manifestへ固定し、各paragraphは全page合計で
  ちょうど1回出現する。現在章/総章、前/次、戻る、閉じるを常時表示する。
- Help: server/API silent起動、recorder daemon、WSL、remote SSH、API確認、認証、設定/復旧、
  update/uninstall、診断情報の9章をpre-authored pageとして固定し、全paragraphを重複・欠落なく
  page manifestへ結合する。現在章/総章、前/次、戻る、閉じるを常時表示する。
- Graph: `graph.action.back`とtitle `graph.window.close`を同じviewportへ常時表示し、押下は既存Mainへ
  戻る1 actionとする。period/page/metric/toggle/plot、last-good、DB、settingsは保持する。
- Threads: `threads.action.back`とtitle `threads.window.close`を同じviewportへ常時表示し、押下は既存Mainへ
  戻る1 actionとする。selection/page/refresh result、last-good、DB、settingsは保持する。各surfaceのBack/Close
  boundsとUIA treeは共通navigationではなくGraph/Threads個別manifestへ登録する。
- Main内Help: Main HWNDの独立focus scopeとしてchapter/page、Back、Closeを同じviewportへ表示し、
  forward=`help.Chapter→page.Previous(if)→page.Next(if)→action.Back→action.Close`、逆順は完全逆順とする。
  Enterはfocused actionを1回、Escape/Back/Closeはentry時のcaller route/HWND/focusへ1回だけrestoreする。
  Helpのadditional HWNDは0であり、Help scope外へTabを出さない。
- 10言語のいずれかでsurfaceごとのsupported minimum viewportへpre-authored pageが収まらない場合、font縮小やscrollで逃げず、
  意味paragraph境界で翻訳catalogのpageを分割する。runtimeで文字数だけを使って途中切断しない。
- locale dimensionは`[ja,en,zh-Hans,ko,es,fr,de,pt,it,ru]`とunknown→`en` fallbackを固定する。
  各surface＋Main内Helpのsemantic item/paragraph IDは全pageでちょうど1回、locale別text hashとpage assignmentを
  manifestへ保存し、missing/extra/duplicate/clip=0とする。未翻訳key、混在locale、文字の途中切断をpage数削減の
  根拠にしない。Setupはtitle/profile/step/primary/Back/Cancel/error keyを一つのresolved localeからjoinする。
- state dimensionは`UX-20260823-FULL-STATE-001`の17状態（initializing、auth_required、normal、quota_warning、
  quota_danger、reset_warning、zero、full、api_error、transport_error、status_invalid、details_invalid、
  history_error、thread_error、db_error、stale、no_history）を適用し、通常画像で未定義状態を代用しない。
- fixed Windowは900×480 logical、Graphは940×640 initial/700×480 minimumで、高DPI、複数モニタ、
  locale、長文、エラー、空データでも上記を崩さない。700×480をfixed Windowの要求へ適用しない。

## 受入oracle（実装後）

登録された6 surfaceのruntime open HWND（Main=1＋child subset 0..5、合計1..6、child singleton）と
Main内Help additional HWND=0について、状態×supported size×locale×DPI×topology×keyboard matrixで
page scroll inputなしに主要操作へ到達できる。固定surfaceのsupported boundary未満は`unsupported_scope`
manifestへ記録し、supported PASSへ混ぜない。client thresholdを満たしてもDWM visible frame全体が
target `MONITORINFO.rcWork`内へ入らない場合はunsupportedとする。reopenの無効rect回復は一度だけの
topology_recovery、reason raw、timer/poll/drag後recenter=0を確認する。
主操作がScrollViewer下にある、内部スクロールを前提にする、固定操作が消える、文字がclipする、
代替ページへ到達できない場合はFAIL。実装前は証拠未取得としてHOLDにする。

一覧/本文の完全性はsource inventoryのID/paragraph hashと全pageのID/paragraph hashをjoinし、
`missing=0`、`extra=0`、`duplicate=0`で判定する。最初と最後のpageでは無効方向の操作をdisabledにし、
押下してもpage/stateを変更しない。中間pageの前→次および次→前は同じ選択IDとpageへ戻る。

## 影響要求

`RC-083`, `RC-084`, `RC-085`, `RC-086`, `RC-087`, `RC-088`, `WIN-C-017..019`,
`WIN-D-002..004`, `WIN-E-001..012`, `WIN-F-001..007`, `WIN-G-013..016`,
`WIN-K-013..015`, `WIN-M-003..010`, `WIN-M-025..029`。

`DESIGN.md` の旧ScrollViewer導線はこのDecisionに従って仕様上廃止し、ページング、選択詳細、
折りたたみへ置換した。実装コードと実画像がこの正本へ一致したかは後段の製品受入で判定する。

## X版との関係

X版の値、期間、全item、全paragraph、Graph/Threadsの順序と意味論は維持する。Windows版では
到達方法だけをpage/章/選択詳細へ変更し、scroll削除を理由に情報を削除・要約・再順序化しない。

## 非スクロール影響

本Decision自体が非スクロール正本である。登録された6 surfaceのprimary情報、primary action、Back、
Closeを同一viewportへ固定し、Graph/Threadsの個別Back・title CloseとHelpのscope内Back・Closeを共通行の
省略で代替しない。HelpはMain内900×480 surfaceとして同じ条件を適用する。runtime open HWNDの個数は
Main=1＋open child subset 0..5であり、Helpの追加HWNDは0とする。root/internal ScrollViewerだけを
到達手段にする旧要求は本Decisionがsupersedeする。

## 証拠計画

同一release artifact SHAで、登録された6 surfaceのruntime subset＋Main内Help×17 state×10 locale＋unknown→en×
canonical/minimum size×DPI×topology×keyboard matrixのfresh画像、UIA bounds、AutomationId/name/description、
logical/physical/frame/work-area rect、DPI、center residual、HWND subset/singleton、native move/resize/control-hit、
reopen fallback/reason、`surface/locale/resolved_locale/page_id/semantic_id/text_hash/page_index/page_count/bounds/clip`
anchor、page item/paragraph hash、route/action logを取得する。実装者と異なる担当がmissing/extra/duplicate、
frame-fit、root/internal-scroll依存、clip/overlap、Back/Close bounds、unsupported_scope境界を再計算する。

## 未確定

要求と分割規則は確定した。実artifact、fresh画像、操作log、独立製品判定は未取得であり、製品状態は
`PRODUCT_PENDING`である。X版のitem/paragraph/state/意味論を変更する判断、root/internal scrollへの回帰、
localeごとのTab/route差、未登録stateの推測は未採用である。

## DESIGN.mdとの整合範囲（抽出候補記録済み・独立監査／実装証拠待ち）

`DESIGN.md` のThreads全件表示とGraph期間選択popupは、`UX-20260822-UX-002`を正本として
ページング、選択詳細、折りたたみ等へ変更し、内部ScrollViewerだけを到達手段にする旧仕様を禁止した。
この仕様整合案は独立抽出監査または実装受入を意味しない。canonical active snapshotの
取得・dedup・presentation意味論、表示順・role・tree rail、固定Window/viewport寸法、row/popup寸法、
文字サイズ・clip・安全域など既存のデータ意味論とレイアウト契約は維持し、スクロール導線だけを
変更対象とする。実装、画面評価、独立評価の証拠は未取得であり、製品状態は`PRODUCT_PENDING`のままとする。

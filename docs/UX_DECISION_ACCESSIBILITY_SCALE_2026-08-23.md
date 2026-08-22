# UX Decision: Windows text scaling and assistive status

Decision ID: `UX-20260823-ACCESSIBILITY-SCALE-001`

状態: `EXTRACTION_DECISION_RECORDED / PRODUCT_PENDING`

## 利用者の課題

DPI scalingだけを確認しても、Windowsの「テキストのサイズ」設定は別の入力である。custom control、固定高、
独自描画を使う画面では、文字だけが拡大されてclip、重なり、操作の押し出しが起こり得る。また、poll、接続、
保存、installerの結果が視覚だけで更新されると、screen reader利用者は変化を知るためにfocusを移動し続ける
必要がある。

## 目的

WindowsのOS text scaleをDPIと別入力として扱い、100〜225%でも情報・操作・意味を削除せず、
既存の非スクロール、表示所有権、focus/cursor非奪取を維持する。同時に、意味のある状態変化を
画面へfocus移動させず支援技術へ一度だけ通知できる受入契約を作る。

## 外部基準と適用境界

- Microsoftの
  [Text scaling](https://learn.microsoft.com/windows/apps/design/input/text-scaling) は、Windowsの文字scaleを
  100%から225%の独立設定として扱い、custom controlや固定高を持つ実装ではcontainerのresize/reflowが
  必要になり得ると説明している。
- Microsoftの
  [Accessible text requirements](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessible-text-requirements)
  は、Windowsのtext size、display scale、MagnifierとUI Automation text semanticsを別々に確認するよう求める。
- W3C WCAG 2.2の
  [Resize Text](https://www.w3.org/WAI/WCAG22/Understanding/resize-text.html) と
  [Status Messages](https://www.w3.org/TR/WCAG22/#status-messages) は、200%で内容・機能を失わないことと、
  focusを移さず状態変化を支援技術へ通知できることを検査可能な補助基準として使う。

これらはWeb実装や特定toolkitの採用を要求するものではない。本製品ではWindows desktopの実挙動、UIA tree、
画像、操作結果をoracleとし、外部文書の記載だけを製品PASSにしない。

## 代替案と各棄却理由

| 代替案 | 判定と理由 |
| --- | --- |
| DPI試験だけでtext scaleを代用 | OS上で独立設定であり、固定高/custom controlのclipを検出できないため棄却 |
| text scale時だけ文字を縮小またはroot scrollを追加 | 利用者設定を無効化し、既存の非スクロール・全機能到達要求を破るため棄却 |
| 100%と200%だけを確認 | Windowsの範囲上端225%と中間reflow境界を覆わないため棄却 |
| event-driven reflowと既存page/detailへ意味単位で再配置し、UIA semantic notificationをdedup | 採用。protocol値・所有者・focus・cursorを変えず全範囲を観測可能にできる |

## 採用案

### 1. text scaleをDPIと分離する

`text_scale_percent`はOSから得る整数`100..225`とし、display DPI、monitor scale、window logical sizeとは
別fieldで保持する。必須fixtureは`100,125,150,175,200,225`であり、`96/144/192 DPI`の各fixtureと直積にする。

- physical geometryは既存のinteger DPI式を一度だけ適用する。
- font/content scaleは`text_scale_percent / 100`を一度だけ適用する。
- DPIをtext scaleへ、text scaleをDPIへ代入せず、二重scaleしない。
- OS setting changeはevent-drivenに同一Window generationへ一度適用し、timer/pollによるre-layout loopを作らない。
- unsupported、欠測、範囲外のscaleを100へ黙って丸めず、直前のvalid layoutを保持してbounded accessibility errorを
  所有者へ出す。

### 2. 非スクロールと意味保持

100..225%の全fixtureで、各surfaceのprimary value、現在状態、primary CTA、Back、Close、keyboard focus、
UIA Name/Descriptionは同じsupported viewportから到達可能でなければならない。root/internal scroll、文字縮小、
clip、overlap、文字途中切断、操作の画面外配置で逃げない。

長い内容は既存のpage/chapter/step/detail契約を使い、`semantic_item_id`または`paragraph_id`をページ間で
`missing=0,extra=0,duplicate=0`にする。text scale変更前後でprotocol値、period、選択、last-good、設定、DB、
process ownerを変更しない。Mainで一画面に収まらない補助領域は、RemainingQuota、現在Status、primary recovery、
navigation、Window controlsを常時残したまま、登録済みpriorityに従う追加page/detailへ移す。情報を削除したり、
同じ事実を別カードへ複製したりしない。

全表示文字はUIAからfull semantic textを取得できる。視覚上のellipsisを使用できるのは、同じviewportの
明示操作でfull textへ到達でき、UIA Name/Descriptionが省略されず、owner tableへ登録済みの場合だけである。
主値、state Cause/Impact/Recovery、操作label、期間両端、法的本文にはellipsisを使わない。

### 3. focusとdynamic status notification

background poll、tunnel監視、daemon状態、設定保存、clipboard、installer progress/resultはfocus、foreground、
cursorを変更しない。意味が変わったときだけ、該当Status ownerからUI Automationのlive-regionまたは同等の
notification eventを一度送る。

| event | notification semantic | focus/route | dedup |
| --- | --- | --- | --- |
| initializing→ready | 接続・認証確認完了 | 変更0 | 同じaccepted rootは0回 |
| ready→canonical failure | Cause、Impact、primary recovery | 変更0 | 同じfailure class＋resource generationは0回 |
| failure→ready | 復旧完了 | 変更0 | 同じaccepted rootは0回 |
| Settings save success/failure | 保存結果と旧bytes保持 | Settings内focusを維持 | 同じoperation tokenは最大1回 |
| Setup/installer step change | 現在stepと次の1操作 | 現在scope内focusを維持 | 同じoperation generation＋stepは最大1回 |
| Help/Legal/Graph/Threads page change | 現在page/chapterと総数 | activated controlまたは規定初期focus | 同じpage IDは追加通知0 |

10秒pollで値が変わらない場合、notification countは0である。値だけが変わった場合は表示ownerのsemantic value
changeを最大1回通知し、毎回全画面を読み上げない。秘密、raw error、path、hostname/user、token、stderr、
内部PID/HWND/hashはName/Description/notificationへ含めない。screen reader notificationを理由に新しいWindow、
modal dialog、focus移動、音声fileを生成しない。

### 4. 直積と失敗保持

対象直積は、7 surface projection、17 visual state、19 failure class、10 locale＋unknown→en、
`text_scale_percent=[100,125,150,175,200,225]`、DPI `[96,144,192]`、normal/high-contrast、
full/reduced motion、supported monitor topology、keyboard/UIAである。全直積を画像1枚へ詰めず、各cellを
manifestの`applicable`または根拠付き`N/A`へ割り当てる。未割当cellは抽出FAILである。

scale change中にlayout、font、UIA treeのいずれかが構築できなければ、partial treeや文字だけ拡大した画面を
publishせず、直前の完全layout generationを保持する。保存、DB、REST root、process、selectionは無変更で、
同じeventを無制限retryしない。

## X版との関係

X版のquota、history、Graph、Threads、期間、値の所有権は変更しない。WindowsのOS text scaleとUIA notificationは
Windows固有の到達性契約であり、X版から値や機能を削減する根拠にも、Windows側で派生値・重複文言を追加する
根拠にも使わない。同じfixture/data generationを使い、差分はlayout projectionとassistive semanticsだけに限定する。

## 受入oracle（実装後）

同一release artifact SHAのclean Windowsで、OS-reported DPIとtext scaleを別々に採取し、各applicable cellの
fresh画像、UIA tree、text bounds、control bounds、focus、route/action、notification eventを保存する。

- displayed semantic itemの`missing/extra/duplicate=0`
- text/control `clip=0`、overlap=0、root/internal scroll input=0
- primary value/Status/primary CTA/Back/Closeのviewport外件数=0
- DPI適用回数=1、text scale適用回数=1、二重scale=0
- unchanged poll notification=0、semantic transition notification=1
- background foreground/focus/cursor delta=0
- notification/UIA内のsecret/raw/path occurrence=0
- 200%と225%の双方で内容・機能欠落=0

実装担当と異なる評価者がraw manifestから上記を再計算する。画像だけ、UIAだけ、100%だけ、DPI scalingだけを
根拠にPASSへしない。

## 影響要求

`WIN-C-014`, `WIN-C-018..019`, `WIN-G-013..016`, `WIN-L-007..010`,
`WIN-M-002..010`, `WIN-M-019..021`, `WIN-M-024..029`, `RC-085..088`, `RC-113`。

## 未確定

text scale domain、非スクロール保持、notification/dedup/focus/秘密境界、evidence式を要件として確定した。
実artifact、Windows text settingのruntime反映、screen reader/UIA notification raw、fresh画像、独立製品判定は
未取得であり、製品状態は`PRODUCT_PENDING`である。

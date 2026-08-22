# UX Decision: Graph label parity

Decision ID: `UX-20260822-GRAPH-001`

状態: `EXTRACTION_DECISION_RECORDED / EVIDENCE_PENDING`

## 利用者の課題と目的

Windows版がX版と異なるlabel、単位、owner、系列順を表示すると、同じデータでも別の意味に見える。
platform表現差とデータ意味差を分離し、登録した一対一mapping以外の変更を検出することを目的とする。

## 代替案と棄却理由

1. 全文字列のbyte一致は、正当なlocale/Windows表現差まで禁止するため棄却する。
2. 実装者の目視で「同じ意味」と判断する案は、言い換え・省略・重複を再現可能に検出できないため棄却する。
3. canonical key、role、unit、order、owner、action resultを一致させ、登録mappingだけを許す案を採用する。

## 決定

X版とWindows版のグラフ文字列は、同一localeでは意味単位を一対一に対応させる。
文字列の完全一致を要求するのではなく、次の2種類だけ差分を許可する。

1. `docs/LOCALIZATION.md` に登録されたlocale翻訳差分。
2. Windows標準のメニュー/アクセシブル名/Window操作作法に必要な表現差分。

それ以外の言い換え、短縮、派生値、単位変更、系列順変更、状態文の置換、重複説明は許可しない。
日本語localeでは既存のX版カタログをcanonical keyとし、Windows文言はそのkeyのmappingとして
管理する。mappingのない文字列を実装者が追加してはならない。

## 固定する意味フィールド

- period start/end、reset境界、timezone offset
- period selectorはcanonical `id`と受理array indexだけで選択する。REST `label`はLinux/X referenceであり、
  Windows表示labelは同じstart/end/current/idを保存済み`timeZoneId`とlocaleで再renderする。表示文字列から
  日時・ID・currentを逆parseしない
- metric（ドル/トークン）
- series（Remaining/LUNA/TERRA/SOL）、系列順、visibility
- Remainingの独立0–100%軸
- modelの値、単位、桁、終端値、leader色
- 記録なし、欠測、idle、active、認証要求、エラーの状態
- menu、戻る、閉じる、更新、設定、法的情報の操作結果

## 禁止される差分

- X版と異なる軸意味、終端、期間、折れ点、補間、系列順
- timezone/locale差を除いたperiod両端instant、canonical ID、offset、current/duplicate suffix roleの不一致、
  またはreference/display labelから日時やIDを逆推定すること
- `remaining`をmodel dollar/token scaleへ暗黙に混ぜること
- 同じ事実を別ラベルや別カードで再掲すること
- 文字数を理由に主値・単位・状態を省略すること
- X版にない説明、推定値、クレジット換算、請求額を追加すること

## 受入oracle

同一fixture、同一artifact世代、同一locale、同一timezoneで、X/Windowsの正規化label列を比較する。
各列は `canonical_key, normalized_text, semantic_role, unit, order, owner, action_result` とする。

- canonical key、semantic role、unit、order、owner、action_resultが一致する。
- normalized textが完全一致しない場合は、登録済みlocale/Windows mappingに一意に解決できる。
- period selectorはserver reference `label -> id`が1対1で、Windows表示labelも同じ`id/start_at/end_at/current`、
  選択済み`timeZoneId`の両端offset、suffix roleへ1対1に解決できる。旧期間→最新へ戻した後も文字列ではなく
  同じcanonical IDを再選択し、DST、同一基底label、current quota欠損を含むfixtureで逆parse回数0とする。
- mapping外の差分、clip、overlap、重複、未登録label、意味の欠落が一つでもあればFAIL。
- 同一labelの比較画像だけではPASSにせず、raw label列とmapping manifestを必須とする。

## 影響要求

`WIN-B-017..022`, `WIN-C-013..015`, `WIN-F-002`, `WIN-G-001..012`,
`WIN-L-006..009`, `WIN-M-019..024`, `WIN-M-030`。

このDecision Recordは仕様曖昧U-04を閉じるが、同一fixture、同一SHA、fresh画像、独立評価の
証拠が揃うまで、実装・受入・完了判定をPASSへ変更しない。

## X版との関係

X版のデータ意味、期間、軸、系列、終端、状態所有者を正本として維持する。Windowsのmenu、accessible
name、Window操作だけを登録mappingとして許し、X版にない派生値や説明を追加しない。

## 非スクロール影響

label差分を収めるために文字縮小、clip、主値省略、root/internal scrollを導入しない。10言語で収まらない
場合は登録済みlayout/page Decisionへ戻し、意味labelを切断しない。

## 証拠計画

同一fixture/source freeze/releaseでX/Windowsのraw canonical label列、mapping manifest、locale/timezone、
artifact別SHA、fresh画像、boundsを取得する。独立担当がkey/role/unit/order/owner/action resultと
missing/extra/duplicate/clip/overlapを再計算する。periodについてはserver reference label、canonical ID、
start/end/current、Windows rendered label、両端offset、suffix role、selection index、parse invocation countを
同じrowへ結合する。

## 未確定

許容差分と合否式は確定した。実artifact、raw label列、fresh画像、独立製品判定は未取得であり、
`PRODUCT_EVIDENCE_PENDING`である。

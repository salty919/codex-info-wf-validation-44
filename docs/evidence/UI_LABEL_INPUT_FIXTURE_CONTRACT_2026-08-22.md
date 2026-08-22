# UI label / non-cursor input evidence fixture contract

状態: `EXTRACTION_CONTRACT_DEFINED / PRODUCT_EVIDENCE_PENDING`

## 現行WIN-IDへの逆リンク

| U契約 | 所有する現行要求ID | 受入対象 |
| --- | --- | --- |
| U-04 label inventory | WIN-B-017..018, WIN-B-021..024, WIN-G-001..016, WIN-M-019..024, WIN-M-030 | locale/timezone/state別の文字列、単位、系列順、色owner、mapping IDを突合する |
| U-05 input policy | WIN-G-013..015, WIN-K-016, WIN-L-010..011, WIN-M-025..028 | accessible name、keyboard/focus、物理cursor書込み0、無断focus steal 0を判定する |

各IDは `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md` の同一行へ逆参照する。
`UX-20260822-GRAPH-001` は WIN-B-017..018、WIN-B-021..024、WIN-G-006、WIN-G-010..012、
WIN-M-019..024、WIN-M-030 が所有し、別IDの言換え根拠にはしない。

## U-04 label inventory

locale（ja-JP、en-US、de-DE以上）× timezone（local、UTC）× state（initializing、auth、ready、warning、danger、error）
の全セルに、画面名、label key、意味field、単位、値、period start/end、系列順、色owner、X文字列、Windows文字列、
登録済みmapping ID、clip/overlap測定欄を持たせる。`UX-20260822-GRAPH-001` 以外の言換え・単位変更・順序変更はFAIL。
fresh image、raw text export、geometry、artifact SHA、independent reviewが揃わない限り製品証拠はINCONCLUSIVE。

## U-05 input policy fixture

default smokeはカーソル書込み、synthetic mouse、`SetCursorPos`、無断focus stealを0回とし、静的API scanとprocess logを保存する。
window移動の実測は隔離VMのopt-inだけで行い、native message/OS automationの入力列、Window.Position、BeginMoveDrag、
focus owner、button click保持、完了時刻をraw logへ記録する。hostのユーザー入力を奪う試験はFAILであり、未実施をPASSにしない。

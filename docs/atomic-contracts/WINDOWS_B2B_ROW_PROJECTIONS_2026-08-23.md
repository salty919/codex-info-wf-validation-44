# Windows B2B row projections

Decision ID: `UX-20260823-B2B-CUSTOMER-DELIVERY-001`

Decision version: `b2b-customer-delivery-v1`

状態: `REQUIREMENTS_SELECTED / PRODUCT_PENDING / FRESH_AUDIT_REQUIRED`

## 目的と解決規則

本書は `RC-122..129` と `RC-150..159` のcurrent Windows targetを、226件の10-field原子行へ
1:1で結ぶtyped projection正本である。exact値の唯一ownerは
`docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md` §14とし、具体行へ値を複製して
driftさせない。各base 10-field行と本書の同じrow IDを一つの具体契約として解決する。

machine gateはopen-conflict targetをfresh展開してこの表と集合一致させる。欠落・extra・重複、
RC順序差、Decision ID/version不一致、authorityのHOLD/FAIL/INCONCLUSIVEを1件でも検出した場合、
対象行、要求抽出、実装再開、customer deliveryをPASSへ昇格しない。製品実装・artifact・実機証拠は
別gateであり、本表の存在を製品PASSに使わない。

## A-D projection

`current_target_count=0`。対象外のB2B値をA-D行へ発明・複製しない。

## E-I projection

| concrete row ID | `b2b_authority_projection` exact set |
| --- | --- |
| `WIN-E-016` | `RC-126,RC-150,RC-155` |
| `WIN-G-013` | `RC-128,RC-157,RC-159` |
| `WIN-G-014` | `RC-128,RC-157,RC-159` |
| `WIN-G-015` | `RC-128,RC-157,RC-159` |
| `WIN-G-016` | `RC-128,RC-157,RC-159` |
| `WIN-H-001` | `RC-122,RC-124,RC-150,RC-151,RC-152,RC-154` |
| `WIN-H-002` | `RC-122,RC-124,RC-150,RC-151,RC-152,RC-154` |
| `WIN-H-003` | `RC-122,RC-123,RC-125,RC-150,RC-151,RC-152,RC-153,RC-154` |
| `WIN-H-004` | `RC-122,RC-123,RC-150,RC-151,RC-152,RC-153,RC-154` |
| `WIN-H-005` | `RC-122,RC-123,RC-124,RC-150,RC-151,RC-152,RC-153,RC-154` |
| `WIN-H-006` | `RC-122,RC-123,RC-150,RC-151,RC-152,RC-153,RC-154` |
| `WIN-H-007` | `RC-122,RC-123,RC-125,RC-150,RC-151,RC-152,RC-153,RC-154` |
| `WIN-H-008` | `RC-122,RC-123,RC-127,RC-150,RC-151,RC-152,RC-153,RC-154,RC-156` |
| `WIN-H-009` | `RC-122,RC-123,RC-124,RC-127,RC-150,RC-151,RC-152,RC-153,RC-154,RC-156` |
| `WIN-H-010` | `RC-122,RC-123,RC-125,RC-127,RC-150,RC-151,RC-152,RC-153,RC-154,RC-156` |
| `WIN-H-011` | `RC-122,RC-123,RC-125,RC-127,RC-150,RC-151,RC-152,RC-153,RC-154,RC-156` |
| `WIN-H-012` | `RC-122,RC-127,RC-150,RC-151,RC-152,RC-154,RC-156` |
| `WIN-I-001` | `RC-126,RC-155` |
| `WIN-I-002` | `RC-126,RC-155` |
| `WIN-I-003` | `RC-126,RC-155` |
| `WIN-I-004` | `RC-126,RC-155` |
| `WIN-I-005` | `RC-126,RC-155` |
| `WIN-I-006` | `RC-126,RC-155` |
| `WIN-I-007` | `RC-126,RC-155` |
| `WIN-I-008` | `RC-126,RC-155` |
| `WIN-I-009` | `RC-126,RC-155` |
| `WIN-I-010` | `RC-126,RC-155` |
| `WIN-I-011` | `RC-126,RC-155` |
| `WIN-I-012` | `RC-126,RC-155` |
| `WIN-I-013` | `RC-126,RC-155` |
| `WIN-I-014` | `RC-126,RC-127,RC-150,RC-155,RC-156` |
| `WIN-I-015` | `RC-126,RC-127,RC-150,RC-155,RC-156` |
| `WIN-I-016` | `RC-126,RC-155` |

`projection_target_count=33`。

## J-M projection

| concrete row ID | `b2b_authority_projection` exact set |
| --- | --- |
| `WIN-J-006` | `RC-129,RC-150,RC-158` |
| `WIN-J-014` | `RC-129,RC-150,RC-158` |
| `WIN-J-015` | `RC-129,RC-150,RC-158` |
| `WIN-K-010` | `RC-159` |
| `WIN-K-011` | `RC-159` |
| `WIN-K-012` | `RC-159` |
| `WIN-K-013` | `RC-159` |
| `WIN-K-014` | `RC-159` |
| `WIN-K-015` | `RC-159` |
| `WIN-L-004` | `RC-150,RC-151` |
| `WIN-L-007` | `RC-157,RC-159` |
| `WIN-L-008` | `RC-157,RC-159` |
| `WIN-L-009` | `RC-157,RC-159` |
| `WIN-L-010` | `RC-157,RC-159` |
| `WIN-L-015` | `RC-122,RC-123,RC-124,RC-125,RC-126,RC-127,RC-128,RC-150,RC-151,RC-152,RC-153,RC-154,RC-155,RC-156,RC-157` |
| `WIN-L-016` | `RC-129,RC-150,RC-158` |
| `WIN-M-001` | `RC-159` |
| `WIN-M-002` | `RC-159` |
| `WIN-M-003` | `RC-159` |
| `WIN-M-004` | `RC-123,RC-125,RC-153,RC-159` |
| `WIN-M-005` | `RC-159` |
| `WIN-M-006` | `RC-159` |
| `WIN-M-007` | `RC-159` |
| `WIN-M-008` | `RC-159` |
| `WIN-M-009` | `RC-159` |
| `WIN-M-010` | `RC-159` |
| `WIN-M-011` | `RC-159` |
| `WIN-M-012` | `RC-159` |
| `WIN-M-013` | `RC-159` |
| `WIN-M-014` | `RC-123,RC-125,RC-153,RC-159` |
| `WIN-M-015` | `RC-122,RC-123,RC-125,RC-126,RC-127,RC-129,RC-150,RC-151,RC-152,RC-153,RC-154,RC-155,RC-156,RC-158,RC-159` |
| `WIN-M-016` | `RC-122,RC-123,RC-125,RC-126,RC-127,RC-129,RC-150,RC-151,RC-152,RC-153,RC-154,RC-155,RC-156,RC-158,RC-159` |
| `WIN-M-017` | `RC-159` |
| `WIN-M-018` | `RC-159` |
| `WIN-M-019` | `RC-128,RC-157,RC-159` |
| `WIN-M-020` | `RC-128,RC-157,RC-159` |
| `WIN-M-021` | `RC-128,RC-157,RC-159` |
| `WIN-M-022` | `RC-159` |
| `WIN-M-023` | `RC-159` |
| `WIN-M-024` | `RC-159` |
| `WIN-M-025` | `RC-128,RC-157,RC-159` |
| `WIN-M-026` | `RC-128,RC-157,RC-159` |
| `WIN-M-027` | `RC-128,RC-157,RC-159` |
| `WIN-M-028` | `RC-128,RC-157,RC-159` |
| `WIN-M-029` | `RC-128,RC-157,RC-159` |
| `WIN-M-030` | `RC-159` |

`projection_target_count=46`、`total_projection_target_count=79`。

## 独立oracle

1. conflict台帳からRC-122..129/150..159のcurrent `WIN-*` targetをrange展開する。
2. 各targetについて所属RC集合を昇順に正規化する。
3. 本書の79 ID/RC集合と完全一致することを確認する。
4. 226 concrete ID集合に全79 IDが存在し、A-D targetが0、E-Iが33、J-Mが46であることを確認する。
5. B2B Decision ID/version、§14.1..10、installer identity、ready predicate、7 flow、10 locale＋unknown、
   normal/high-contrast、typed N/A、DR禁止field、UI ownerをsemantic anchorで検査する。
6. source SHA、projection table SHA、Decision SHA、independent reviewerを同じrequirements freezeへjoinする。

上記のどれかが不一致なら `projection_pass=0`、`implementation_resume=0`、
`customer_delivery_eligible=0` とする。

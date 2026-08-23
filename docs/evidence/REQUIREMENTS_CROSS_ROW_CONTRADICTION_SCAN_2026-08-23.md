# 要件cross-row矛盾スキャン記録（2026-08-23）

状態: `SEMANTIC_HOLD`（直接矛盾の一部を修正、未決authority・fresh監査・製品証拠は未完了）

## 対象と方法

- 現行226具体契約、旧96 crosswalk、value authority、FULL-STATE/SSH/REST/DATA Decision、conflict台帳を同一source revisionで再読した。
- `bash scripts/windows_requirements_extraction_check.sh` の構造・型付きDAG・cross-row authority guardを実行した。
- `bash scripts/requirements_intake_guard.sh` は抽出未完了を検出して終了コード1となることを確認した。
- 値が資料にない場合は補完せず、`OPEN` または `OPEN_AUTHORITY_CONFLICT` のまま保持した。

## 機械的に一致した共通値

- current 226 / legacy 96、hard 412 / related 165 / total 577、hard cycle/SCC/backward 0。
- FULL-STATEの17 state集合、旧aliasのstate昇格禁止、wire `state=ready`とUI canonical stateの分離。
- fixed surface 900×480、Graph 940×640 initial / 700×480 minimum、Help追加HWND=0。
- 設定6-key、profile enum `none|wsl|sshConfigAlias`、REST Content-Type `application/json; charset=utf-8`、wire ready boolean禁止、ready predicate。
- A-DのStatus境界、locale/catalog失敗の分離、900×480 canonical grid、minimum viewport契約。

## 直接矛盾の修正

`wire state=ready`をUI state IDとして扱うと、canonical 17-stateの`normal`/`quota_warning`/`quota_danger`/`reset_warning`と衝突するため、value authorityと`WIN-C-018`へ「readyはAPI入力事実、UI state IDではない」を明記し、machine guardへ追加した。`ready=true` wire fieldを発明する意味は採用していない。

再監査で、`docs/CUSTOMER_OPERATIONS_RUNBOOK.md`の起動順に残っていた`ready=true`が、wire boolean禁止・`state=ready AND authenticated=true`導出の正本と直接衝突することを検出した。ランブックを導出条件と`wire ready boolean field=0`へ修正し、runbook内の旧曖昧表現を拒否するcross-row guardを追加した。これは実装値やAPI fieldを推測して追加する修正ではない。

fresh独立監査が指摘したJ-M補助表のraw ID重複については、RC-167〜169 extensionのjoin列を`source_id=WIN-J-007..016`へ明示し、補助表を本体226行と機械的に区別するguardを追加した。本体の10列契約IDや依存辺は変更していない。

追加再読で、`WIN-J-004`の期待値に残っていた`keyは(reset_at,timestamp)だけ`が、同じ行のfixture・`WIN-J-003`/`WIN-J-005`・DB authorityのcanonical `(partition_id,reset_at,timestamp)`と直接矛盾していることを検出した。根拠のない補完はせず、RC-057が指定するpartition-aware keyへ行内期待値を統一し、`partition_id欠落`をFAIL条件へ追加した。旧表現の残存を機械guardで拒否し、fresh独立再監査へ戻した。
同じ再読で、`WIN-J-005`の旧タイトル`period+minute upsert`も本文のtimestamp key・minute列禁止と意味的に衝突するため、`period+timestamp upsert`へ改め、旧タイトルを機械guardで拒否する。
認証境界では、`WIN-K-004`の旧文言「auth_requiredで安全なquota/plan値を表示できる」が、認証消去authorityおよび`WIN-C-002`の`plan_label=null/quota=null`と衝突していた。auth_requiredで根拠のない現行account値を残さないよう、plan/quotaは表示せず旧account-scoped detailsもcurrentへ混在させない文言へ統一し、旧許可文言を機械guardで拒否する。これは安全値の推測によるハルシネーションを防ぐための境界である。
`WIN-K-012`にも、Helpを`Help when separate`として子Window集合へ入れられる旧表現が残っていた。これはHelp=Main内、追加HWND=0の正本と直接矛盾するため、子Window集合からHelpを除外し、Main-internal routeとして明記した。旧表現の残存は機械guardで拒否する。
RC-167/168の`source_id=WIN-J-012`補助oracleには、`attempt1 full rollback`を無条件に要求する表現があり、2秒以内commitと期限超過BUSY rollbackを分ける本体契約と衝突していた。補助行を「BUSY期限超過時だけ全rollback、commit-within-deadlineならattempt1 commit、same-cycle retry=0」へ条件付きに修正し、無条件表現を機械guardで拒否する。

さらに`WIN-J-012`本体fixture欄にも`attempt1=BUSY_full_rollback`がsubcaseを限定せず残っており、1.5秒ロック解放のsubcase A（期限内commit）と意味的に衝突していた。fixtureを`subcase-B attempt1=BUSY_full_rollback`と`subcase-A attempt1=commit_within_deadline`へ分離し、negative/gateも「BUSY期限超過時だけ全rollback、期限内commitではcommit」と明記した。機械guardは両subcaseの条件付き値を必須化し、無条件rollback表現を採用しない。

## fresh独立再監査（V9）

修正者と異なる評価担当が最新bytesを再読し、`WIN-J-004`/`WIN-J-005`/`WIN-K-004`の直接矛盾修正、RC extensionの`source_id` join、旧禁止表現の現行契約からの除去を確認した。machine gateはexit 0、intake guardはexit 1。rawログSHAはmachine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`。RC状態はOPEN 93 / OPEN_AUTHORITY_CONFLICT 27 / FIXED_PENDING_FRESH_AUDIT 51 / CLOSED 3で、semantic/product/freezeは`SEMANTIC_HOLD` / `PRODUCT_PENDING` / `FREEZE_NOT_CAPTURED`のまま。全体判定はFAIL/HOLDであり、局所修正を抽出完了へ昇格しない。

## fresh独立再監査（V11）

`fresh_requirements_audit_v11` は、RC-167/168 extension条件付きoracle、WIN-J-012本体のbusy deadline=2.000秒、BUSY時rollback、same-cycle retry=0、次cycle/明示操作の最大1 retry、ならびにJ-004/J-005/K-004/K-012の既存修正を最新bytesで再読した。機械ゲートはexit 0、requirements-intake-guardはexit 1。rawログSHAはmachine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`。直接修正の構造・意味アンカーはPASSだが、RC状態はOPEN 93 / OPEN_AUTHORITY_CONFLICT 27 / FIXED_PENDING_FRESH_AUDIT 51 / CLOSED 3、semantic/product/freezeは`SEMANTIC_HOLD` / `PRODUCT_PENDING` / `FREEZE_NOT_CAPTURED`であり、全体はFAIL/HOLDを維持する。

## fresh独立再監査（V12）

`fresh_requirements_audit_v12` は、V11後に修正したWIN-J-012本体fixtureの条件分離（subcase-B `BUSY_full_rollback` / subcase-A `commit_within_deadline`）を最新bytesで再確認し、RC extension J012、J-004/J-005/K-004/K-012も突合した。`git diff --check` と機械ゲートはexit 0、requirements-intake-guardはexit 1。rawログSHAはmachine=`3f0057190c2739c6b9ae4cdc11f7313de5091a49711076d2e5d732b1779c6a41`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`。全体抽出は未完了であり、独立判定はFAIL/HOLD、semantic/product/freezeのPASSへ昇格しない。

## fresh独立再監査（V16）

`fresh_requirements_audit_v16` は、ランブックのUIあり成功条件を含む3箇所の`state=ready AND authenticated=true`、wire boolean=0、J-004/J-005/J-012、K-004/K-012、RC-020/055/107の`FIXED_PENDING_FRESH_AUDIT`状態を最新bytesで確認した。machine gateと`git diff --check`はexit 0、requirements-intake-guardはexit 1。rawログSHAはmachine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`。canonical indexは`EXTRACTION_INCOMPLETE / PRODUCT_CHANGE_FROZEN`のままであり、全体はFAIL/HOLDを維持する。

## 未解決（勝手に解決してはいけないもの）

現行 conflict ledger の状態は次のとおりである。

| 状態 | 件数 | 扱い |
| --- | ---: | --- |
| `OPEN` | 90 | authority値・境界またはcross-row決定が未確定。推測で閉じない |
| `OPEN_AUTHORITY_CONFLICT` | 27 | 正本選択または承認境界が未確定。別Decisionを発明しない |
| `FIXED_PENDING_FRESH_AUDIT` | 54 | 修正bytesはあるが、修正者と別のfresh意味監査が未PASS |
| `CLOSED` | 3 | GOV-THREAD-END / GOV-NO-INPUT-END / GOV-ESCALATION-100Xのみ |

V15のfresh独立監査で最新bytesのJ012、J004/J005、K004/K012、canonical ready、運用ランブックを突合したため、直接矛盾修正済みのRC-020/055/107を`FIXED_PENDING_FRESH_AUDIT`へ移した。これは製品証拠・freeze証拠・全行意味監査を満たす`CLOSED`昇格ではない。

`WIN-G-014`の`Alt+H` fixtureにある`top_level_window_count=6`は、同じfixtureで6 surfaceをロードした際の既存top-level総数としてはHelp追加HWND=0と矛盾しない。ただしHelp開閉前後のdeltaを行内で明示していないため、別のHWND追加がないと推測して閉じず、lifecycle/UIAの独立証拠待ちとして保持する。

`cross_row_direct_scan_v14` は、WIN-G-014/G-015/M-013とHelp/keyboard/ready authorityをread-onlyで突合し、G-014の6件は全surface-loaded fixtureの既存総数としてHelp追加HWND=0と整合、ready predicateも直接矛盾なしと判定した。一方、未ロードchild時のcount scope、fresh UIA/画像、製品証拠はINCONCLUSIVE/HOLDのまま。raw memo SHA=`f1710130d1575075c2e9ca92ad9a0bcb1c7da9146e7b122aba272d181e79a55e`。

`fresh_requirements_audit_v17` は、RC statusを含む最新台帳・証跡・契約を再集計し、OPEN 90 / OPEN_AUTHORITY_CONFLICT 27 / FIXED_PENDING_FRESH_AUDIT 54 / CLOSED 3の一致を確認した。machine gate exit=0、intake guard exit=1、`git diff --check` exit=0。rawログSHAはmachine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`、diff log=`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。RC-020/055/107はFIXED_PENDINGのままで、全体FAIL/HOLDを維持する。

`fresh_requirements_audit_v18` は、V17後の証跡・台帳追加を含む最新bytesを再確認し、machine gate PASS、`git diff --check` PASS、requirements-intake-guard FAIL（`EXTRACTION_INCOMPLETE`）を確認した。machine rawはcurrent=226、legacy=96、conflicts=174、raw_tokens=617、expanded_targets=1662。rawログSHAはmachine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`。全体HOLDを維持する。

旧96のfresh監査は`88 PASS / 7 FAIL / 1 INCONCLUSIVE`であり、legacy-gap RC-164..171の再監査が必要である。freeze JSON/content SHA/artifact lineage、実プロセス・実機・DB証拠も未取得である。

したがって機械構造PASSを要求抽出完了へ昇格せず、`EXTRACTION_INCOMPLETE / PRODUCT_CHANGE_FROZEN`、`requirements-intake-guard=FAIL`、`SEMANTIC_HOLD`を維持する。

## 直接矛盾の追加修正（V19）

`direct_conflict_scan_jm_v19` のread-only突合で、`docs/UX_DECISION_SSH_CONNECTION_2026-08-23.md:80,82`
にwire状態を `auth_required,false` / `ready,true` と表す旧記述が残っていることを検出した。
これはREST正本、Value Authorities、WIN-E-013/WIN-K-004の
「wireに`ready` booleanを持たず、`state=ready AND authenticated=true`だけを受理する」境界と直接矛盾する。

修正は次の範囲に限定した。

- Setup stepを `state=auth_required,authenticated=false` と
  `state=ready,authenticated=true` の実在fieldへ置換。
- Main遷移条件を `state=ready AND authenticated=true` に固定。
- `windows_requirements_extraction_check.sh`へSSH decisionのcanonical anchorと、
  `auth_required,false` / `ready,true`等の旧wire shorthandを拒否するnegative guardを追加。

修正後の機械ゲートはexit 0、`git diff --check`とbash構文検査もPASS。
`requirements-intake-guard.sh`はexit 1（`EXTRACTION_INCOMPLETE`）を維持し、RC-107は
`FIXED_PENDING_FRESH_AUDIT / READINESS-WIRE-AUTHORITY`からCLOSEDへ昇格していない。
fresh独立評価の再確認まではsemantic/product/freezeをHOLDとして保持する。

## fresh独立再監査（V7）

修正後bytesを別評価担当が再読し、RC extensionの本体ID重複なし（本体1件＋extension oracle join 1件）、machine gate exit 0、intake gate exit 1を確認した。wire/canonical境界も構造上確認された。一方、`OPEN_AUTHORITY_CONFLICT`、`PRODUCT_PENDING`、fresh product evidence未取得は残り、意味・製品受入は`INCONCLUSIVE / HOLD`である。

## A-D/E-I追加突合と修正（V20候補）

`direct_conflict_scan_ad_ei_v19` は、A-D/E-Iの現行bytesをread-onlyで再突合した。
authority未決のargv順序、reset_at過去値、auth-clear優先順位は推測で閉じず、既存RC-059/078/108等の
`OPEN`または`OPEN_AUTHORITY_CONFLICT`へ保持する。一方、根拠が現行正本にある直接矛盾は次の範囲だけ修正した。

- RESTの`ssh -N -o ...`は概念shell例と明記し、Windowsのcanonical `ArgumentList`（Value Authorities/E-006..010）と競合しない境界を追加。
- WIN-B-009の両側実測で挟まれたactive interior補間を、Value Authoritiesへ明記し、確定gap・終端・expired reset hint・tombstoned AuthEpochは補間しない。
- WIN-F-006/RC-029はLegal UIの6カテゴリとinstaller/packageの7分類（runtime独立）をスコープ分離。
- WIN-I-014はHTTP 500 wire=`internal_error`、UI=`DB_SERVER_ERROR`、`error.data.unavailable.*`へ修正。
- WIN-I-016のauth-clearはdata pair commitではないsecurity visibility transitionであり、details invalid時も旧account可視値だけを消去してstore/DB/pair bytesを保持する、とREST/DP正本へ明記。

修正後のmachine gateはexit 0、`git diff --check`とbash構文検査もPASS、intake guardはexit 1のまま。
上記は要求文書とnegative guardの修正であり、製品証拠・freeze・全174 RCのfresh closureを意味しない。

V20 evaluatorは所定時間内にraw結果を返さず、判定を採用しなかった（`HOLD/NO_USABLE_OUTPUT`）。
同一入力の再試行はせず、短い独立ゲートだけを実行するV21 evaluatorへ再配置している。

`fresh_requirements_audit_v21` は修正後bytesを独立read-onlyで再監査した。`git diff --check`、bash構文検査、
machine gateは全てexit 0、intake guardはexit 1（要求抽出未完了）であった。再計数は226/96、hard=412、
related=165、typed_total=577、RC=174、raw_tokens=617、expanded_targets=1662、CLOSED=3であり、
OPEN/OPEN_AUTHORITY_CONFLICT/FIXED_PENDINGが残るため判定はHOLD。対象SHAはscript=`eed0e07182e8c30d70240fd04f90f7969d4e52bbc32254b067226fa1f5d23dd1`、
REST=`cbfad245415f6d016834dc60f574f86509c51954debf398b2bd48b184d5e14fe`、
DP=`8ee1673056ce9e7e64c10348d4ae792f8f67790b69fba0cb3b4bcbed72dfccf9`、
E-I=`2dd80c13669bfaeb5e2cc7d4b7e2189c24d04d49f5f3f83e10864b49ba387938`である。

最終metadata反映後に再配置したV22 evaluatorはraw結果を返さず割り込みとなったため、V22判定は採用しない。
直前のV21 fresh auditと直近の主担当machine再実行（machine=0、intake=1）を別々の証拠として保持する。

## fresh独立再監査（V24）

`fresh_requirements_audit_v24` は、DESIGNのpartition-aware DB key追記と、旧一意キー表現を拒否する最新negative guardを含むbytesを、修正者とは別にread-only再監査した。`git diff --check` と bash構文検査はexit 0、`scripts/windows_requirements_extraction_check.sh` はexit 0（`MACHINE_GATE_PASS`）、`scripts/requirements_intake_guard.sh` はexit 1（`requirements extraction is incomplete; implementation/evaluation/release remain blocked`）だった。DESIGN.md:78/110の`(partition_id,reset_at,timestamp)`、wire projectionとDB keyの分離、旧`` `(reset_at,timestamp)`が一意キーで ``不在を確認した。

最新bytesのSHA256は、DESIGN=`990d3b93d3acad36149b7de81b202ff88631b2e34cd29e9e63fcd970e594df40`、script=`611fb9b74ce01f5f7506ecc3113fd57b0c4b14934d7b2b6ead10f04afc64f581`、REST=`cbfad245415f6d016834dc60f574f86509c51954debf398b2bd48b184d5e14fe`、DP=`8ee1673056ce9e7e64c10348d4ae792f8f67790b69fba0cb3b4bcbed72dfccf9`、E-I=`2dd80c13669bfaeb5e2cc7d4b7e2189c24d04d49f5f3f83e10864b49ba387938`である。OPEN / OPEN_AUTHORITY_CONFLICT / FIXED_PENDING_FRESH_AUDIT、PRODUCT_PENDING、FREEZE_NOT_CAPTUREDが残るため、V24の全体判定はFAIL/HOLDであり、抽出完了へ昇格しない。
最終再実行のraw log SHAは`git diff --check=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`、machine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`である。RC状態は`OPEN=90 / OPEN_AUTHORITY_CONFLICT=27 / FIXED_PENDING_FRESH_AUDIT=54 / CLOSED=3`を維持する。

## 直接修正の追加（V25候補）

現行具体契約を再読し、RC-023/058/160/161/162/163の必須修正が本文へ反映済みであることを確認した。未確定authorityや製品証拠を閉じる変更は行わず、状態だけを`FIXED_PENDING_FRESH_AUDIT`へ移し、M-015/M-016、I-001/J-001、A-019、C-017、C-020、D-004の必須語を機械guardへ追加した。V25 evaluatorは所定時間内にraw結果を返さず割り込みとなったため、その判定は採用しない（`HOLD/NO_USABLE_OUTPUT`）。この時点のRC状態は`OPEN=85 / OPEN_AUTHORITY_CONFLICT=26 / FIXED_PENDING_FRESH_AUDIT=60 / CLOSED=3`である。

## fresh独立再監査（V26）

V26はV25と別の短縮監査として、`git diff --check`、bash構文、machine gate、対象6 conflict状態、8つの必須意味アンカーを再確認した。全コマンドはmachine側までexit 0、対象RC-023/058/160/161/162/163は全て`FIXED_PENDING_FRESH_AUDIT`、必須語句は全件PASSだった。`requirements_intake_guard.sh`はexit 1（要求抽出未完了）であり、V26の全体判定はFAIL/HOLD。ソース変更はない。未解決のOPEN/authority行、製品証拠、freezeは残るため、6行をCLOSEDへ昇格せずfresh待ちとして保持する。
V26はV25と別の短縮監査として、`git diff --check`、bash構文、machine gate、対象6 conflict状態、8つの必須意味アンカーを再確認した。全コマンドはmachine側までexit 0、対象RC-023/058/160/161/162/163は全て`FIXED_PENDING_FRESH_AUDIT`、必須語句は全件PASSだった。`requirements_intake_guard.sh`はexit 1（要求抽出未完了）であり、V26の全体判定はFAIL/HOLD。ソース変更はない。未解決のOPEN/authority行、製品証拠、freezeは残るため、6行をCLOSEDへ昇格せずfresh待ちとして保持する。V26後のRC状態も`OPEN=85 / OPEN_AUTHORITY_CONFLICT=26 / FIXED_PENDING_FRESH_AUDIT=60 / CLOSED=3`である。

## fresh独立再監査（V27）

V27はRC-031/036の状態昇格後に、K-002のexact argv・7 token・shell/cmd/PowerShell禁止・last-good、M-030の11 Decision record・freeze anchor・required-field検査をread-onlyで再確認した。`git diff --check`、bash構文、machine gateは全てexit 0、`requirements_intake_guard.sh`はexit 1。対象マーカーは確認済みだが、全体判定は`INCONCLUSIVE / HOLD`であり、RC-031/036をCLOSEDへ昇格しない。RC状態は`OPEN=83 / OPEN_AUTHORITY_CONFLICT=26 / FIXED_PENDING_FRESH_AUDIT=62 / CLOSED=3`である。

## fresh独立再監査（V28）

V28はRC-089/090の状態昇格後に、Settings保存失敗のcanonical class/CTA/保持境界、5つのSSH観測可能class、generic exitからDNS/key/password等への推測禁止、非canonical class禁止をread-onlyで再確認した。`git diff --check`、bash構文、machine gateはexit 0、intake guardはexit 1。限定範囲はPASSだが全体はHOLDであり、RC-089/090をCLOSEDへ昇格しない。実artifact・fresh UI・製品実行時証拠は未取得である。RC状態は`OPEN=81 / OPEN_AUTHORITY_CONFLICT=26 / FIXED_PENDING_FRESH_AUDIT=64 / CLOSED=3`である。

## fresh独立再監査（V29）

V29はRC-084のHelpアクセシビリティ契約をread-onlyで再確認した。`git diff --check`、bash構文、machine gateはexit 0、intake guardはexit 1。M-025のHelpScopeGeneration/HelpCloseToken、7 Escape branches、reverse focus、UIA、追加HWND 0、G-015のMain owner/2 logical px/3:1、M-013のroute instance=1を確認した。限定範囲は`INCONCLUSIVE / HOLD`で、製品UI証拠なしのためCLOSEDへ昇格しない。RC状態は`OPEN=80 / OPEN_AUTHORITY_CONFLICT=26 / FIXED_PENDING_FRESH_AUDIT=65 / CLOSED=3`である。

## 直接矛盾修正とfresh独立再監査（V30）

同一正本内で旧§7の`OPEN`記述と後段DP-REST採用値が併存していたため、旧節を検出履歴・参照へ降格し、`REST_API_V1.md`のDP-REST wire authorityと`DATA_PROTECTION_POLICY.md` §8.1〜§8.13を値の唯一ownerへ統一した。対象はRC-047、RC-067、RC-078、RC-139〜RC-149である。freezeは65 path/11 Decisionを維持し、実freeze manifest未生成を閉じていない。

独立担当V30は修正者と別のread-only監査として、`git diff --check`、bash構文、machine gateをexit 0、`requirements_intake_guard.sh`をexit 1で再確認した。RC-047/067/078/139〜149の全件が`FIXED_PENDING_FRESH_AUDIT`であり、REST health/error/request/effect、partition/checkpoint/generation/restore/boot/lineage/load、gap projection、pair atomicityの採用節を突合した。製品artifact・実Windows・freeze capture・全conflict closureは未取得のため、全体判定は`HOLD / SEMANTIC_HOLD / PRODUCT_PENDING`であり、CLOSEDへ昇格しない。

V30時点の状態は `OPEN=65 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=84 / CLOSED=3`（追加5件のfresh監査待ち）である。主要SHAは script=`3d2062cdaa377e55a8286fada870cd9387e001913578d92bb40710371cd162c5`、conflict=`83ec1c2cf7111222c3f459476f0f5c05a4c31bc8b4580c828375e943515684c0`、REST=`ffa2b830b76b878737d124723996cd820da9be30233305b010666a86fe2a28df`、DP=`2a161b3313a3d6c4e0e6ba6354bd67609c7d7ab73513fff8ec5b4a365cb9fdad`である。

## 追加5行のfresh独立再監査（V31）

V31はRC-076/079/080/081/109を修正者と別にread-only再監査した。`git diff --check`、bash構文、machine gateはexit 0、intake guardはexit 1。JSONL/REST上限、全JSON response header/no-store、SQLiteおよび非SQLite read-only effect、healthのactor/last-good境界、ERROR-001 19 class・5 SSH観測class・generic原因推測禁止とnegative guardを各根拠へ突合した。

5件は全て`FIXED_PENDING_FRESH_AUDIT`のまま保持する。`PRODUCT_PENDING`、freeze未capture、他のOPEN/OPEN_AUTHORITY_CONFLICTが残るため、全体は`HOLD / SEMANTIC_HOLD / PRODUCT_PENDING`であり、CLOSEDへ昇格しない。V31の主要SHAはV30と同一である。

## 追加4行のfresh独立再監査（V32）

V32はRC-108/111/112/113を修正者と別にread-only再監査した。`git diff --check`、bash構文、machine gateはexit 0、intake guardはexit 1。認証profile別exact argv、Graph period id/indexとlocale/timezone再render、PlanType 15 enumのlabel/monthly同一cycle写像、text-scale×DPI×locale直積とUIA/clip否定条件をValue Authorities・REST・Decision・具体契約・negative guardへ突合した。

4件は全て`FIXED_PENDING_FRESH_AUDIT`のまま保持する。実Windows/製品artifact/freeze未capture、他のOPEN/OPEN_AUTHORITY_CONFLICTが残るため、全体は`HOLD / SEMANTIC_HOLD / PRODUCT_PENDING`である。V32時点のRC状態は`OPEN=61 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=88 / CLOSED=3`。

## 追加2行のfresh独立再監査（V33）

V33はRC-054/088を修正者と別にread-only再監査した。`git diff --check`、bash構文、machine gateはexit 0、intake guardはexit 1。Window/DPI/topology/reopenの正本、17-state×7-surface full-state matrix、text-scale/DPI/theme/motion/locale/UIA dimensions、negative guardを突合した。

2件は`FIXED_PENDING_FRESH_AUDIT`のまま保持する。実UI/実Windows/artifact/freeze未capture、他のOPEN/OPEN_AUTHORITY_CONFLICTが残るため、全体は`HOLD / SEMANTIC_HOLD / PRODUCT_PENDING`である。V33時点のRC状態は`OPEN=59 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=90 / CLOSED=3`。

## V33後の最終決定論的ゲート

同一作業ツリーで `git diff --check`=0、`bash -n scripts/windows_requirements_extraction_check.sh`=0、machine gate=0（`MACHINE_GATE_PASS`）、intake guard=1（`requirements extraction is incomplete`）を再実行した。raw log SHAは machine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。現行RC状態は`OPEN=59 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=90 / CLOSED=3`で、semantic/product/freezeはHOLDのまま保持する。

## RC-083 の行間欠落修正とfresh独立再監査（V34）

WIN-M-012の共有ナビゲーション所有権がGraph/Threads各行へ個別に伝播していなかったため、`WIN-M-006 / WIN-M-007 surface-navigation addendum (RC-083)` と `source_id=WIN-M-006/007` の投影表を追加した。Back/title Closeの同一viewport可視性、keyboard/UIA到達性、Main route復帰、Graphのperiod/metric/toggleまたはThreadsのpage/selection・last-good・DB・settings保持、非破壊、副作用0、surface別bounds/focus/key/route/hash証拠を固定し、共有行だけの合格を禁止した。機械gateへaddendum・source_id joinの存在検査を追加した。

fresh独立担当は修正者と別にread-only監査し、RC-083 scoped verdictをPASS、全体をHOLDとした。`git diff --check`=0、machine gate=0、requirements-intake-guard=1、実Windows/UIA/fresh画像は未取得である。RC-083は`FIXED_PENDING_FRESH_AUDIT / GRAPH+THREADS-UX`のまま保持し、他OPEN/OPEN_AUTHORITY_CONFLICTは変更していない。

V34監査時点SHA256は conflicts=`d40f5aa64632fe1dd98e69d5253bc002839db6e485e5eb4ef5c2c70fd4f9dd28`、concrete contracts=`d78791d8111223cabe3efb5d4327f32395b842f7bf1631b321d4f229c26b8da9`、machine script=`1baabc3013db943ec4105e7216d85a4951ac0aa2938ad40e75aef546dcffff93`、独立証跡=`28efacc98464067f2837a4637e0d29712ab536fd8ebeca60001c68c9639cda1c`である。現行RC状態は`OPEN=58 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=91 / CLOSED=3`。なお要求抽出・freeze・製品証拠は未完了であり、実装/evaluation/releaseは引き続きブロックする。

V34最終再実行では `git diff --check`=0、bash構文=0、machine gate=0、intake guard=1を確認した。raw log SHAは machine=`92a9172c3fd3655d66d9805ed4849a3b4fd962baac66c83d1b7aa981189cce06`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`。`requirements-intake-guard`のFAILは未解決OPEN/authority、freeze未capture、製品証拠未取得を意味するため、状態をPASSへ昇格しない。

## RC-082 部分joinと独立監査再配置（V35候補）

`WIN-G-014` の既存Escape branch、`WIN-F-007` のCancel confirmation、`WIN-E-011`/`WIN-E-016` のfailure oracleを `WIN-M-004` へ部分joinした。ただし現行正本だけではBackの直前step、初回Cancel後のMain disconnected＋Settings復帰、再表示Cancel時の全旧6-key bytes保持を一つの完全な契約として確定できないため、RC-082は `OPEN / SETUP-UX` のまま維持する。

最初の独立監査担当と再配置したV2担当は終端結果・raw SHAを返さず、各々 `HOLD/NO_USABLE_OUTPUT` と記録した。単純再試行はせず、最新revisionを別の既存独立担当へV3として再突合中である。未確認値をPASSへ昇格していない。

V3/V4再配置も終端raw結果を返さなかったため、`HOLD/NO_USABLE_OUTPUT` として状態証跡を保存した。最終決定論的再実行は `git diff --check`=0、bash構文=0、machine gate=0、intake guard=1。raw SHAは machine=`abeeee0409d9e24c37445dd477d71df7d292d221afea537d26d46935fa3ac23b`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`、conflict=`ca86fecd48252f6fd931b4a0605bac16c080f9b56581fa99423a2282a8fd61fa`、E-I契約=`e5c66e4b6b320d43056ed7e58ee525b48b23859abc40cfb5419ff488413b550d`、machine script=`7b5d6e83f50f0508e585aa97bfa6466f29f78d4c5d9a0b1a60b7e9cd94efaf45`。RC状態は`OPEN=58 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=91 / CLOSED=3`で、RC-082はOPENを維持する。

## RC-028 の根拠付きstatus移行（V36候補）

`WIN-C-006` の全状態優先順位・86400/86401境界・複合優先、`WIN-C-012` の ja/unknown locale解決、English catalog valid/missing/invalid UTF-8/key-setの分離、`WIN-C-018/019` の17-state・viewport・DPI式は現行具体契約に存在し、machine guardが各断片を検査している。値の追加発明は行わず、RC-028を `FIXED_PENDING_FRESH_AUDIT / A-D` へ移行した。

独立担当は終端raw結果を返さず `HOLD/NO_USABLE_OUTPUT` と記録したため、CLOSEDへ昇格していない。最終ゲートは `git diff --check`=0、bash構文=0、machine gate=0、intake guard=1。raw SHAは machine=`abeeee0409d9e24c37445dd477d71df7d292d221afea537d26d46935fa3ac23b`、intake=`ffc0e0efd6f404bcdb6733a451b4583b1f609c60fb035953beaa691d313ebd52`、conflict=`f39db95939df387f00239c0b7cd1bb5494c5ea422bb2eb16aea68c110c94e3f8`。現行RC状態は`OPEN=57 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=92 / CLOSED=3`。

## RC-077 の責務境界固定とfresh独立監査（V37）

REST/daemonの責務境界を既存の `REST_API_V1.md` と `DESIGN.md` へ再突合した。RecorderDaemonはsource JSONL→SQLiteの独立writerでHTTP listenerを持たず、SnapshotPublisherはcommit済みvalidated pairをnative UIとREST workerへread-onlyで供給し、UI/RESTはrecorderをspawnせず終了してもrecorderを停止しない。REST workerのDB/WAL/SHM/PublishedPair mutation=0、owner/publisher/DB fault時のhealth/errorと直前last-good pair保持も既存authorityへ明記されている。これらを機械guardへ追加し、RC-077を `FIXED_PENDING_FRESH_AUDIT` へ移行した。

fresh独立監査は文書根拠とmachine gateをPASS相当としたが、製品実機、release-artifact lineage、freeze captureは未取得のため全体判定を `HOLD` とした。証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC077_2026-08-23.md`。`git diff --check`=0、machine gate=0、requirements-intake-guard=1。最新RC状態は `OPEN=56 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=93 / CLOSED=3` で、RC-110はSetupのerror-stateセル別focus適用性が現行authorityで未確定のためOPENを維持する。

## RC-066 unexpected-exit projectionとfresh独立監査（V38）

既存のDATA_PROTECTION_POLICY・DESIGNにあるdaemon supervisor authorityを、WIN-J-011の行固有projectionへjoinした。明示TERM、unexpected exitの2秒検知、同一epochの5秒backoff後restart 1回、restart失敗または2回目unexpected exitのFailed latch、explicit start/systemd新activationによる新epoch、last-good/DB/hint/cursor/gap ledger保持、停止区間とgapの捏造禁止を固定し、RC-066を `FIXED_PENDING_FRESH_AUDIT` へ移行した。

fresh独立監査はprojection・machine gateをPASS相当としたが、製品実機、same-release artifact、freeze lineageが未取得のため `HOLD` を維持した。証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC066_2026-08-23.md`。最終RC状態は `OPEN=55 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=94 / CLOSED=3`。

## RC-061 顧客運用ランブックjoinとfresh独立監査（V39）

顧客運用ランブックの既存記述をRC-061へ行単位でjoinした。UIあり／UIなし起動、GUI・HWND・loopback境界、Cargo/repository/run.sh不要の通常顧客導線、setup install、systemd targetのstart/status/health/stop/restart、health/status/auth/ready分離、update/rollback/uninstall、restore/migrate、失敗時の旧server/unit/DB・settings/history/backup保持を機械guardへ追加し、RC-061を `FIXED_PENDING_FRESH_AUDIT` へ移行した。

fresh独立監査は静的文書とmachine gateを確認したが、runbook自身が実装・installed service・Windows・release/freeze証拠未取得を示すため `INCONCLUSIVE / HOLD` とした。証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC061_2026-08-23.md`。最終RC状態は `OPEN=54 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=95 / CLOSED=3`。

## RC-065 reset-hint/fingerprint/backfill projectionとfresh独立監査（V40）

DATA_PROTECTION_POLICY §4.1/§4.4/§8.7/§8.13およびWIN-J-010の既存値を、行固有projectionへjoinした。canonical regular non-symlink JSONL集合、device/inode・size・mtime_ns・LF offset・row SHA fingerprint、unchanged scan/write/retry=0、append cursor、rotate/truncate one recheck、4KiB UTF-8 hint、AuthEpoch/nonce、backfill latch=1、1024 rows/1MiB、expired/tombstoned拒否、old root保持、捏造禁止を固定し、RC-065を `FIXED_PENDING_FRESH_AUDIT` へ移行した。

fresh独立監査は静的projectionとmachine gateを確認したが、runtime fixture、same-release lineage、freeze captureが未取得のため `INCONCLUSIVE / HOLD` とした。証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC065_2026-08-23.md`。最終RC状態は `OPEN=53 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=96 / CLOSED=3`。

## RC-068 singleton/contention境界固定とfresh独立監査（V41）

`DATA_PROTECTION_POLICY.md` と `WIN-J-012`/`WIN-J-013` を行単位で再突合し、J-012の同一DB transaction contention（別の許可済みwriter process）と、J-013の同一profile recorder launch（canonical DB path + profile lease）を別fixtureとして固定した。J-012は2.000秒deadline、A=1.5秒/B=3.0秒のlock-release、Bのattempt1 full rollbackと後続cycle/explicit operationでの最大attempt2、unique/upsert/busy契約を保持する。J-013はlive owner count<=1、second recorderのlease前no-op、別profile/DBの独立、lease bypass禁止、PID/process-startとreopened identity一致、age-only reclaim禁止を保持する。値は既存正本からのみ採用し、daemon二重起動とDB writer競合を同一caseへ混同しない。

RC-068を `FIXED_PENDING_FRESH_AUDIT` へ移行し、machine guardへprojection断片を追加した。fresh独立監査は同一SHAの静的projectionを再確認するが、製品runtime、same-release artifact lineage、freeze captureが未取得なら `HOLD/INCONCLUSIVE` を維持する。

fresh独立監査の証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC068_2026-08-23.md`。raw結果は `git diff --check=0`、`bash -n=0`、machine gate=`0/MACHINE_GATE_PASS`、requirements intake guard=`1`。現行RC状態は `OPEN=52 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=97 / CLOSED=3` であり、RC-068はCLOSEDへ昇格しない。

## RC-069 stale-lock identity境界固定（V42）

`DATA_PROTECTION_POLICY.md` §4.1 のlease schemaとstale-lock authority、`WIN-J-013`の同一path再open oracleを再突合した。leaseは最大4KiB UTF-8 `recorder-lease-v1`で、PID、process-start、owner nonce、canonical DB path、OS file identityを保持する。stale recoveryはPID不存在またはprocess-start不一致を必要とし、削除直前に同じpathをreopenして取得時identityと再比較する。24時間は診断用経過時間に限定し、年齢単独、別owner、path/identity不一致の削除は0件とする。RC-069を `FIXED_PENDING_FRESH_AUDIT` へ移行したが、製品runtime・same-release lineage・freeze未取得のため独立評価後もCLOSEDへ昇格しない。
fresh独立監査の証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC069_2026-08-23.md`。静的projectionとmachine gateは整合したが、runtime removal trace、release lineage、freeze captureがないため `INCONCLUSIVE/HOLD`。現行RC状態は `OPEN=51 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=98 / CLOSED=3`。

## V43 data-protection gate boundary (RC-069外の未解消台帳)

独立監査が追加実行した `scripts/data_protection_gate.sh` は、schema mismatch・backup失敗時prune禁止・顧客runbookのDB保護文言を確認した後、`docs/REQUIREMENTS_LEDGER.md` の「verifiedが10件」を満たさず exit=1 となった。DP-001、DP-005、DP-009がHOLDのためであり、これらをverifiedへ偽装せず、RC-069の静的PASSやmachine gateへ混同しない。全体のrequirements intakeは引き続きFAIL/HOLDである。

## RC-070 maintenance owner/prune admission境界固定（V44）

`DATA_PROTECTION_POLICY.md` §2/§4.2、`WIN-J-006`、`WIN-J-014`を行単位で突合し、canonical DB profileごとの唯一のMaintenanceOwner、writer admission閉鎖、backup候補検証からrotationを経てpruneする順序、backup失敗・検証失敗・writer競合時のprune=0、1 activationあたり新generation最大1件、generation `0→1→2→3`、crash回復前のwriter/prune/publish=0を固定した。RC-070を `FIXED_PENDING_FRESH_AUDIT` へ移行するが、製品runtime、same-release lineage、freeze未取得のためCLOSEDへ昇格しない。
fresh独立監査の証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC070_2026-08-23.md`。静的projectionとmachine gateは整合したが、owner/admission、backup/prune、generation、crash runtime trace、release lineage、freeze captureがないため `INCONCLUSIVE/HOLD`。現行RC状態は `OPEN=50 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=99 / CLOSED=3`。

## RC-071 backup-generation journal境界固定（V45）

`DATA_PROTECTION_POLICY.md` §4.2 と `WIN-J-006`/`WIN-J-014`を突合し、`.bak.1/.bak.2/.bak.3`のverified順位、0→1→2→3の実時間蓄積、同一snapshot複製禁止、欠損・破損世代の除外、`backup-rotation-v1` journalのold rank/path/inode/hash・candidate hash・rename phase、各fsync、crash時のrollback/roll-forward、journal回復前のwriter/prune/publish=0、最新完全verified世代のみのrestore候補を固定した。RC-071を `FIXED_PENDING_FRESH_AUDIT` へ移行するが、runtime journal/recovery、release lineage、freeze未取得のためCLOSEDへ昇格しない。
fresh独立監査の証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC071_2026-08-23.md`。静的projectionとmachine gateは整合したが、runtime journal/recovery、crash rollback/roll-forward、backup inventory、restore oracle、release lineage、freeze captureがないため `INCONCLUSIVE/HOLD`。現行RC状態は `OPEN=49 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=100 / CLOSED=3`。

## RC-073 migration-switch 3経路境界固定（V46）

`DATA_PROTECTION_POLICY.md` §4.3 と `WIN-J-015` を突合し、old-schema startup reject、明示candidate success、candidate/lock/rename/fsync/reload/pair failureの3経路を固定した。`migration-switch-v1`のowner-only 0600 UTF-8 JSON <=64KiB、exact phase/interrupt、全row/type/value・unique key・quick_check・schema・row count・fingerprint・period/partition検証、current path missing/double/empty、rollback/roll-forward、回復前writer/publish=0、旧DB/backup/memory/root保持、同一operation再入とforeign operationのmutation=0をWIN-J-015へjoinした。RC-073を `FIXED_PENDING_FRESH_AUDIT` へ移行するが、runtime journal/recovery、same-release lineage、freeze未取得のためCLOSEDへ昇格しない。
fresh独立監査の証跡は `docs/evidence/REQUIREMENTS_EXTRACTION_INDEPENDENT_AUDIT_RC073_2026-08-23.md`。静的projectionとmachine gateは整合したが、runtime journal/recovery、interrupt replay、publication/current oracle、release lineage、freeze captureがないため `INCONCLUSIVE/HOLD`。現行RC状態は `OPEN=48 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=101 / CLOSED=3`。

## RC-072 explicit restore operation boundary (V47)

`CUSTOMER_OPERATIONS_RUNBOOK.md` §4、`WINDOWS_DATA_PROTECTION_ROW_CONTRACTS.md` の WIN-J-014/WIN-J-015、`DATA_PROTECTION_POLICY.md` §4.2/§4.3、および具体契約の RC-072 projection を行単位で突合した。顧客が実行するcommandは既存の `codex-info-server-setup restore --generation 1` に限定し、全writer/API/UI停止確認、現DBを削除しない退避、選択した完全verified世代のSHA/quick_check/schema/row count/deterministic fingerprint/reset-period境界検証、同一filesystem staging・flush・atomic replace、reload後のREST status/details pairとUI確認、失敗時の旧DB・全verified backup・old memory/root/history保持を固定した。`migrate --dry-run`/`migrate --apply`とは別operationとし、通常起動の自動restore、暗黙schema変換、current/backup削除、空DB置換、同一operation再入の二重replace/publication/generationを禁止した。新しい数値・path・成功値は追加していない。

RC-072は `FIXED_PENDING_FRESH_AUDIT` へ移行するが、stop/order/path/hash/validation/replace/reloadのruntime trace、same-release artifact lineage、freeze captureが未取得のためCLOSEDへ昇格しない。fresh独立監査は静的projectionとmachine gateを再計算し、必須runtime/release/freeze証跡の欠落により `INCONCLUSIVE/HOLD` とした。現行RC状態は `OPEN=47 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=102 / CLOSED=3`。

## RC-074 database fault matrix boundary (V48)

`DATA_PROTECTION_POLICY.md` §RC-168の11 fault enum、`WINDOWS_DATA_PROTECTION_ROW_CONTRACTS.md` のJ行fault join、WIN-J-006/J-009/J-012/J-014/J-015具体契約、およびRC-074投影を行単位で突合した。`BUSY`、`LOCKED`、`IOERR`、`FULL`、`READONLY`、`PERMISSION`、`CORRUPT`、`BACKUP_VALIDATION`、`BACKUP_ROTATION`、`PRUNE_CONTENTION`、`MIGRATION_LOCK`を既存の注入点・SQLite/OS結果・rollback/admission/prune/switch遷移・retry予算・保持対象へ固定し、各caseのoperation ID、hash、quick_check、restart open/read、publish/generation deltaを独立oracleとした。BUSY/LOCKEDのbounded next-cycle retry以外はsame-callback retry=0、fault結果の流用・partial publish・synthetic recovery・空DB再生成・未検証backup採用を禁止した。新しい数値・fault名・復旧経路は追加していない。

RC-074は `FIXED_PENDING_FRESH_AUDIT` へ移行するが、11 faultの実注入・rollback/restart trace、same-release artifact lineage、freeze captureが未取得のためCLOSEDへ昇格しない。fresh独立評価は静的matrixとmachine gateを再計算し、runtime/release/freeze欠落により `INCONCLUSIVE/HOLD` を維持した。追加で `scripts/data_protection_gate.sh` は verified ledger rows 不足（DP-001/005/009 HOLD）でexit=1となり、RC-074の静的PASSへ混同しない。現行RC状態は `OPEN=46 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=103 / CLOSED=3`。

## RC-075 bounded daemon work boundary (V49)

`DATA_PROTECTION_POLICY.md` §4.1/§8.7/§8.13、WIN-J-010/J-011 row contracts、J-M具体契約のRC-075 projection、既存RC-167 cursor規則を行単位で突合した。intervalは整数5..3600/default60、eventごとscan/transaction最大1、transaction最大1024 rows/1MiB、record/file/aggregate上限4MiB/256MiB/2GiB、canonical fingerprint、append/rotate/replace/truncate cursor規則、outage epochごとのbackfill latch=1、scan/restart retry最大1、same-callback retry=0、fingerprint不変時scan/write/retry=0を固定した。CPU/RSS値や未定義のretry動作は追加していない。

RC-075は `FIXED_PENDING_FRESH_AUDIT` へ移行するが、実daemonのbounded counters、fingerprint/cursor/backfill/restart raw trace、same-release artifact lineage、freeze captureが未取得のためCLOSEDへ昇格しない。fresh独立評価は静的boundとmachine gateを再計算し、runtime/release/freeze欠落により `INCONCLUSIVE/HOLD` を維持した。現行RC状態は `OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

## RC-076 local/REST resource boundary (V50)

`DATA_PROTECTION_POLICY.md` resource table、`REST_API_V1.md` transfer limits、WIN-I-006/WIN-J-010 row contract、RC-076 conflict、およびRC-076 projectionを突合した。local JSONLのline/file/aggregate（4MiB/256MiB/2GiB、decode前bytes）、internal validated snapshot（1MiB）、REST transfer（header/status/details 8KiB/64KiB/32MiB、Content-Lengthとstream cutoff）を別resourceとして固定し、local record隔離の後続cumulative snapshot条件、internal candidate reject、REST PublishedPair全体reject、last-good保持を分離した。新しい上限やfailure単位は追加していない。

RC-076は既存の `FIXED_PENDING_FRESH_AUDIT` を維持する。実byte counter、Content-Length/stream cutoff、local/internal/REST別state、same-release lineage、freeze captureが未取得のためCLOSEDへ昇格しない。fresh独立評価は静的resource境界とmachine gateの整合を確認したが、runtime/release/freeze欠落により `INCONCLUSIVE/HOLD` とした。RC状態は `OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。
## RC-077 current-source re-audit (V51)

RC-077のRecorderDaemon/source→SQLite、SnapshotPublisher/PublishedPair、native UI/REST read-only共有、daemon単独HTTP listenerなし、owner/publisher/DB fault時のlast-good保持を、`REST_API_V1.md`と`DESIGN.md`へ再突合した。RC-076後の現行script・conflict ledger・REST・DESIGN・tracker/cross-scan SHAを監査記録へ更新し、既存の責務境界以外の値は追加していない。machine gateはPASS、intake guardはFAIL。実runtime、同一release lineage、freeze captureが未取得のため、RC-077は`FIXED_PENDING_FRESH_AUDIT`を維持し、現行release PASSへ昇格しない。RC状態は`OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

## RC-078 status/details pair atomicity re-audit (V52)

`REST_API_V1.md`のAtomic status/details client admissionと`DATA_PROTECTION_POLICY.md` §8.1/§8.3を、RC-078 conflict rowへ再突合した。status/detailsは各1回取得し、schema/domain/common-coreとrequest-cycle ID/canonical common-core hashが一致する同一候補だけを一括commitし、片側失敗・不一致では両方discardして直前完全pairを保持する。`auth_required`の可視性消去はdata pair commitと分離したsecurity transitionとして扱う。新しい世代値やfailure経路は追加していない。machine gateはPASS、intake guardはFAIL、実candidate trace・pair generation・auth-clear runtime・same-release lineage・freeze captureは未取得であるため、RC-078は`FIXED_PENDING_FRESH_AUDIT`を維持する。現行RC状態は`OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

## RC-079 all-response header and route matrix re-audit (V53)

`REST_API_V1.md`の全response header、known GET、non-GET=405、unknown/case-altered/normalized/query path=404、read-only effect set、status/details transfer limitsをRC-079 conflict rowと`DATA_PROTECTION_POLICY.md`へ突合した。全responseにexact JSON Content-Type、no-store、8 KiB header aggregateを適用し、fixed Content-LengthをUTF-8 body bytesへ結び付けた。新しいheader、route、上限、failure値は追加していない。machine gateはPASS、intake guardはFAIL。per-route runtime trace、body cutoff計測、same-release lineage、freeze captureが未取得のためRC-079は`FIXED_PENDING_FRESH_AUDIT`を維持する。現行RC状態は`OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

## RC-080 all-endpoint read-only effect boundary re-audit (V54)

`REST_API_V1.md`のRead-only effect setとRC-080 conflict rowを、health/status/detailsの成功、known-path non-GET=405、unknown path=404の全responseへ突合した。許可はrequest-lifetime heap、bounded in-memory counter、loopback socket、read-only open/statだけであり、SQLite/WAL/SHM、PublishedPair、backup/migration/checkpoint、filesystem/process/networkの永続副作用、Windows direct DB accessを禁止する。OS atimeは成功根拠にせず、re-entry副作用増加もFAIL/HOLDとした。新しい副作用分類や値は追加していない。machine gateはPASS、intake guardはFAIL。per-route syscall/effect trace、Windows DB trace、re-entry counters、same-release lineage、freeze capture未取得のためRC-080は`FIXED_PENDING_FRESH_AUDIT`を維持する。現行RC状態は`OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

## RC-081 health actor and polling boundary re-audit (V55)

`WIN-E-012`のSetup/bootstrap/reconnect health probe、`WIN-I-001`のhealth→status→auth-check→ready sequence、`WIN-M-017`のclient ownershipを、RC-081 conflict rowとREST health authorityへ突合した。healthは到達性の判定、通常pollはstatus/details、failureは`HealthUnavailable`と直前last-good保持であり、healthをdata snapshotやDB writerへ昇格させない。新しいactor、route、failure値は追加していない。machine gateはPASS、intake guardはFAIL。Setup/reconnect・normal-poll runtime trace、health failure遷移、same-release lineage、freeze capture未取得のためRC-081は`FIXED_PENDING_FRESH_AUDIT`を維持する。現行RC状態は`OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

## RC-082 current-source bounded reconciliation (V56)

`WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md`のRC-082 projection、`WIN-G-014`/`WIN-F-007`/`WIN-E-011`/`WIN-E-016`の既存oracleを再突合した。初回SetupとSettings再表示のCancel境界で静的に確定できる値だけを記録し、Backの直前step、初回Cancel/Close後のMain disconnected＋Settings recovery、再表示Cancel/Close時の旧6-key bytes完全保持はOPENのまま保持した。推測によるroute・保持値・副作用追加は行っていない。machine gateはPASS、intake guardはFAIL、runtime UIA/keyboard/process/settings、same-release lineage、freeze capture未取得のためRC-082は`OPEN / SETUP-UX`を維持する。現行RC状態は`OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

## RC-083 current-source navigation re-audit (V57)

`WIN-M-006 / WIN-M-007 surface-navigation addendum (RC-083)`と`WIN-M-012`の共有navigation ownerを、Graph/Threads各surfaceのBack/Close可視性、keyboard/UIA到達性、Main route復帰、surface固有保持値、非破壊境界へ突合した。共有行だけでの合格を許さず、各source_idのbounds/focus/key/route/hash evidence scopeを維持した。新しいsurface、route、保持値は追加していない。machine gateはPASS、intake guardはFAIL、実Windows/UIA/画像、same-release lineage、freeze capture未取得のためRC-083は`FIXED_PENDING_FRESH_AUDIT`を維持する。現行RC状態は`OPEN=45 / OPEN_AUTHORITY_CONFLICT=22 / FIXED_PENDING_FRESH_AUDIT=104 / CLOSED=3`。

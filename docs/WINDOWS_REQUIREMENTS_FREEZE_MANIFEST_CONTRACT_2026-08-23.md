# Windows要求抽出 freeze manifest契約（2026-08-23）

状態: `CONTRACT_DEFINED / FREEZE_NOT_CAPTURED`

## 1. 目的

要求文書の作業途中SHAを具体契約へ埋め込むと、正当な要求修正だけで契約自身が即座にstaleになる。
本書は意味契約と証拠取得を分離し、要求抽出をfreezeする瞬間のexact file set、SHA-256、ID集合を
一つのmanifestへ記録する方式を固定する。古いSHA、別時点のファイル、一部だけの再計算をPASSへ使わない。

## 2. 出力と自己参照境界

- 出力: `docs/evidence/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_2026-08-23.json`
- manifest自身は自分のentriesへ含めない。manifest fileのSHAは、manifest生成後の独立監査報告と
  後段release manifestが記録する。
- 本書の状態語やplaceholderは実際のfreeze取得を意味しない。entriesを取得するまでは
  `FREEZE_NOT_CAPTURED`、要求抽出全体は`EXTRACTION_INCOMPLETE`である。
- entries対象を1 byteでも変更した場合は旧manifest全体を失効させ、全entryを同じcaptureで再生成する。

## 3. 必須entry集合

次のexact pathを欠落なく含める。順序はUTF-8 path byteの昇順とする。

1. `AGENTS.md`
2. `DESIGN.md`
3. `LICENSE`
4. `LICENSE.ja.md`
5. `LICENSES/Apache-2.0.txt`
6. `LICENSES/BSD-3-Clause-ANGLE.txt`
7. `LICENSES/MIT.txt`
8. `LICENSES/OFL-1.1.txt`
9. `LICENSES/OPENAI-CODEX-NOTICE.txt`
10. `README.en.md`
11. `README.md`
12. `SECURITY.md`
13. `THIRD_PARTY_NOTICES.md`
14. `VERIFICATION_PLAN.md`
15. `assets/NOTICE.txt`
16. `docs/AGENT_REQUIREMENTS_TRACKER.md`
17. `docs/B2B_RELEASE_ACCEPTANCE.md`
18. `docs/COMPLETION_PROTOCOL.md`
19. `docs/CUSTOMER_OPERATIONS_RUNBOOK.md`
20. `docs/DATA_PROTECTION_POLICY.md`
21. `docs/LIVE_STATE_DECISION_MATRIX.md`
22. `docs/LOCALIZATION.md`
23. `docs/REGRESSION_PREVENTION_POLICY.md`
24. `docs/REQUIREMENTS_AUDIT_2026-08-22.md`
25. `docs/REQUIREMENTS_INTAKE_POLICY.md`
26. `docs/REQUIREMENTS_LEDGER.md`
27. `docs/REST_API_V1.md`
28. `docs/TEST_GAP_REGISTER_2026-08-22.md`
29. `docs/THREAD_PIPELINE_FIXTURE_CONTRACT_2026-08-23.md`
30. `docs/UX_DECISION_ACCESSIBILITY_SCALE_2026-08-23.md`
31. `docs/UX_DECISION_B2B_CUSTOMER_DELIVERY_2026-08-23.md`
32. `docs/UX_DECISION_ERROR_RECOVERY_2026-08-23.md`
33. `docs/UX_DECISION_FULL_STATE_MATRIX_2026-08-23.md`
34. `docs/UX_DECISION_GRAPH_LABELS_2026-08-22.md`
35. `docs/UX_DECISION_HELP_FOCUS_2026-08-23.md`
36. `docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md`
37. `docs/UX_DECISION_KEYBOARD_FOCUS_2026-08-23.md`
38. `docs/UX_DECISION_NON_SCROLL_2026-08-22.md`
39. `docs/UX_DECISION_RELEASE_SUPPLY_CHAIN_2026-08-23.md`
40. `docs/UX_DECISION_SSH_CONNECTION_2026-08-23.md`
41. `docs/WINDOWS_CLIENT.md`
42. `docs/WINDOWS_CLIENT_REQUIREMENTS.md`
43. `docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md`
44. `docs/WINDOWS_LEGACY_REQUIREMENT_CROSSWALK_2026-08-22.md`
45. `docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md`
46. `docs/WINDOWS_REQUIREMENTS_CANONICAL_INDEX_2026-08-23.md`
47. `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`
48. `docs/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md`
49. `docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md`
50. `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`
51. `docs/WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md`
52. `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md`
53. `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md`
54. `docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md`
55. `docs/WINDOWS_UX_SPEC.md`
56. `docs/atomic-contracts/WINDOWS_B2B_ROW_PROJECTIONS_2026-08-23.md`
57. `docs/atomic-contracts/WINDOWS_LEGACY_GAP_PROJECTIONS_2026-08-23.md`
58. `docs/atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md`
59. `docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md`
60. `docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md`
61. `docs/evidence/ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md`
62. `docs/evidence/GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md`
63. `docs/evidence/UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md`
64. `scripts/windows_requirements_extraction_check.sh`
65. `windows-client/THIRD_PARTY_NOTICES.md`

上記65 pathを`requirements content entries`と呼ぶ。fresh独立監査報告が確定した時点で、
その報告ファイルも最終`entries`へ追加する。監査報告は監査開始前に予約した`capture_id`と、
報告自身を除く65 pathから計算した`requirements_content_set_sha256`を参照する。
報告を追加した全entryの`entry_set_sha256`とは別fieldであり、循環参照させない。最終manifest file自身の
SHAは後段release manifestが所有する。

## 4. JSON schema

トップレベルは次のexact keyだけを持つ。

| key | 型・条件 |
| --- | --- |
| `schema_version` | exact string `windows-requirements-freeze-v1` |
| `phase` | exact string `requirements-extraction` |
| `capture_id` | UUID v4 lowercase canonical文字列 |
| `captured_at_utc` | RFC3339 UTC、fractionなし、末尾`Z` |
| `entries` | §3と独立監査報告を含むarray、path昇順、重複0 |
| `requirements_content_set_sha256` | §3の65 pathだけをcanonical列化したSHA-256 lowercase 64-hex |
| `entry_set_sha256` | 下記canonical entry列のSHA-256 lowercase 64-hex |
| `current_requirement_ids` | count=226、sorted ID set SHA |
| `legacy_source_ids` | count=96、sorted source ID set SHA |
| `conflicts` | total、closed、openの非負整数。final freezeはopen=0 |
| `machine_gate` | script path、script SHA、exit_code=0、stdout SHA、実行時刻 |
| `semantic_audits` | A-D/E-I/J-M/legacy/fullの各report path、report SHA、PASS件数、FAIL/INCONCLUSIVE=0 |

各`entries[]`はexact key `path,sha256,bytes`を持つ。`path`はrepo-relative UTF-8、`sha256`は
実file bytesのlowercase 64-hex、`bytes`は0以上の整数である。symlink、missing、directory、path traversalを拒否する。

`entry_set_sha256`は各entryをpath昇順で
`<path>\0<sha256>\0<decimal-bytes>\n`へUTF-8 encodeして連結したbytesのSHA-256とする。
ID set SHAはIDをASCII昇順にし、各IDへLFを付けて連結したbytesのSHA-256とする。

## 5. 取得・検証手順

1. 3領域具体契約とcrosswalkの領域別fresh意味監査をPASSさせ、open conflictを0にする。
2. 機械ゲートを実行しexit 0とstdout bytesを保存する。
3. `capture_id`を予約し、同じ作業tree snapshotから§3の65 content entryのbytes/sha256と
   `requirements_content_set_sha256`を一度に取得してfreezeする。
4. 226/96 ID集合を独立parserで再抽出し、count・set SHA・missing/extra/duplicate 0を記録する。
5. full fresh evaluatorは予約`capture_id`と`requirements_content_set_sha256`をreportへ記載し、
   65 content entriesが変わっていないことを確認して226/96全件を判定する。
6. 全監査reportを追加して最終entriesと`entry_set_sha256`を計算し、manifestをcanonical JSONで書く。
7. 別担当が65 content SHA、report SHA、両set SHA、ID set、gate outputを再計算する。
8. 再計算後にcontent/report対象が変わっていないことをもう一度確認する。変化が1件でもあれば
   manifestと監査reportを失効させ、新しいcapture_idで手順2からやり直す。

## 6. 合否式

`PASS`は次のANDだけである。

```text
required_paths_missing = 0
entry_duplicate_paths = 0
entry_sha_mismatch = 0
entry_byte_count_mismatch = 0
requirements_content_set_sha256_mismatch = 0
current_ids = 226 unique / missing 0 / extra 0
legacy_ids = 96 unique / missing 0 / extra 0
open_conflicts = 0
machine_gate.exit_code = 0
semantic_audits = A-D PASS and E-I PASS and J-M PASS and legacy PASS and full PASS
semantic_audit_FAIL_or_INCONCLUSIVE = 0
post_capture_entry_changes = 0
independent_recalculation = PASS
```

製品artifact、実画像、実DB、実process、物理Windows hostのSHA/証拠は要求抽出manifestへ捏造して
入れず、抽出後のrelease/artifact evidence manifestで同じsource freezeへ結ぶ。

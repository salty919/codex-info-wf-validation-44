# UX Decision: Windows release supply chain and trust boundaries

Decision ID: `UX-20260823-RELEASE-SUPPLY-CHAIN-001`

状態: `OPEN_AUTHORITY_CONFLICT / PRODUCT_PENDING`

## 目的

WindowsのB2B配布、update、rollback、uninstall、Apps登録、license/notice同梱、診断情報、権限境界を、
同じsource releaseとartifact lineageへ結合できる原子契約にする。既存のinstaller lifecycle契約が定める
transaction・journal・公開境界を置き換えず、署名、供給網、版番号、Apps identity、診断保持、Windows権限の
未定義部分だけを決定待ちとして分離する。

この文書は決定候補と証拠schemaを定める要求文書であり、製品実装、実Windows証拠、署名済み成果物、
独立評価、PASS、出荷許可を意味しない。7領域のいずれかが決定されるまで、`OPEN_AUTHORITY_CONFLICT`
および`PRODUCT_PENDING`を維持する。

## 非目的

- Authenticode、Azure Artifact Signing、OV/EV、MSIX、self-signedのいずれかを採用済みとはしない。
- semver、channel、Apps registry optional value、ACL、redaction retention、license manifestの具体値を推測しない。
- 既存の`UX-20260823-INSTALLER-001`が定める15秒、staging、atomic switch、tombstone、journal、Retry/Cancel、
  settings/history保持、crash/power/reboot/re-entry契約を再定義しない。
- `GLOBAL:INSTALL-*`のようにcrosswalkで実体が確認できないtargetを新規正本として発明しない。

## Decision record必須フィールド

### 利用者の課題

顧客は、同じ製品名・版番号に見えるWindows成果物について、正しい発行元の改ざんされていない
payloadか、導入・更新・rollback・削除後も同じ世代か、第三者通知が実payloadを覆うか、診断物が
秘密を含まないか、対象Windowsで検証済みかを画面の印象だけでは判別できない。これらが未定義の
ままでは、動作した1台の結果をB2B配布可能性へ一般化してしまう。

### 代替案と棄却理由

| 代替案 | 現時点の扱いと理由 |
| --- | --- |
| unsignedまたはself-signed成果物を既定で許可 | 発行元・SmartScreen・企業trust policyを満たす根拠がなく棄却。署名なしを暗黙許可しない |
| SHA-256一致だけで全供給網を許可 | 発行元、package/license、version replay、Apps identity、ACL、対応hostを証明できないため棄却 |
| 現行実装値・registry値・hostを事後に正本化 | ユーザー未承認の仕様変更と1台依存を固定するため棄却 |
| 7領域のauthority値を先に決定し、同一lineageのnegative/retention evidenceを要求 | 候補。ただし本書時点では決定者・具体値・実証が未確定なので製品採用済みにしない |

### 採用案

製品方式は未採用である。要求抽出上の暫定決定として、SC-01〜SC-07を
`OPEN_AUTHORITY_CONFLICT / PRODUCT_PENDING`のrelease blockerへ登録し、具体値、失敗時保持、
evidence schema、独立評価が確定する前に既存実装やunsigned成果物を既定値として採用しない。

### X版との関係

SC-01〜SC-07はWindows配布・導入・更新・診断・host受入の境界であり、X版のquota、history、Graph、
Threads、DBの意味を変更しない。Windows固有の配布方式を理由にX/Linuxの値、期間、系列、livenessを
再計算・縮退せず、同じsource releaseとデータfixtureへjoinする。

### 影響要求

`WIN-E-016`、`WIN-H-001..012`、`WIN-I-014..015`、`WIN-L-004`、`WIN-L-008`、
`WIN-L-015`、`WIN-M-015..016`、`WIN-M-027`、`GLOBAL:AUD-011`、`GLOBAL:AUD-020`、
`GLOBAL:AUD-022`。各SC節のtargetが狭い適用範囲を所有する。

### 非スクロール影響

本DecisionはMain/Graph/Threadsへ新しい値や説明を追加しない。将来のinstaller/Setup compatibility・failure
表示は、既存の同一viewport、原因・影響・primary CTA、Back/Cancel/Close、ページ全体scroll 0の正本へ
従う。署名、manifest、ACL、host matrixのraw詳細を監視画面へ詰め込まず、顧客文書とredacted evidenceが
所有する。

### 証拠計画と未確定

証拠計画は各SC節のcandidate schemaと末尾の共通ゲートが所有する。具体的な署名方式・publisher、
package/license mapping、version grammar、Apps key/value、diagnostic retention、token/DACL/reparse policy、
supported Windows cellおよび物理host evidenceは未確定であり、一つでも未確定なら本DecisionはOPENのままとする。

## 正本と証拠の境界

| 境界 | 正本・anchor | この文書での扱い |
| --- | --- | --- |
| 行の完了条件 | `docs/WINDOWS_REQUIREMENTS_CANONICAL_INDEX_2026-08-23.md:66-86` | actor、entry、precondition、failure/last-good、依存、独立oracleが必要。未定義値は未決のまま保持する |
| 要求freeze | `docs/WINDOWS_REQUIREMENTS_FREEZE_MANIFEST_CONTRACT_2026-08-23.md:91-115` | freeze manifestは要求sourceのpath/SHA/bytes/ID集合を所有する。製品署名・payload package graph・host ACLの実測を要求freezeへ混ぜない |
| B2B gate | `docs/B2B_RELEASE_ACCEPTANCE.md:5-27` | fresh Windows install/update/uninstall、artifact SHA、license、security、独立評価が必要。現状のstatusを製品合格へ昇格しない |
| installer lifecycle | `docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md:15-115` | 既存transaction/journal/公開/保持を前提にし、下記7領域の不足だけを追加する |
| artifact evidence | `docs/evidence/ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md:5-8` | source/artifact/evidenceを同一artifact SHAへ結合する。下記schemaはこのmanifestを補完する候補で、実物証拠ではない |
| release/client/security | `docs/RELEASE_MANIFEST_2026-08-22.md:3-30`、`docs/WINDOWS_CLIENT.md:51-66,199-211`、`SECURITY.md:10-40` | 現行releaseはHOLD、installer exact evidenceはPRODUCT_PENDING。security trust assumptionを製品要求へ黙って変換しない |
| concrete contracts | `docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:62-73,87-88`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:48-59,75-76` | H/E-I/J-Mの既存positive/negative oracleを保持し、追加の未定義境界を既存IDへjoinする |

## 共通の決定待ち状態と不変条件

### 7領域共通の状態

```text
SourceRelease
  -> CandidateObserved
  -> AuthorityFieldsEvaluated
AuthorityFieldsEvaluated --open/unknownあり--> AuthorityHold -> LastGoodOrNotInstalledRetained
AuthorityFieldsEvaluated --必須値すべて確定--> DecisionComplete (現時点では未到達)
DecisionComplete
  -> EvidencePending
  -> ProductPending
```

`AuthorityHold`では、実装が選んだ既定値、unsigned許可、downgrade許可、registry optional値、
diagnostic retention、ACL trustを正本として扱わない。決定がない候補はrelease eligibility、公開、
現行要求の抽出完了へ進めない。

### 共通不変条件

1. 検証不能、署名/発行元不一致、provenance/license不一致、版番号不正、Apps identity不一致、秘密値の
   redaction失敗、権限境界不明は、初回なら未導入、既存installなら完全なlast-good世代を保持する。
2. failure表示は既存canonical classへjoinし、raw exception、username、private path、token、SSH情報、
   command lineを新しい表示・log・evidenceへ流さない。
3. installer、payload、installed executable、Apps metadata、shortcut、support artifact、host evidenceは、
   同じsource releaseとartifact-specific SHAを一意にjoinする。異なるartifactのSHAを同一値として扱わない。
4. updateのmetadataだけの切替、旧/新payloadの混在、未承認downgrade、foreign ownerによるcleanup、
   settings/historyへの副作用は0とする。
5. 下記の`evidence`は将来の独立検証schemaであり、placeholder、旧世代、実装者の自己判定を製品証拠としない。

### 依存DAG

```text
source release -> SC-01 signature/publisher trust
source release -> SC-02 package/license provenance
SC-01 + SC-02 -> SC-03 version/update/rollback authorization

host observed -> SC-07 supported Windows matrix
SC-07 -> SC-06 token/ACL/reparse scope

SC-01 + SC-02 + SC-03 + SC-06 + SC-07 -> SC-04 Apps identity/publication
SC-01 + SC-02 + SC-03 + SC-04 + SC-06 + SC-07 -> SC-05 diagnostic redaction/evidence
SC-05 -> B2B release and independent evidence
```

SC-04はSC-01（publisher表示・identity）、SC-02（payload/notice）、SC-03（DisplayVersion/世代）、SC-06
（HKCU/install root権限）、SC-07（edition/build/architecture/capability）へ依存する。SC-05は全操作へ横断適用する
最終evidence boundaryであり、SC-01〜04・06〜07へ逆向きのhard prerequisiteを作らない。SC-07は
initial/update/rollbackと全physical-host evidenceの入口ゲートとし、unsupported hostでの安全なuninstall可否は
SC-07の未決値として残す。DAGの決定未了を後段のartifact証拠で埋め合わせない。

## SC-01: 署名・発行元・信頼チェーン

### 現在のauthority境界

`docs/CUSTOMER_OPERATIONS_RUNBOOK.md:48-49`はWindows WSL Setupが同一の`signed/hash-verified payload`
をstageすると記載するが、`docs/CUSTOMER_OPERATIONS_RUNBOOK.md:104-109`、
`docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md:40-62`、`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:62-63`
はmanifest/SHA/version/runtime/noticeだけを検査する。`docs/RELEASE_MANIFEST_2026-08-22.md:13-18`、
`docs/evidence/ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md:5-8`にもsigner/publisher fieldがない。

これはunsignedを許可する契約ではなく、署名がどのartifactへ、どのtrust policyで必要かが未決である。
Microsoftの一次資料は、公開EXEの署名なし・self-signedがSmartScreen/enterprise policyの強い拒否要因に
なり得ること、publisher identityとfile hashが別々の評価軸であることを説明している。

- [Code signing options for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
- [SmartScreen reputation for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)

### 決定すべき値（未決）

| field | 決定待ち内容 |
| --- | --- |
| `signed_artifact_roles` | installer、embedded payload内の各PE、manifest、server bundle、uninstallerの対象集合 |
| `signature_format` | Authenticode/別配布署名/MSIX等。採用方式は未決であり、無署名を既定値にしない |
| `publisher_identity` | 表示名、subject、issuer、certificate thumbprint、組織identity、更新時の同一性 |
| `trust_policy` | Trusted Root、chain、timestamp、失効、offline/失敗時の扱い、tool/version |
| `verification_phase` | download/embedded extraction/staging/installed executable/rollback/re-entryの各再検証点 |
| `failure_class` | unsigned、invalid、publisher mismatch、chain unavailable、expired/revoked、timestamp invalidのcanonical join |
| `evidence_owner` | signer evidenceとartifact SHA/source releaseの独立reviewer、secret-safe保存先 |

### 状態機械・entry/re-entry

```text
ArtifactProduced
  -> SignatureUnknown
  -> SignatureVerified
  -> PublisherAccepted
  -> ReleaseCandidate
  -> Staged
  -> InstalledOrRolledBack
```

`SignatureUnknown`、`PublisherMismatch`、`TrustUnavailable`、`SignatureInvalid`から`Staged`へ直接遷移しない。
initial install、update、explicit rollback、WSL/remote transfer、Apps re-entryの各entryで同じartifact role集合と
trust policyを再適用し、過去のaccepted判定だけで再入を許可しない。

### 拒否・旧版保持

- 初回: 署名/発行元決定または検証が未完了・不一致ならfinal root、shortcut、HKCU Apps entryを公開せず、
  settings/server/historyを変更しない。
- update/rollback: new candidateを公開せず、完全な`V_old`のroot、shortcut、HKCU、version/hashを保持する。
- 失敗理由は既存installer failure classへ固定joinする。unsignedを「検証済み」と再分類しない。

### 不変条件

`signature_artifact_role_set`、`artifact_sha256`、`source_release_id`、publisher identity、verification resultは
一つのcandidateで一致する。署名後にPE bytesを変更しない。署名対象外のPEが存在する場合は、対象外を許す決定が
明示されるまでcandidateを公開しない。

### evidence schema（候補、実証未取得）

```text
signature_evidence = {
  schema_version,
  source_release_id,
  artifact_role,
  artifact_sha256,
  file_relative_role,
  signature_present,
  signature_format,
  signer_subject_redacted,
  publisher_display_redacted,
  certificate_thumbprint,
  issuer,
  chain_result,
  timestamp_result,
  revocation_policy_id,
  trust_store_id,
  verification_tool_version,
  verified_at_utc,
  result,
  failure_class,
  decision_ref,
  independent_reviewer
}
```

`signature_present=false`の記録は観測値であり、許可判定ではない。private key、raw certificate secret、local
username/path、未redact commandを保存しない。

### RC overlap / target

- overlap: RC-024、RC-032、RC-043、RC-047はSHA/freeze/evidence lineage、RC-093は`signed/hashed`のcustomer update入口を扱う。
  これらはsigner identity、signature scope、trust failure、publisher evidenceを閉じない。直前7 findingsとの意味重複なし。
- targets: `WIN-H-001`, `WIN-H-002`, `WIN-H-005`, `WIN-L-004`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`

## SC-02: payload provenance・license/noticeの実artifact join

### 現在のauthority境界

`docs/B2B_RELEASE_ACCEPTANCE.md:19-27`はlockfile restore、RID、artifact SHA、OSS notices/licensesを要求する。
`THIRD_PARTY_NOTICES.md:40-51,77-87`と`docs/WINDOWS_CLIENT.md:199-211`は実際に含まれる.NET runtime、native/package
noticeの収集と欠落時の配布停止を要求する。しかし`WIN-H-002`はカテゴリ/source/license entryまでで（`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:63`）、
`WIN-L-004`はbinary単位のSHA graphまでである（`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:48`）。

### 決定すべき値（未決）

| field | 決定待ち内容 |
| --- | --- |
| `build_inputs` | source commit、Windows lockfile、restore mode/toolchain、RID/TFM、build environment identity |
| `package_inventory` | package ID/version/license SPDX/source URLとruntime/native assetの許可集合 |
| `payload_file_map` | package/runtime/license/noticeから実payload fileへの1:1または明示many-to-one mapping |
| `exclusions` | test/build-only package、未選択RID、未同梱sourceの扱いとoracle |
| `notice_completeness` | root/package/runtime/font/schema/dependency/distribution noticeの必須集合、本文・copyright・sourceの関係 |
| `re-entry_relation` | update/rollback時にlicense/notice manifestが旧/新payloadと同じ世代へ結合する規則 |
| `failure_class` | lock mismatch、unexpected file、missing notice、unknown license、source unavailableのreject class |

### 状態機械・entry/re-entry

```text
SourceReleaseDeclared
  -> LockAndRestoreCaptured
  -> PackageGraphCaptured
  -> PayloadEnumerated
  -> NoticeLicenseMapped
  -> InstallerEmbedded
  -> LineageRechecked
  -> ReleaseCandidate
```

各update/rollback/WSL/remote transferは`SourceReleaseDeclared`から再開し、前世代のpackage/notice acceptanceだけを
流用しない。payload enumerationとnotice mappingのどちらかが欠けたcandidateは`ReleaseCandidate`へ進めない。

### 拒否・旧版保持

- 初回: package graph、RID、payload inventory、license/notice mappingのどれかが不一致なら、installer公開とApps登録を行わない。
- update: new manifest/payload/noticeの一部だけを採用せず、旧世代の完全なpayload・license/notice・metadataを保持する。
- rollback: 選択したprevious世代の全file/license/notice lineageが一致しない場合、rollbackを実行せず現行世代を保持する。

### 不変条件

`lockfile_digest`、`package_graph_digest`、`payload_file_set_digest`、`notice_license_set_digest`、`artifact_sha256`、
`source_release_id`、`rid`は同一candidateへ結合する。test-only assetをruntime payloadへ混ぜず、runtime/native assetを
noticeなしで同梱しない。installerのembedded payloadとinstalled payloadは同じinventoryである。

### evidence schema（候補、実証未取得）

```text
provenance_license_evidence = {
  schema_version,
  source_release_id,
  source_commit,
  lockfile_path_redacted,
  lockfile_sha256,
  restore_tool_version,
  rid,
  tfm,
  package_graph_sha256,
  packages: [{id, version, license_id, source_id, runtime_asset_ids}],
  payload_files: [{relative_role, sha256, size, package_ids}],
  notice_files: [{relative_role, sha256, license_ids, package_ids}],
  excluded_assets: [{id, reason}],
  installer_embedded_inventory_sha256,
  installed_inventory_sha256,
  result,
  failure_class,
  captured_at_utc,
  independent_reviewer
}
```

path、username、private feed credential、token、raw restore logは保存しない。placeholderやカテゴリ名だけの一覧は
package-level joinの証拠にならない。

### RC overlap / target

- overlap: RC-029はnoticeカテゴリ、RC-032、RC-043、RC-047はSHA/freeze、RC-050、RC-091以降はstaging/transactionを扱う。
  package graph→実payload file→license/notice→installerのjoin、runtime/test exclusion、世代再入は未定義。直前7 findingsとの意味重複なし。
- targets: `WIN-H-001`, `WIN-H-002`, `WIN-L-004`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-022`

## SC-03: version単調性・authorized rollback・anti-replay

### 現在のauthority境界

`docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md:49-62`、`docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md:130-131`、
`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:69-70`は`V_old != V_new`とmanifest/DisplayVersion/file versionの
一致を定める。`docs/CUSTOMER_OPERATIONS_RUNBOOK.md:93-109`はprevious verified世代へのrollbackを定めるが、version grammar、
update order、rollback authorization、replay条件は定めない。

### 決定すべき値（未決）

| field | 決定待ち内容 |
| --- | --- |
| `version_grammar` | versionの型、比較規則、zero/leading/metadata/invalidの扱い |
| `update_order` | 通常updateで許される順序（単調増加か、channel別か） |
| `rollback_authority` | explicit user action、previous verified generation、source release、signature/publisherの必要条件 |
| `same_version_rule` | 同一versionでbytes/hashが異なるcandidateの扱い |
| `replay_freshness` | 同一artifact再適用、old manifest、channel/RID mismatch、nonce/expiryの扱い |
| `failure_class` | malformed/same-version-different-identity/unauthorized-downgrade/replay/channel mismatchを既存classへjoinする規則 |

### 状態機械・entry/re-entry

```text
CandidateReceived
  -> VersionParsed
  -> BaselineCompared
  -> UpdateOrRollbackAuthorized
  -> StagedAndVerified
  -> Published
```

rollbackは`BaselineCompared`から`UpdateOrRollbackAuthorized`へ入る別entryであり、通常updateの比較規則を
黙って再利用しない。re-entry、Retry、resume、Apps起動、WSL/remote転送はcandidateのversion/source/publisher/RIDを
再比較する。`Malformed`、`EqualBytesMismatch`、`UnauthorizedDowngrade`、`Replay`、`ChannelMismatch`は`Rejected`へ遷移する。

### 拒否・旧版保持

- 通常updateで順序判定が未決または不合格ならnew root、shortcut、HKCU DisplayVersionを変更せず、完全な`V_old`を保持する。
- rollback authorizationが未決、不一致、期限/世代証拠欠落ならrollbackせず、現行世代を保持する。
- 同一versionのhash不一致をmetadataだけで成功扱いにせず、既存の`INSTALL_OR_UPDATE_FAILED`へjoinする。

### 不変条件

`manifest.version == DisplayVersion == installed file version`は必要条件に留まり、十分条件とはしない。
公開candidateには`version_relation`、`source_release_id`、`artifact_sha256`、`publisher_identity`、`channel/RID`、
`rollback_authorization`を結合する。update/rollback後にold/new root、shortcut、HKCU metadataが混在しない。

### evidence schema（候補、実証未取得）

```text
version_transition_evidence = {
  schema_version,
  operation_id,
  operation_kind,
  source_release_id,
  baseline: {version, artifact_sha256, publisher_identity, channel, rid},
  candidate: {version, artifact_sha256, publisher_identity, channel, rid},
  version_grammar_id,
  version_relation,
  rollback_authorization_id,
  replay_check_id,
  manifest_display_file_version_equal,
  shortcut_registry_relation,
  result,
  failure_class,
  retained_generation,
  captured_at_utc,
  independent_reviewer
}
```

### RC overlap / target

- overlap: RC-091、RC-093はcustomer updateと旧版保持、RC-024、RC-032、RC-043はartifact SHA、RC-103はcrash/reboot resumeを扱う。
  version grammar、通常updateの単調性、explicit rollback authorization、replay/freshnessは未定義。直前7 findingsとの意味重複なし。
- targets: `WIN-H-001`, `WIN-H-002`, `WIN-H-005`, `WIN-H-008`, `WIN-H-009`, `WIN-L-004`, `WIN-L-015`, `GLOBAL:AUD-011`

## SC-04: Windows Apps登録のcanonical identity

### 現在のauthority境界

`docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md:15-17,31-33,43-47,64-78`と
`docs/CUSTOMER_OPERATIONS_RUNBOOK.md:113-123`はApps entry/HKCU registrationを通常uninstall入口・公開metadataとして扱う。
しかし`WIN-H-007`はHKCU scope、DisplayName、InstallLocation、UninstallString、version equalityまでで、product key、
Publisher、quiet command、duplicate/orphan policyはない（`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:68`）。
`docs/WINDOWS_CLIENT.md:58-62`もexact registration/path/evidence未取得とする。

### 決定すべき値（未決）

| field | 決定待ち内容 |
| --- | --- |
| `product_key` | HKCU uninstall keyのcanonical product identityと世代を跨ぐ安定性 |
| `required_values` | DisplayName、Publisher、DisplayVersion、InstallLocation、UninstallString、quiet/interactive semantics、必要な追加値 |
| `scope` | 現行のHKCU/per-user/HKLM=0を維持する値とcross-user visibility policy |
| `identity_join` | Apps keyとinstaller/payload/installed file/shortcut/signer/source releaseのjoin |
| `failure_recovery` | missing、duplicate、orphan、foreign-target、write-failure、stale-version時の旧entry保持とApps再入 |
| `evidence_redaction` | key path/user segmentをどのようにpseudonymous化して、同一entryを再比較するか |

### 状態機械・entry/re-entry

```text
NotRegistered
  -> RegistrationCandidateValidated
  -> HKCURegistrationPending
  -> Registered
  -> Updating / Uninstalling
  -> Reconciled
```

`RegistrationCandidateValidated`ではinstall root、shortcut target/cwd、installed file version/hash、manifest、
publisher identityを同一candidateへjoinする。updateは同一product identityのentryを旧→新へ一度だけ更新し、uninstall
failureはjournal/Apps entryから同じoperationを再開する。missing/duplicate/orphan/target mismatchは`ReconcileRejected`へ遷移し、
新しいentryやforeign keyを自動生成しない。

### 拒否・旧版保持

- initial: HKCU key/required values/target relationが未検証ならshortcutとApps entryを公開しない。
- update: new registrationが完全でない場合、旧HKCU entry、shortcut、旧version/hashを保持する。
- uninstall: binary/shortcut/HKCUの全不存在またはjournalによる再開条件を確認できるまで成功表示しない。settings/historyは保持する。

### 不変条件

同一operationの`product_key`、scope、InstallLocation、UninstallString target、DisplayVersion、installed file version、
shortcut target version、artifact SHAは一致する。HKLM entryを暗黙に作らず、foreign user/key/targetをcleanupしない。

### evidence schema（候補、実証未取得）

```text
apps_registration_evidence = {
  schema_version,
  host_evidence_id,
  operation_id,
  operation_kind,
  product_key_pseudonym,
  registry_scope,
  required_values_redacted,
  publisher_identity_ref,
  install_root_redacted,
  uninstall_target_redacted,
  shortcut_target_redacted,
  display_version,
  installed_file_version,
  installed_sha256,
  hkcu_before_digest,
  hkcu_after_digest,
  hklm_entry_count,
  duplicate_orphan_count,
  result,
  failure_class,
  captured_at_utc,
  independent_reviewer
}
```

raw username、profile path、full registry path、command line、credentialsは保存しない。`product_key_pseudonym`は
同一host/evidence内で再比較できるが、利用者identityを復元できない値でなければならない。

### RC overlap / target

- overlap: RC-091、RC-092は公開・削除途中、RC-102..105はcrash/reboot/re-entry/owner、RC-024はhost manifestを扱う。
  Apps canonical key/value identity、publisher/display metadata、duplicate/orphan evidenceは未定義。直前7 findingsとの意味重複なし。
- targets: `WIN-H-004`, `WIN-H-005`, `WIN-H-007`, `WIN-H-008`, `WIN-H-009`, `WIN-H-010`, `WIN-H-011`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`

## SC-05: installer journal・diagnostic redaction/retention

### 現在のauthority境界

`docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md:88-99`はraw exception、Windows username、private path、token、SSH情報を保存せず、
failure class/step/version/redacted target kind/exit codeだけを保存するとする。一方、同書`106-110`はprocess identity、
staging/final/rollback/tombstone/journal、shortcut/HKCU、version/hash等を証拠採取対象とする。
`docs/CUSTOMER_OPERATIONS_RUNBOOK.md:179-184`はsupport共有項目を限定するが、journal、Windows Event Log、crash dump、
installer stdout/stderr、support exportの寿命・redaction schemaはない。`WIN-I-014..015`と`WIN-M-015..016`（
`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:87-88`、`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:75-76`）はREST/UIのgeneric secret scanであり、installer operation lifecycleの証拠ではない。

### 決定すべき値（未決）

| field | 決定待ち内容 |
| --- | --- |
| `channel_inventory` | UI、product logger、journal、stdout/stderr、Event Log、crash dump、temp/rollback、registry、support export |
| `allowed_fields` | operation identity、phase、failure class、version、redacted target kind、exit code、artifact SHA、process identity等の許可集合 |
| `redaction_policy` | username/path/nonce/argv/commandline/remote target/certificate/raw bodyの検出・変換・拒否規則 |
| `retention` | success、failure、cancel、crash/reboot、resume、support exportごとの保存・削除・再利用期間 |
| `access/export` | owner、independent reviewer、customer supportへの共有境界と再識別不可条件 |
| `failure_class` | redaction failure、raw persistence、incomplete cleanup、unavailable evidenceのcanonical join |

### 状態機械・entry/re-entry

```text
OperationStarted
  -> StructuredJournalWriteBoundary / ExternalCandidateCaptureBoundary
  -> RedactionValidated
  -> RedactedJournalOrEvidenceStored
  -> SupportExportEligible
  -> RetentionExpiredOrDeleted
```

installer/update/uninstallの各fault、Retry、Cancel、crash/reboot/resume、Apps re-entryで同じchannel inventoryを通す。
`RawDetected`、`RedactionFailed`、`UnknownChannel`は`EvidenceRejected`へ遷移し、raw内容をsuccess artifact、support export、
次世代journalへ流さない。journalはraw captureを経ず、再開に必要なtyped allowlist fieldsだけを決定済みschemaで直接保持する。
外部process由来candidateだけをowner-only一時領域で検査し、失敗candidateはbounded purgeまたは削除不能状態へ進める。

### 拒否・旧版保持

- redaction完了を証明できない場合、診断artifactを受入証拠として扱わず、operationの成功表示・new publicationを抑止する。
- update/uninstall中のdiagnostic failureはpayload/registry cleanupの成功へ丸めず、既存transactionの旧版/last-good保持へjoinする。
- raw secret/pathを発見した場合は、既存の`SECRET_SCAN_FAILED`相当の固定classへjoinする値を決定するまで未決とする。

### 不変条件

全channelのsecret sentinel、raw exception、private path、username、token、SSH情報のoccurrenceは0である。
redacted artifactの各fieldはallowlistとredaction policy versionへjoinし、同じoperationのjournal epoch、artifact SHA、capture時刻を
再比較できる。raw payloadを後で削除するだけではredaction合格としない。

### evidence schema（候補、実証未取得）

```text
diagnostic_redaction_evidence = {
  schema_version,
  operation_id,
  operation_kind,
  journal_epoch,
  channel,
  artifact_sha256,
  redaction_policy_id,
  allowed_field_set_id,
  raw_present,
  secret_sentinel_occurrence_count,
  forbidden_field_occurrence_count,
  redacted_record_digest,
  retention_action,
  export_actor_pseudonym,
  captured_at_utc,
  result,
  failure_class,
  independent_reviewer
}
```

`raw_record_digest`はraw payload復元に使える可逆情報であってはならない。support artifactの共有値は既存runbookの
version/OS/repro/UTC/exit/fixed class/SHA/process stateの決定待ち集合を超えない。

### RC overlap / target

- overlap: RC-023、RC-090、`WIN-I-014..015`、`WIN-M-015..016`はerror class、REST/SSH/UI redactionを扱う。RC-102..106はjournal/re-entry/owner lifecycleを扱う。
  installer全channel inventory、redaction failure、retention/export、raw persistence evidenceは未定義。直前7 findingsのfailure-class driftとは分離する。
- targets: `WIN-E-016`, `WIN-I-014`, `WIN-I-015`, `WIN-H-008`, `WIN-H-009`, `WIN-H-010`, `WIN-H-011`, `WIN-H-012`, `WIN-M-015`, `WIN-M-016`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`

## SC-06: Windows token・ACL・cross-user/reparse boundary

### 現在のauthority境界

`WIN-H-003`はstandard user、elevation prompt 0、HKLM write 0を定める（`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:64`）。
UXはper-user pathとowner限定stagingを要求する（`docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md:40-47,69-77`）。
一方、`SECURITY.md:14,19`はsame-UID local administrationをtrustedとし、Windows ACLをapplication security boundaryとして扱わない。
これはACL強化を自動採用すべきという意味ではなく、B2B standard-user promiseとsecurity trust assumptionの適用範囲が未決という
authority conflictである。

### 決定すべき値（未決）

| field | 決定待ち内容 |
| --- | --- |
| `process_token` | Setup/client/uninstaller/server bootstrapのuser SID、integrity level、elevation/UAC、helper processの許可範囲 |
| `filesystem_scope` | install root、staging、journal、rollback、tombstone、Start Menu、settingsのowner/DACL/inheritance |
| `registry_scope` | HKCU keyのowner/ACL、HKLM access拒否、cross-user read/write policy |
| `reparse_policy` | junction/symlink/reparse point、foreign target、path replacementを検出・拒否する規則 |
| `foreign_owner` | 別PID/user/operationによるcleanup・commit・lease takeoverの拒否条件 |
| `failure_recovery` | ACL write failure、read-only、UAC prompt、foreign owner、reparse検出時の旧版保持と再入 |
| `evidence_boundary` | SID/token/ACLをprivate identityなしに独立比較するschema |

### 状態機械・entry/re-entry

```text
InstallContextUnresolved
  -> TokenValidated
  -> ScopeValidated
  -> ObjectHandleSnapshotValidated
  -> ACLAndReparseValidated
  -> SameHandleMutationAuthorized
  -> Reconciled
```

各initial/update/rollback/uninstall/Apps re-entryでtoken、owner identity、canonical path、ACL、reparse状態を再検証する。
`ElevatedUnexpectedly`、`ForeignOwner`、`ACLWriteFailed`、`ReparseDetected`、`ScopeUnknown`は`MutationRejected`へ遷移し、
検査後に変化したforeign objectを削除・commitしない。検証済みhandleまたはそのhandle相対のOS操作を使えない対象はmutation 0とする。
same-UID trustを採用するか、product ACL boundaryとして扱うかは未決である。

### 拒否・旧版保持

- 初回: token/ACL/path scopeが未決または検証失敗なら、staging/final root/shortcut/HKCUを公開しない。
- update/rollback/uninstall: foreign owner、unexpected elevation、reparse、ACL不一致があればmutationを開始せず、完全な旧世代と
  settings/historyを保持する。owner takeoverは既存installer lifecycleのidentity/journal条件を満たす場合だけ候補とする。
- HKLM write、cross-user cleanup、foreign journal/tombstone削除を成功処理へ丸めない。

### 不変条件

`elevation_prompt_count=0`、`HKLM_write_count=0`という既存H003条件を維持しつつ、token/ACL/reparse決定が完了するまで
per-userを安全境界と断定しない。mutation主体のidentityと対象objectのowner identityが一致し、検査前後のpath/inode/file hash/
registry digestが同じoperationへ結合する。foreign objectは保持し、推測削除しない。

### evidence schema（候補、実証未取得）

```text
windows_privilege_evidence = {
  schema_version,
  host_evidence_id,
  operation_id,
  process_identity_pseudonym,
  user_sid_pseudonym,
  token_type,
  integrity_level,
  elevation_prompt_count,
  helper_process_count,
  install_root_acl_digest,
  staging_acl_digest,
  journal_acl_digest,
  start_menu_acl_digest,
  hkcu_acl_digest,
  hklm_write_count,
  reparse_scan_result,
  foreign_owner_count,
  path_scope_result,
  result,
  failure_class,
  captured_at_utc,
  independent_reviewer
}
```

raw username、hostname、full path、private SID、token handle、credential、command lineは保存しない。ACL digestは同一hostの
許可された再比較に十分で、利用者identityを復元できない表現を決定する必要がある。

### RC overlap / target

- overlap: H003はno-admin/HKLM=0、RC-091..106はinstaller transaction/owner/re-entryを扱う。`SECURITY.md`のtrust assumptionは
  Windows product ACL specificationではない。token/IL、DACL/inheritance、cross-user、reparse、foreign cleanup evidenceは未定義。
  直前7 findingsとの意味重複なし。
- targets: `WIN-H-003`, `WIN-H-006`, `WIN-H-007`, `WIN-H-008`, `WIN-H-009`, `WIN-H-010`, `WIN-H-011`, `WIN-H-012`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`

## SC-07: supported Windows matrix とphysical-host evidence cell

### 現在のauthority境界

`docs/WINDOWS_CLIENT.md:201-204`、`docs/WINDOWS_REQUIREMENTS_VALUE_AUTHORITIES_2026-08-22.md:130-133`、
`docs/B2B_RELEASE_ACCEPTANCE.md:10,20-27`はclean supported Windows、`win-x64` self-contained、RID、実Windows導入を要求する。
`WIN-H-001`はWindows x64を前提とし（`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md:62`）、
`WIN-L-015`はarchitectureを`<x64-or-supported-arch>` placeholderとしている（`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:59`）。
しかしedition、minimum build、support end、locale、display capability、WSL/OpenSSH capability、offline installの
matrixと、matrix外でmutationを禁止するentry gateは定義されていない。

これは特定のWindows edition/build/architecture/localeをサポート済みと確定する項目ではない。未決のまま
`clean supported Windows`を顧客環境へ一般化せず、matrix外の実行を製品証拠やinstall成功へ昇格しない。

### 決定すべき値（未決）

| field | 決定待ち内容 |
| --- | --- |
| `edition_set` | Windows edition SKU/variant、client/server/IoT等の適用範囲 |
| `build_range` | minimum/maximum build、servicing branch、support end、out-of-support時の扱い |
| `architecture_set` | `win-x64`のみか、arm64等を許可するか。installer/payload/RIDの対応関係 |
| `locale_set` | supported locale、fallback、非Unicode/文字表示 capability、localeごとのnotice/error evidence |
| `display_capability` | minimum logical work area、DPI/scale、multi-monitor、color/font/rendering capability、unsupported predicate |
| `connection_capability` | WSL version/distribution、OpenSSH client/version/config、Windows Apps/Start Menu prerequisites |
| `offline_install` | offline payload、certificate/trust time、runtime/license inventory、network-free setup/rollbackの許可範囲 |
| `support_end` | matrix cellのsupport終了後にinstall/update/uninstallを拒否するか、既存installを保持するか |
| `mutation_gate` | matrix probe前後で許可するwriteと、unsupported/unknown時の全mutation=0規則 |

### 状態機械・entry/re-entry

```text
HostObserved
  -> CapabilityProbe
  -> MatrixClassified
  -> SupportedCellAuthorized
  -> InstallerOperationAuthorized
  -> PhysicalEvidenceCaptured
```

`MatrixClassified`の結果は`Supported`、`Unsupported`、`Unknown`の3値とし、edition/build/architecture/locale/display/
connection/offlineの各軸を欠落させない。`Unsupported`または`Unknown`は`MutationRejected`へ遷移し、initial install、update、
rollback、uninstall、Apps repair、WSL/remote bootstrapを開始しない。re-entryでは同じhost evidence idだけを信用せず、
edition/build/support end、capability、artifact SHAを再取得する。

### matrix cellとentry条件

matrixの1 cellは、少なくとも次の直積を持つ。実際の値は決定者が確定するまでplaceholder/fixture-onlyである。

```text
cell = {
  windows_edition,
  windows_build,
  architecture,
  locale,
  display_capability,
  wsl_capability,
  openssh_capability,
  apps_start_menu_capability,
  network_mode=(online|offline),
  support_window,
  matrix_result
}
```

各cellでinstaller/payload/RID、署名・license manifest、version/rollback、Apps registration、ACL、UI geometryを同じ
source release/artifact SHAへ結合する。`clean VM`、WSL/Linux directory、folder-openだけの代替はphysical-host cellを満たさない。

### 拒否・旧版保持・matrix外mutation=0

- `Unsupported`/`Unknown` cellではinstaller payload展開、staging、final root rename、shortcut、HKCU、settings/history、
  WSL/remote transfer、service mutationをすべて0とし、固定されたbounded compatibility classだけを表示する。
- 既存installのupdate/rollback/uninstallでcellがsupport endを越えた、または再probe不能な場合は、決定済みpolicyがない限り
  new mutationを開始せず、完全なlast-good root/shortcut/HKCU/settings/historyとdiagnostic journalを保持する。
- `Supported`でもedition/build/capabilityの一軸が未取得なら`Unknown`として扱い、成功表示・fresh evidence PASSへ進めない。

### 不変条件

1. `matrix_result=Supported`は全軸の具体値、判定規則version、判定時刻、artifact/source releaseへのjoinがある場合だけ成立する。
2. Windows build/edition/architectureの判定を、installer filename、`win-x64`、過去のhost manifest、またはUI画像だけから推測しない。
3. display capabilityがsupported predicate外なら、font縮小、clip、scroll、画面外描画でmatrix内へ偽装しない。
4. offline cellではネットワーク依存のrestore、runtime取得、certificate取得、license/notice収集を暗黙に開始しない。
5. WSL/OpenSSH未取得時に、profileを使えると仮定してsetup/install/reconnectを自動実行しない。

### physical-host evidence cell（候補schema、実証未取得）

```text
windows_support_matrix_evidence = {
  schema_version,
  host_evidence_id,
  source_release_id,
  artifact_sha256,
  installer_sha256,
  payload_sha256,
  installed_sha256,
  windows_edition,
  windows_build,
  architecture,
  locale,
  display_capability: {
    logical_work_area,
    dpi_set,
    monitor_topology_id,
    rendering_font_capability,
    supported_predicate_result
  },
  wsl_capability: {present, version, distribution_selector_result},
  openssh_capability: {present, version, config_alias_result},
  apps_start_menu_capability,
  network_mode,
  support_window,
  matrix_result,
  mutation_counts: {
    staging,
    final_root,
    shortcut,
    hkcu,
    settings_history,
    wsl_remote_transfer,
    service
  },
  captured_at_utc,
  result,
  failure_class,
  independent_reviewer
}
```

`host_evidence_id`はpseudonymousで、hostname、Windows username、raw profile path、credential、private network detailを含めない。
placeholder値はschema completenessにのみ使え、Supportedの事実証拠にはならない。

### RC overlap / target

- overlap: RC-024、RC-032、RC-043、RC-050はhost/artifact freshness、SHA、staging/DPI evidenceの一部を扱う。RC-120はsupported Windows
  matrixの未決を明示するためのentryであり、edition/build/support end、capability直積、offline、matrix外mutation=0、
  physical-host cellは未定義。直前7 findingsとの意味重複なし。
- targets: `WIN-H-001`, `WIN-H-003`, `WIN-H-004`, `WIN-H-007`, `WIN-H-008`, `WIN-H-009`, `WIN-L-008`, `WIN-L-015`, `WIN-M-027`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`

## Fresh atomic closure additions（RC-114〜120監査反映、未決のまま）

以下は先行する `OPEN_AUTHORITY_CONFLICT / PRODUCT_PENDING` を解消するものではない。各SCに対して、先行監査で検出した状態、入力、失敗時保持、idempotence、identity、独立oracle、evidence schema、DAGの欠落を原子項目として追加する。具体値、署名方式、trust policy、retention期間、対応Windows集合、recovery入口の採択は決定待ちであり、未決項目が残る間は候補を `DecisionComplete`、`ReleaseCandidate`、`SupportedCellAuthorized`、公開可能状態へ進めない。

### SC-01追加契約: role/member digest と signer rotation

#### 状態・入力・失敗・再入

署名状態は次の辺を持つ。既存のhappy pathを置き換えず、role/member単位の検証境界を明示する。

```text
ArtifactProduced
  -> SignatureScopeEnumerated(role_set, member_manifest)
  -> SignatureVerified(role, signed_bytes_digest, member_digest)
  -> PublisherAccepted(publisher_generation)
  -> ReleaseCandidate

PublisherAccepted --signer generation changes--> RotationAuthorized(old_publisher_generation, new_publisher_generation)
RotationAuthorized -> SignatureScopeEnumerated(new publisher generation)
SignatureScopeEnumerated -> SignatureVerified(all roles under new generation) -> PublisherAccepted -> ReleaseCandidate
```

`SignatureScopeEnumerated`への入力は installer container、installer内の各embedded PE、manifest、server bundle、
uninstaller、payload member path、signed bytes digest、member digest、source release、release manifestである。
roleの欠落、member digestの不一致、containerとembedded payloadのbinding不一致、repack、署名未検証、publisher不一致、
chain/timestamp/revocation不明は `RoleCoverageRejected`、`MemberDigestRejected`、`PublisherRejected`、または
`RotationRejected`へ送り、初回は未導入、既存installは完全なlast-goodを保持する。失敗をunsigned許可へ丸めない。

同一 `source_release_id + release_manifest_id + role_set_digest + member_manifest_digest + artifact_sha256` の再入は、
同じ候補 generation に対する検証を一度だけ記録し、publication/actionを増やさない。証明書のthumbprintだけが変わる
rotationは、旧generationから新generationへの明示された承認・失効・有効期間・適用範囲がなければreplayと同じく拒否する。
identityは `source_release_id`、`release_manifest_id`、role、member relative role、signed bytes digest、member digest、
publisher/certificate generation、rotation authorization、operation generationで結合する。

#### evidence schema追加とoracle

既存 `signature_evidence`へ、少なくとも次の未決fieldを追加する。fieldの具体形式・redaction・許可値はauthority decisionで決める。

```text
signature_scope_binding = {
  release_manifest_id,
  role_set_digest,
  member_manifest_digest,
  members: [{member_role, relative_role, signed_bytes_sha256, member_sha256,
             container_binding_digest, signature_coverage_result}],
  publisher_generation,
  predecessor_publisher_generation,
  rotation_authorization_id,
  rotation_authorized_by_role,
  rotation_target_role_set_digest,
  rotation_issued_at_utc,
  rotation_expires_at_utc,
  predecessor_revocation_or_grace_result,
  rotation_epoch,
  rotation_consumed_count,
  repack_detection_result,
  verification_attempt_generation
}
```

独立oracleは role集合とmember集合の完全性、container/member digestの1:1 relation、署名後bytes不変、旧/新 signerの
rotation relation、同一候補のpublication/action countを再計算する。placeholder、署名なし観測値、旧世代のsigner evidenceを
新世代へ流用した記録は受理しない。対象は `WIN-H-001..002`、`WIN-H-005`、`WIN-L-004`、`WIN-L-015`、
`GLOBAL:AUD-011`、`GLOBAL:AUD-020`。既存 overlap は `RC-114`、`RC-024/032/043/047`、`RC-093`であり、これはrole binding/
rotation closureの追加である。

### SC-02追加契約: provenance mapping と license/notice lineage

#### 状態・入力・失敗・再入

SC-02の状態は、payload/noticeの集合とmappingを検証した後だけ次へ進む。

```text
SourceReleaseDeclared
  -> LockAndRestoreCaptured
  -> PackageGraphCaptured
  -> PayloadEnumerated
  -> ProvenanceMappingValidated
  -> NoticeLicenseMapped
  -> InstallerEmbedded
  -> EmbeddedInstalledLineageRechecked
  -> ReleaseCandidate
```

入力は source commit、lockfile/restore/toolchain、package ID/version、license identifier、package source、graph edge、
RID/TFM、runtime/native asset、実payload relative role/SHA/size、notice/license text/source/copyright digest、
test/build-only exclusion、installer embedded inventory、installed inventoryである。package source/licenseが欠落・unknown、
graph edgeが欠落、payload fileまたはnoticeが重複、mapping cardinalityが不正、RID外またはtest-only assetが混入、
embedded/installed inventoryが不一致なら `ProvenanceRejected` とし、初回は公開0、update/rollbackは旧世代の
payload/license/notice/metadataを保持する。

同一 `source_release_id + lockfile_digest + package_graph_digest + payload_file_set_digest + notice_license_set_digest + RID/TFM`
の再入は同じ mapping generationへ一度だけ適用する。update/rollbackは旧/新世代のmappingを混ぜず、同じgenerationの
installer embedded inventoryとinstalled inventoryだけを受理する。identityはpackage graph、mapping、payload file、notice
file、installer artifact、installed artifactの各digestを結合する。

#### evidence schema追加とoracle

既存 `provenance_license_evidence` の `source_id`/`license_id` はopaque値のままでは判定に使わない。次のmapping recordを追加し、
未決のsource URL、license表現、private feedのredaction規則をauthorityで確定する。

```text
provenance_mapping = {
  release_manifest_id,
  installer_artifact_sha256,
  package_graph_edges: [{edge_id, parent_package_id, child_package_id, edge_kind}],
  packages: [{package_id, version, license_spdx_or_authority_id, source_url_or_source_id,
              runtime_asset_ids, package_record_digest}],
  payload_map: [{mapping_id, package_id, asset_id, relative_role, sha256, size, map_cardinality}],
  notice_map: [{mapping_id, package_id, license_id, notice_relative_role, text_digest,
                copyright_digest, source_digest}],
  excluded_assets: [{asset_id, reason, exclusion_rule_id}],
  mapping_result,
  mapping_generation
}
```

独立oracleは canonical orderingを固定して package graph edge、package source/license、payload/notice mapping、除外理由、
embedded/installed inventory、artifact SHAを再生成する。カテゴリ名だけ、package IDだけ、または本文のないnotice一覧は
mapping evidenceとみなさない。対象は `WIN-H-001..002`、`WIN-L-004`、`WIN-L-015`、`GLOBAL:AUD-011`、`GLOBAL:AUD-022`。
既存 overlap は `RC-115`、`RC-029/032/043/047/050/091`であり、これはschemaのsource/license/mapping観測可能性を補う。

### SC-03追加契約: same-version no-op と一回限り rollback authorization

#### 状態・入力・失敗・再入

version transitionは同一versionの二種類を分ける。

```text
CandidateReceived
  -> VersionParsed
  -> BaselineCompared

BaselineCompared --same version and same bytes--> SameVersionSameBytesNoOp -> LastGoodRetained
BaselineCompared --strictly ordered update or explicitly authorized rollback--> UpdateOrRollbackAuthorized
  -> StagedAndVerified
  -> Published

BaselineCompared --same version but different bytes--> Rejected
BaselineCompared --malformed/same-version-different-identity/unauthorized-downgrade/replay/channel mismatch--> Rejected
```

入力は version grammar、baseline/candidate manifest/file/artifact SHA、source release、publisher、channel/RID、operation
generation、candidate generation、explicit rollback intent、authorization actor pseudonym、target generation、nonce/expiry policyである。
same versionかつ同一bytesは publication、shortcut、HKCU、process、history writeを0とするno-op。same version別bytes、
unauthorized downgrade、stale target、replay、channel/RID/publisher/provenance mismatchは `Rejected` とし、完全なbaselineを保持する。

rollback authorizationは `operation_id + candidate_generation + target_generation` にbindし、明示操作を一度だけ消費する。
Retry、Apps re-entry、resume、WSL/remote transferは新しいattemptとしてbaseline/candidateを再比較し、旧attemptのcompletionはno-op。

#### evidence schema追加とoracle

既存 `version_transition_evidence`へ次を追加する。nonce/expiryやactorの許可形式は未決であり、値を推測しない。

```text
version_transition_atomic = {
  baseline_manifest_sha256,
  candidate_manifest_sha256,
  comparison_input_digest,
  operation_generation,
  candidate_generation,
  same_version_disposition,
  rollback_intent_ref,
  rollback_authorization_ref,
  rollback_authorized_actor_ref,
  rollback_authorization_nonce_digest,
  rollback_authorization_issued_at_utc,
  rollback_authorization_expires_at_utc,
  rollback_authorization_consumed_count,
  target_generation,
  replay_nonce_or_policy_ref,
  stale_candidate_rejected_count,
  publication_count,
  shortcut_registry_action_count
}
```

独立oracleは grammar parse、version relation、same-version no-op count、rollback authorizationの消費回数、stale candidateの
拒否、旧/新 root/shortcut/HKCU/hash lineageを比較する。対象は `WIN-H-001..002`、`WIN-H-005`、`WIN-H-008..009`、
`WIN-L-004`、`WIN-L-015`、`GLOBAL:AUD-011`。既存 overlap は `RC-116`、`RC-091/093/103`、`RC-024/032/043`であり、
これはgrammarそのものではなく同一候補再入と一回限り承認のclosureである。

### SC-04追加契約: Apps/matrix reject からの signed recovery/uninstall 経路

#### 状態・入力・失敗・再入

`ReconcileRejected` と `MatrixRejected` を永久停止のままにせず、採択前の安全な recovery decisionを明示する。方式・入口・
署名方式は未決だが、次の遷移と観測境界を欠落させない。

```text
NotRegistered
  -> RegistrationCandidateValidated
  -> HKCURegistrationPending
  -> Registered
  -> Updating / Uninstalling
  -> Reconciled

RegistrationCandidateValidated --missing/duplicate/orphan/foreign/mismatch--> ReconcileRejected
ReconcileRejected -> SignedRecoveryRequired -> RecoveryCandidateValidated
RecoveryCandidateValidated -> UninstallOrRepairAuthorized -> Reconciled
```

入力は Apps key、scope、DisplayName/Publisher/DisplayVersion、InstallLocation、UninstallString token、shortcut target、
installed file/version/hash、journal/owner、host matrix result、signed recovery artifact、明示 uninstall intentである。Apps
identityまたはmatrixが不明な場合は foreign mutation、key自動生成、推測削除を0にし、完全なlast-goodと bounded blocked reasonを
保持する。安全な署名済み recovery/uninstall入口が未決のままなら、uninstall成功表示や次操作への暗黙遷移を許可しない。

同一 product identity＋operation generation のrecovery/re-entryは一度だけ行い、duplicate Apps entry、foreign key、stale
uninstall targetを作らない。identityは `product_key`、scope、source release、installer/payload/signer generation、install root、
shortcut、journal/owner、host matrix evidenceで結合する。`product_key_pseudonym`は同一hostだけでなく同一世代を再比較できる
非復元値でなければならない。

#### evidence schema追加とoracle

既存 `apps_registration_evidence`へ次を追加する。signed recovery方式、許可者、command token/quote規則、pseudonym方式は未決である。

```text
apps_recovery_identity = {
  source_release_id,
  release_manifest_id,
  installer_artifact_sha256,
  installed_artifact_sha256,
  signed_recovery_ref,
  recovery_generation,
  product_key_identity_ref,
  uninstall_authorization_ref,
  matrix_evidence_ref,
  blocked_reason,
  duplicate_orphan_foreign_counts,
  recovery_action_count,
  uninstall_action_count
}
```

独立oracleは exact registry exportのtokenized values、同一product keyの世代join、signed recovery artifact、Apps/shortcut/file
target一致、foreign mutation=0、recovery/uninstall action count、settings/history hashを再計算する。対象は `WIN-H-004..012`、
`WIN-L-015`、`WIN-M-015..016`、`GLOBAL:AUD-011`、`GLOBAL:AUD-020`、`WIN-INSTALL-04`。既存 overlap は `RC-117`、
`RC-120`、`RC-104/105`であり、これはrejectから安全なuninstall/recoveryへ到達する経路の追加である。

### SC-05追加契約: journal と support export の分離、redaction、retention

#### 状態・入力・失敗・再入

Recoveryに必要なjournalと顧客共有support exportは別の境界で処理する。

```text
OperationStarted
  -> StructuredJournalWriteBoundary / SupportExportCaptureBoundary
  -> RedactionValidated
  -> JournalSafeStored / SupportExportEligible
  -> RetentionScheduled
  -> RetentionExpiredOrDeleted

RawDetected / RedactionFailed / UnknownChannel -> EvidenceRejected
EvidenceRejected -> RawCandidateQuarantined -> RawCandidatePurgedOrDeletionBlocked
```

入力はchannel、operation kind、phase、journal epoch、artifact/source lineage、allowed field set、redaction policy、export actor、
retention policy、ACLである。journal writerはraw command/outputを一度保存してからredactする方式を使わず、resumeに必要な
owner/phase/generationだけをtyped allowlist fieldとして直接書く。support exportにはそれを再識別不能な表現へ変換する。
redaction不能、未知channel、raw secret/path/argv/nonce、ACL不明、retention期限不明は evidence reject とし、rawをsuccess artifact、
support export、次世代journalへ流さない。外部processが作ったraw candidateはowner-only隔離後にbounded purgeし、削除不能を
cleanup完了へ丸めない。product transactionのcommit可否とsupport evidenceの受理可否は別結果として保持する。

同一 `operation_id + journal_epoch + channel + capture_generation` は一度だけredact/store/export/deleteする。Retry、Cancel、
crash/reboot/resume、Apps re-entryで古いredacted recordを新世代へコピーせず、late exportはno-opとする。identityは journal-safe
record と support-safe record を別 generation として、artifact/source/operationへ結合する。

#### evidence schema追加とoracle

単一の `diagnostic_redaction_evidence` でjournalとexportを兼用せず、少なくとも次の二つへ分ける。具体的な保存期間・ACL・許可fieldは未決である。

```text
journal_redaction_evidence = {
  source_release_id,
  release_manifest_id,
  artifact_sha256,
  operation_id,
  journal_epoch,
  channel,
  journal_allowed_field_set_id,
  raw_discarded_at,
  structured_write_only_result,
  storage_acl_digest,
  retention_policy_id,
  retention_deadline_or_policy_ref,
  deletion_observed_at,
  deletion_blocked_reason,
  raw_present,
  forbidden_field_occurrence_count,
  result,
  failure_class
}

support_export_evidence = {
  source_release_id,
  release_manifest_id,
  artifact_sha256,
  operation_id,
  export_generation,
  support_allowed_field_set_id,
  export_actor_authorization_ref,
  export_acl_digest,
  retention_policy_id,
  retention_deadline_or_policy_ref,
  deletion_observed_at,
  secret_sentinel_occurrence_count,
  raw_reconstruction_result,
  result,
  failure_class
}
```

独立oracleはchannel inventoryを全列挙し、structural allowlist、secret/path/argv/nonce scan、不可逆digest、journal/export ACL、
期限・削除時刻、raw discard、同一operationのgeneration/action countを別々に再計算する。対象は `WIN-E-016`、`WIN-I-014..015`、
`WIN-H-008..012`、`WIN-M-015..016`、`WIN-L-015`、`GLOBAL:AUD-011`、`GLOBAL:AUD-020`。既存 overlap は `RC-118`、
`RC-023/090`、`RC-102..106`であり、これはjournal/export分離とretention oracleの追加である。

### SC-06追加契約: per-object token/ACL/reparse TOCTOU evidence

#### 状態・入力・失敗・再入

path全体のaggregate scanではなく、mutation対象各objectをhandle/file identityで閉じる。

```text
InstallContextUnresolved
  -> TokenValidated
  -> ScopeValidated
  -> ObjectHandleSnapshotValidated
  -> ACLAndReparseValidated
  -> SameHandleMutationAuthorized
  -> Reconciled

TokenMismatch / ForeignOwner / ACLWriteFailed / ReparseDetected / ScopeUnknown
  -> MutationRejected -> LastGoodRetained
```

入力は Setup/client/uninstaller/server bootstrap の token/SID/integrity/elevation、各 path component の handle/file ID/volume、
owner SID/operation owner、reparse tag/target、journal epoch、foreign process identity、mutation kindである。scan後に junction/
reparse/ownerが変化した、ACL設定不能、unexpected elevation、foreign owner、identity取得不能の場合は write/delete/rename/commit/
cleanupを全て0にし、旧版・settings/history・foreign objectを保持する。
検証後にpath文字列を再解決してmutationせず、検証した同一handleまたはそのhandleに相対なOS操作だけを使い、commit後にも
対象file identityとownerを再取得する。same-handle操作を提供できない対象はauthorityで許可された安全方式が決まるまでmutation 0とする。

同一 `operation_id + journal_epoch + object_generation + owner_identity` の mutation は一度だけ許可し、PID/HWND/ownerの再利用や
late callbackはno-opとする。identityは process PID/start/image、token SID pseudonym、journal owner、object volume/file ID、
reparse generation、path scope、artifact/source lineageを結合する。same-UID trustを採るかACLをproduct boundaryとするかは
authority fieldとして記録し、Securityのtrust assumptionを黙って上書きしない。

#### evidence schema追加とoracle

既存 `windows_privilege_evidence` のaggregate digestに加え、per-object evidenceを必須化する。具体的なSID pseudonym/ACL policyは未決である。

```text
privilege_object_evidence = {
  object_role,
  parent_handle_identity,
  volume_or_device_id,
  file_id_or_inode,
  owner_identity_ref,
  pre_mutation_acl_digest,
  post_validation_acl_digest,
  pre_mutation_reparse_result,
  post_validation_reparse_result,
  handle_generation,
  same_handle_mutation_result,
  post_mutation_file_identity,
  mutation_kind,
  mutation_count,
  foreign_owner_count,
  replacement_detected,
  result
}

windows_privilege_atomic = {
  source_release_id,
  release_manifest_id,
  artifact_sha256,
  operation_id,
  journal_epoch,
  trust_boundary_policy_id,
  process_token_generation,
  objects: [privilege_object_evidence],
  total_mutation_count,
  rejected_mutation_count
}
```

独立oracleは各objectのhandle/file ID/ACL/reparseをmutation直前に再取得し、race fixture、foreign owner、unexpected elevation、
per-object mutation count、旧版保持をOS/file/process traceから再計算する。対象は `WIN-H-003`, `WIN-H-006..012`, `WIN-L-015`,
`GLOBAL:AUD-011`, `GLOBAL:AUD-020`。既存 overlap は `RC-119`, `RC-091..106`であり、これはTOCTOUを検証可能にするobject-level
evidenceの追加である。

### SC-07追加契約: support matrix rule、per-axis result、architecture authority

#### 状態・入力・失敗・再入

support matrixは全軸を観測したprobe generationを伴う三値状態として扱う。

```text
HostObserved
  -> CapabilityProbe
  -> MatrixClassified(Supported | Unsupported | Unknown)
  -> SupportedCellAuthorized
  -> InstallerOperationAuthorized
  -> PhysicalEvidenceCaptured

MatrixClassified(Unsupported | Unknown)
  -> MutationRejected -> ReprobeOrSafeRecoveryPending
```

入力は edition、build/servicing branch/support end、architecture/RID、locale、display/DPI/text scale/monitor topology、WSL、
OpenSSH、Apps/Start Menu、network mode、offline trust/time/license capability、support rule versionである。いずれかの軸が欠落、
placeholder、判定規則不明、reprobe不能なら `Unknown` とし、install/update/rollback/uninstall/Apps repair/WSL/remote/service
mutationを0にする。未対応hostで安全なuninstallが必要な場合は SC-04 の signed recovery decisionへjoinし、直接削除を行わない。

同一 `host_evidence_id + probe_generation + matrix_rule_version + artifact_sha256` の判定は一度だけ使い、re-entryではedition/build/
capability/support end/artifactを再取得する。identityはhost evidence、rule version、source/artifact release、operation generation、
architecture authorityを結合する。H001の `win-x64` とL015の architecture placeholderを同じ許可集合として扱わない。

#### evidence schema追加とoracle

既存 `windows_support_matrix_evidence` に、aggregate `matrix_result` だけではなく次を追加する。support値・rule version・architecture
policyはauthorityで決める。

```text
support_matrix_atomic = {
  source_release_id,
  release_manifest_id,
  artifact_sha256,
  host_evidence_id,
  probe_generation,
  matrix_rule_id,
  matrix_rule_version,
  axis_results: {
    windows_edition, windows_build, architecture, locale, display_capability,
    wsl_capability, openssh_capability, apps_start_menu_capability, network_mode,
    support_window
  },
  unknown_reason,
  architecture_authority_ref,
  support_end_action,
  operation_id,
  mutation_counts_by_operation,
  matrix_result
}
```

独立oracleは各axisのraw observation、rule version、architecture/RID relation、Unknown reason、support-end policy、operation別 mutation
count、same-host re-entry generationを再計算する。placeholderやclean VMだけの代替は Supported evidence としない。対象は `WIN-H-001`,
`WIN-H-003..004`, `WIN-H-007..009`, `WIN-L-008`, `WIN-L-015`, `WIN-M-027`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`。既存 overlap は
`RC-120`, `RC-024/032/043/050`, `RC-113`であり、これはmatrix rule/per-axis/architecture oracleの追加である。

### SC横断追加契約: 共通lineageと正式なreject/last-good/re-entry辺

#### 共通 evidence lineage

SC-02/04/05/06の現行schemaは共通gateが要求する source/artifact lineage fieldを全て持たないため、次の tuple を7 schemaへ必須joinする。

```text
release_evidence_lineage = {
  source_release_id,
  release_manifest_id,
  artifact_sha256,
  installer_artifact_sha256,
  payload_artifact_sha256,
  installed_artifact_sha256,
  operation_id,
  attempt_generation,
  host_evidence_id,
  schema_version,
  captured_at_utc,
  independent_reviewer,
  reviewer_role,
  reviewer_independence_result
}
```

SC-02は `installer_artifact_sha256` を、SC-04は source/release/installer/artifact SHA を、SC-05は source/release manifest ID を、
SC-06は source/release/artifact SHA と `journal_epoch` をこの共通 lineageへjoinする。欠落、別artifact、旧generation、異なるhost evidence、
不一致のrelease manifestは `LineageUnbound` として evidence reject、公開/cleanup成功/製品判定へ進めない。`WIN-L-004` の artifact別SHAと
共通 release manifest relationを各SCへ再計算可能にする。

#### 正式なDAGと再入境界

happy pathだけでなく、各SCの拒否・保持・回復を次の共通DAGへ結合する。

```text
SourceReleaseObserved
  -> AuthorityFieldsEvaluated
AuthorityFieldsEvaluated --any field open/unknown--> AuthorityHold
  -> LastGoodOrNotInstalledRetained
AuthorityFieldsEvaluated --all required values resolved--> CandidateGenerationAllocated
  -> SC01..SC07Validated
  -> EvidencePending
  -> EvidenceAccepted
  -> ProductPending

SCxRejected / LineageUnbound / EvidenceRejected / MutationRejected
  -> LastGoodOrNotInstalledRetained
  -> SafeRecoveryOrReprobePending

Resume / duplicate re-entry for the same durable operation
  -> CandidateGenerationAllocated(same attempt_generation, idempotent replay)
Explicit Retry after terminal reject or invalidation
  -> CandidateGenerationAllocated(new monotonic attempt_generation)
```

各reject stateは終端を曖昧にせず、保持対象、表示class、safe recovery/reprobe、次回入口、side-effect countを持つ。旧generationの
callback、accepted decision、Apps、diagnostic、host matrixを新attemptへ流用しない。crash resumeに必要な検証済みjournalだけは同一
operation/attemptへ再bindし、全identityを再照合する。`attempt_generation` が同一のduplicate re-entryはidempotent no-opまたはdurable
resumeとし、明示Retryの新generationは全SCのsource/artifact/host/operationを再照合する。hard prerequisite DAGはacyclicに保つ一方、
runtime state transitionのRetry辺はgenerationが厳密増加するbounded edgeとして別型にし、implicit zero-generation cycleを拒否する。
DAG検査は reject edge の欠落、逆向きhard edge、state without terminal/last-good、generation非増加Retry、duplicate side-effectを報告する。

#### 共通oracle

独立validatorは7 schemaの必須lineage tuple、state eventの `state_generation`/`parent_generation`、decision_ref、failure_class、
last-good/retention、re-entry attempt、各mutation/publication/action countを同一 capture から再計算する。旧世代・別artifact・placeholder・
実装者自己判定・schema completenessだけの値は製品証拠に昇格しない。対象は `WIN-H-001..012`, `WIN-L-004`, `WIN-L-015`, `WIN-M-015..016`,
`GLOBAL:AUD-011`, `GLOBAL:AUD-020`, `GLOBAL:AUD-022`。既存 overlap は `RC-114..120`, `RC-102..106`, `RC-024/032/043/047/050`であり、
これはSC横断のlineage/DAG closureである。

## 7件共通の決定・監査ゲート

各SCについて、実装または製品証拠へ進む前に次を決める必要がある。

1. owner、決定ID、適用artifact/operation/profile、positive/negative boundary、failure class、last-good/旧版保持、
   re-entry、独立evaluatorを明記する。
2. `decision_ref`を既存WIN/GLOBAL IDへjoinし、unknown target、self-reference、`GLOBAL:INSTALL-*`など実体未確認IDを追加しない。
3. concrete contractのfixture-only値と製品固定値を分離する。`V_old != V_new`、`<product-key-redacted>`、placeholder SHA、
   generic notice categoryは製品許可・実host証拠へ昇格しない。
4. seven evidence schemasは同一source release、artifact-specific SHA、UTC capture、独立reviewerへ結合し、旧世代・別artifact・
   実装者自己判定を拒否する。
5. いずれかのdecision field、failure/recovery、identity、evidenceが未確定なら、状態は`OPEN_AUTHORITY_CONFLICT`のままであり、
   署名なし、任意downgrade、任意Apps値、raw diagnostic retention、ACL trust、matrix外installのいずれも既定値として実装しない。

## 未確定

SC-01〜SC-07の決定値、実artifact、実Windows host、署名検証raw、package/license inventory、version transition raw、Apps
registry raw、diagnostic channel scan、token/ACL/reparse raw、supported-matrix physical-host raw、独立評価はいずれも未取得である。
本書の作成は要求抽出・製品実装・build・runtime・release・PASSを意味せず、7件すべて`OPEN_AUTHORITY_CONFLICT / PRODUCT_PENDING`
として保持する。

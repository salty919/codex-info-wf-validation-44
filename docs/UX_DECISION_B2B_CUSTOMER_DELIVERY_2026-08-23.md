# UX Decision: B2B customer delivery and enterprise supportability

Decision ID: `UX-20260823-B2B-CUSTOMER-DELIVERY-001`

状態: `REQUIREMENTS_SELECTED / PRODUCT_PENDING / FRESH_AUDIT_REQUIRED`

本書は、B2B納品時に顧客が導入・更新・障害対応・継続利用を判断するための資料、enterprise
deployment境界、supportability境界を原子化する。対象は直前に抽出されたRC-122〜129相当の
8 findingである。現行226件（WIN-A〜M）、旧96件（GLOBAL/legacy crosswalk）と現行RC-001〜163、
直前のrelease supply-chain決定を入力とする。本書が直接所有するのはRC-122〜129、fresh audit
closureとしてRC-150〜159を所有し、RC-130〜149は参照入力として扱う。RC-130〜149の供給網・
server/data evidenceを顧客資料側から再定義しない。

本書の作成は、製品実装、installer変更、build、runtime、Windows実機操作、顧客への出荷、
conformance宣言、RPO/RTOの約束、または独立評価の判定を意味しない。§4〜13は監査時点の候補・
未決入力を履歴として残し、§14が本書所有値の唯一の採用authorityである。§14の採用値は
ユーザー承認済みまたは製品PASSを意味せず、上位要求や他Decisionと衝突した場合は該当RCを
`OPEN_AUTHORITY_CONFLICT`へ戻して製品変更を停止する。

## 1. 正本、目的、非目的

### 1.1 入力とauthority boundary

| source | relevant anchors | role in this decision |
| --- | --- | --- |
| `docs/B2B_RELEASE_ACCEPTANCE.md` | `:5-31` | 出荷停止条件、必須証拠、運用・問い合わせ境界 |
| `docs/CUSTOMER_OPERATIONS_RUNBOOK.md` | `:1-9,39-58,93-131,158-184` | 顧客向けserver/Windows/DB/support操作の現行契約 |
| `README.md` / `README.en.md` | `README.md:6,10-75`; `README.en.md:8-29,65-79` | 開発者向けquick startとWindows概要。顧客導線との衝突監査対象 |
| `SECURITY.md` | `:12-15,27-29,31-40` | データ、trust boundary、endpoint、redactionの安全境界 |
| `docs/COMPLETION_PROTOCOL.md` | `:22-29,45-58` | 証拠、独立評価、release holdの状態真実 |
| `docs/RELEASE_MANIFEST_2026-08-22.md` | `:1-30` | artifact SHA、current hold、出荷判断のsource |
| `docs/UX_DECISION_INSTALLER_LIFECYCLE_2026-08-23.md` | `:15-17,29-36,40-78,80-173` | installer transaction、利用者画面、保持、journal、support artifact |
| `docs/WINDOWS_CLIENT.md` | `:14-17,51-66,199-211` | Windows導入、未確定command、self-contained notice |
| `docs/WINDOWS_CLIENT_REQUIREMENTS.md` | `:230-239,257-267` | installer、UI、accessibilityの現行要求候補 |
| `docs/WINDOWS_UX_SPEC.md` | `:20-41,315-327,370-381` | 利用者役割、入力/accessibility、non-scroll UX gate |
| `docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md` | `RC-061..063, RC-071..074, RC-084..088, RC-091..149, RC-150..159` | 既存conflict overlap除外、監査closureの状態、対象IDの逆引き |
| `docs/UX_DECISION_RELEASE_SUPPLY_CHAIN_2026-08-23.md` | `SC-01..SC-07`, `RC-130..138` | 署名、provenance、version、Apps、diagnostic、token/ACL、matrixと共通evidence lineageの境界との重複除外 |
| `docs/DATA_PROTECTION_POLICY.md` / `docs/REST_API_V1.md` | `DP-REST-001..011`, `RC-139..149` | data state/retentionとREST wire/routeの所有境界を参照し、顧客資料側で再定義しない |

上位要求、既存Decision、具体契約、顧客資料の間で値が一致しない場合、値を推測せず、該当RCを
`OPEN_AUTHORITY_CONFLICT`として保持する。既存RCのtransaction、署名、payload provenance、
version rollback、Apps identity、diagnostic redaction、token/ACL、supported Windows matrixは、
本書では再定義しない。ただし、顧客資料・役割・support processがそれらへ正しくjoinする境界は
別問題として扱う。

### 1.2 目的

- 顧客、operator、administrator、日常user、support担当が、同一source releaseと同一artifactへ
  結合した正しい資料を受け取ること。
- silent/interactive導入、managed deployment、更新、削除、障害時の結果を機械・人の双方が
  決定的に解釈できること。
- privacy、support bundle、accessibility、災害復旧について、未決値を顧客向け保証へ変換しないこと。
- X版のデータ意味論・保持・認証・履歴を変更せず、Windows固有の配布・資料・運用境界だけを
  release artifactへ結合すること。

### 1.3 非目的

- MSI、MSIX、Intune、Configuration Manager、GPO、wingetその他の配布チャネルを採用済みと
  推測しない。採用しない場合は非対応を資料へ明示する。
- telemetry、support portal、purge、追加のGUI、daemon、バックアップ機能、accessibility標準、
  SLAを新機能として勝手に追加しない。
- RPO/RTO、終了コード、対応OS、conformance levelなどの数値を既存文書から逆算しない。
- 現在の実装・過去の画像・旧release manifest・旧`verified`状態を顧客向け証拠に昇格しない。

### 1.4 共通不変条件

1. 顧客資料のrelease identityは、source release、installer/payload/installed artifact SHA、
   version、対象Windows matrix、資料manifestを同一世代へ結合する。X/Linux binary SHAと
   Windows artifact SHAは別値として保持する。
2. `OPEN_AUTHORITY_CONFLICT`、欠落資料、別release混在、stale SHA、未採取の物理host証拠は、
   顧客向け保証・配布可否・conformance宣言・RPO/RTO宣言へ変換しない。
3. 未確定または非対応環境では、operationを開始しない、または既存のlast-good product/data/
   support artifactを保持する。unsupportedをsuccess、unknownをempty、未取得を未障害へ変換しない。
4. すべての再試行・再入・resume・support exportは、同じoperation identityを一度だけ適用する。
   古いgeneration、古いowner、重複document、遅延callbackはno-opとする。
5. raw token、password、private key、session本文、秘密path、raw host/user、argv、stderr、
   remote target、未redactのdiagnosticは顧客資料、support bundle、evidenceへ入れない。
6. installer UX、Help、Legal、Settingsへ資料を表示する場合、既存のnon-scroll、focus、accessible
   name、ページ境界、同一事実の唯一ownerを維持する。外部資料へ分離する場合も、UIからのリンクは
   ownerとversionを重複表示しない。
7. 本書の各状態は資料設計状態であり、実装状態・実機状態・出荷判定を表さない。

### 1.5 dependency DAG

```text
source release + artifact lineage
  -> support/deployment scope decision
    -> operation mode and role guide
      -> release notes / privacy / accessibility / DR statements
        -> support bundle and escalation handoff
          -> same-release B2B evidence package
            -> customer delivery eligibility
```

上流のsource release、artifact、scope、roleが未確定なら、下流資料の成功・対応・保証を確定しない。

## 2. raw-clause ledger（RC-122〜129相当）

| ID | actor / trigger | observable contract | boundary / failure | oracle | status |
| --- | --- | --- | --- | --- | --- |
| RC-122相当 | customer deployment system / install-update-uninstall invocation | supported mode、引数、prompt、終了コード、operation identityを資料とartifactへ結合する | silent/interactive/unsupported/unknown、lock、reboot、rollback、partial、再入を分離し、旧世代を保持 | process argv/exit/stdout-stderr/registry/filesystem/transaction evidence | `requirements_selected / §14.1..2 / PRODUCT_PENDING` |
| RC-123相当 | enterprise administrator、operator、end user | managed deploymentの対応範囲、責務、権限、配布経路、検出・修復を明示する | per-userのみ、managed非対応、user/machine context差、cross-user、policy拒否を推測しない | role/deployment matrix、token/ACL/registry/physical-host evidence | `requirements_selected / §14.3 / PRODUCT_PENDING` |
| RC-124相当 | release owner、customer reader | release notesとknown limitationsが同じrelease/artifact/matrixを参照する | notes欠落、stale version、unsupported cell、upgrade/rollback mismatchは旧notesを保持し公開不可 | customer-document manifest、source/artifact SHA、matrix and limitation join | `requirements_selected / §14.4 / PRODUCT_PENDING` |
| RC-125相当 | operator/admin/user、support | audience別guideが同一release identity、言語、役割、前提、commandへjoinする | READMEとcustomer runbookの矛盾、誤role、旧guide、未確定commandは配布不可 | document manifest、role review、link/version/hash audit | `requirements_selected / §14.3..4 / PRODUCT_PENDING` |
| RC-126相当 | customer privacy reviewer、product/network owner | telemetry有無、outbound endpoint、データ種別、保持、同意、support exportを声明する | 不明endpoint、未redact export、声明と実traffic不一致は送信・export・宣言を保留 | egress inventory、network trace、privacy statement、redaction scan | `requirements_selected / §14.5 / PRODUCT_PENDING` |
| RC-127相当 | end user、support operator、case owner | support bundle/contact/escalationが安全なmanifest、case、owner、severity、retentionを持つ | raw secret、権限不明、stale artifact、重複case、contact未定義はbundleを保持して送信不可 | bundle manifest、canary scan、ACL、case/escalation trace | `requirements_selected / §14.6 / PRODUCT_PENDING` |
| RC-128相当 | accessibility reviewer、customer procurement | accessibility conformanceの標準、scope、matrix、limitations、artifactを明示する | 標準未選択、assistive tech未確認、欠落証拠はconformance claim不可 | UIA/accessibility raw、scale/DPI/locale/state matrix、statement join | `requirements_selected / §14.7 / PRODUCT_PENDING` |
| RC-129相当 | customer continuity owner、restore operator | DR scenarioとRPO/RTOの約束または非提供を明示する | backup/restore failure、objective未定義、stale generationは旧DB/backupを保持し保証不可 | backup/restore timestamp、generation/hash、objective worksheet | `requirements_selected / §14.8 / PRODUCT_PENDING` |

## 3. 公式資料の境界（値を推測しない）

Microsoft資料は、採用するpackage、deployment、accessibilityの方式を決めるための一次資料候補であり、
本製品の対応値・対応チャネル・conformanceを自動確定しない。

| topic | official reference | use / non-use boundary |
| --- | --- | --- |
| Windows package/deployment | [Package and deploy Windows apps overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/) | Store、enterprise、direct distributionなどの選択肢を比較する根拠。MSIX/Intune等の採用を意味しない |
| Windows Installer CLI | [Command-Line Options](https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options) | MSIを選択した場合だけ参照する。現行EXEがMSIであるとは推測しない |
| Windows accessibility | [Develop accessible Windows apps](https://learn.microsoft.com/en-us/windows/apps/develop/accessibility) | programmatic access、keyboard、color/contrastの評価観点。WCAG、VPAT、特定assistive-tech対応の宣言は別決定 |
| Windows accessibility testing | [Accessibility testing](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-testing) | UIA/アクセシビリティ評価の候補。実機evidence未取得を埋めない |

## 4. RC-122相当: silent/interactive deployment と deterministic exit codes

### 4.1 利用者の課題

顧客は、同一setupを人が起動するのか、enterprise deployment systemが無人で起動するのか、
install/update/uninstall/rollbackのどの結果を成功と解釈するのかを決められない。現行資料には
GUI入口とserver commandの列挙はあるが、automationが読む契約がない。

### 4.2 目的と非目的

目的は、対応するinvocation modeだけを明示し、対応しないmodeは非対応と明示し、成功・拒否・
rollback・reboot待ち・lock・部分失敗を同じ整数や画面成功へ丸めないこと。

非目的は、新しいsilent UI、MSI/MSIX wrapper、system service、machine-wide installを追加すること。
silent/interactiveのどちらか一方だけをサポートする判断も許容するが、顧客資料とevidenceの値を
決める必要がある。

### 4.3 代替案と棄却理由

1. **GUI-onlyを暗黙に採用する案**: `docs/UX_DECISION_INSTALLER_LIFECYCLE...:31-35`の利用者導線には
   合うが、managed deploymentがUI成功を機械判定できないため棄却する。
2. **MSI/MSIX/Intuneを必須とする案**: Microsoftの配布選択肢は根拠になるが、既存要求に特定形式の
   採用根拠がなく、未要求機能を必須化するため棄却する。
3. **暫定採用案**: releaseごとに`mode=supported|unsupported`、操作別invocation、終了結果集合、
   stdout/stderr policy、owner、artifact joinをdecisionへ記録する。非対応modeは開始前に説明し、
   supported mode以外のmutationを0にする。

### 4.4 X版との関係とnon-scroll影響

X版のserver、DB、auth、historyの意味論は変更しない。Windows installerは同一source releaseの
server payloadを扱うが、X版のUI起動結果をinstaller終了コードへ流用しない。

installer画面を使うmodeでは既存の同一viewport、step、Cancel、保持対象、non-scroll契約を維持する。
silent modeを採用しても、画面をhiddenにして成功扱いするのではなく、process resultとevidenceを所有する。
外部CLI資料を表示するための新しい長文scroll surfaceは追加しない。

### 4.5 暫定状態機械

```text
ModeUndeclared
  -> ModeDecision(supported / unsupported / unknown)
ModeDecision(unsupported / unknown)
  -> RejectedBeforeMutation
  -> ExitReported(fixed non-success result)
  -> EvidenceJoined
ModeDecision(supported)
  -> InvocationValidated
  -> Preflight(source, artifact, target, owner)
  -> Staging
  -> CommitOrRollback
  -> Reconciled(success / rollback-complete / held)
  -> ExitReported(exactly once)
  -> EvidenceJoined
InvocationValidated / Preflight / Staging / CommitOrRollback
  -> RejectedOrRecovered
  -> ExitReported(fixed non-success result)
  -> EvidenceJoined
```

- `unsupported` は明示的な非対応結果であり、install root、shortcut、HKCU、server、DBを変更しない。
- `unknown`、引数不正、owner不明、artifact mismatch、policy拒否でもprocessはhangや無応答で終えず、
  mutation 0、既存last-good保持を伴う一意な非成功`ExitReported`へ進む。成功・soft successへ丸めない。
- `CommitOrRollback`は初回install/update/uninstallの既存state machine（RC-091〜106）へjoinする。
- exit結果は`operation_id + mode + source_release + artifact_sha + target_generation`へ結合する。
- silent/unattendedを採用する場合はtop-level window、dialog、toast、focus/cursor移動、対話promptを全て0とし、
  hidden GUIの生成をsilent成功として数えない。interactiveだけが明示された利用者操作でUIを所有する。

### 4.6 拒否、保持、idempotence

| condition | required result |
| --- | --- |
| mode/flag未定義、prompt要求、非対応channel | operation開始前に拒否、mutation=0、非対応理由をredactedに記録 |
| source/artifact/version/matrix不一致 | candidateを拒否、旧世代・shortcut・HKCU・settings/historyを保持 |
| lock/foreign owner/reboot boundary | owner判定と既存journalを保持し、別operationを作らない |
| partial install/update | RC-091〜106のrollback/recoveryへ委譲し、partial successを出さない |
| 同じoperationのretry/resume | 同じoperation identityで一度だけcommit。late callbackと二重exit reportはno-op |
| stderr/stdoutに秘密値がある | raw channelをsupport/evidenceへ渡さず、固定failure classだけ保持 |
| child processがtimeout/crash/unknown exit | parent successを出さず、所有generationをinvalidateして一意な非成功exitを1回返す |

### 4.7 evidence schema（未決の候補）

`b2b_deployment_invocation_evidence`:

```text
source_release_id
artifact_sha256
installer_or_setup_identity
operation_id
operation_generation
operation_kind = install|update|rollback|uninstall|server-install|server-update|server-restore|server-uninstall
mode = interactive|silent|unattended|unsupported|unknown
argv_contract_id
argv_token_array_redacted
argv_token_classification
shell_used = false
prompt_count
top_level_window_count
focus_change_count
cursor_move_count
target_scope
owner_identity_pseudonymous
preflight_result
state_trace
exit_code
exit_code_meaning
exit_report_count
child_process_result
stdout_redaction_result
stderr_redaction_result
mutation_counts
old_generation_hash
new_generation_hash
rollback_result
reboot_required_or_not_applicable
captured_at_utc
reviewer_identity
```

候補段階で未決だったexit codeの整数値、命名、reboot/soft-success、supported mode、serverとWindowsの
schema境界は§14.1〜14.2を唯一の採用値とする。実装やコードから別値を逆算しない。

### 4.8 影響IDと既存RC overlap

対象: `WIN-H-001..012`, `WIN-L-015`, `WIN-M-015..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

RC-061/063はserverの顧客lifecycle、RC-091..093はinstall/update/uninstallの利用者UX、
RC-102..106はcrash/reboot/owner identityを扱う。本findingは、それらの状態結果を外部automationが
読むinvocation modeとdeterministic exit contractに限定し、既存RCを再掲しない。

採用authority: mode、flag grammar、exit、reboot、CLI/GUI owner、server operation境界、non-support表示は
§14.1〜14.2。製品証拠は未取得。

## 5. RC-123相当: managed deployment responsibility

### 5.1 利用者の課題

現行契約はper-user・管理者権限不要の導入を記載する一方、UXには管理者・障害対応者・日常監視者が
列挙されている。enterprise administratorが配布、operatorが更新、userが起動、supportが回収する
場合の責任と権限が資料へ分離されていない。

### 5.2 目的と非目的

目的は、B2B customer deliveryでどのdeployment tierをサポートするか、tier外をどう明示するか、
roleごとの責任とmutation scopeを同じreleaseへ結合すること。

非目的はIntune、ConfigMgr、GPO、MSIX、machine-wide service等の新しい配布機能を導入すること。
managed deploymentを非対応とする判断は許容されるが、per-user導入をenterprise対応と誤認させない。

### 5.3 代替案と棄却理由

1. **管理者なら全scopeを操作できるとする案**: `SECURITY.md:14,19`のsame-UID trustやACL境界と
   矛盾し、cross-user/HKLM mutationを推測するため棄却する。
2. **per-userをmanaged deploymentとして扱う案**: H-003の標準user証拠だけではdeployment toolの
   detection、repair、supersedence、exit処理を証明できないため棄却する。
3. **暫定採用案**: `support_tier = per_user_interactive | managed_supported | explicitly_unsupported | unknown`
   をreleaseごとに決め、role matrix、token/ACL、registry/shortcut scope、reboot/offline前提、
   detection/repair ownerを記録する。値未決のtierは顧客対応済みと表示しない。

### 5.4 X版との関係とnon-scroll影響

X版のuser-systemd、loopback API、DB ownerはWindows managed deploymentの権限を付与しない。WSL/Remote
server準備は既存runbookとRC-061/063へjoinし、enterprise policyを理由にX版のデータ保存やauthを変更しない。

role guideやinstaller HelpをUIへ載せる場合も、admin/operator/userの責務一覧を同一viewport内で到達可能にし、
page/root scroll、隠し操作、roleごとの別success copyを増やさない。外部admin guideはUIとversion linkだけを持つ。

### 5.5 暫定状態機械

```text
DeploymentScopeUndeclared
  -> SupportTierDecision
  -> RoleContextObserved
  -> PolicyAndTokenValidated
  -> DeploymentPreflight
  -> ManagedOrPerUserOperation
  -> DetectionAndReconciliation
  -> HandoffReady
```

- `explicitly_unsupported` は、対象tool/contextでmutation=0、非対応理由と代替するsupported導線だけを資料化する。
- `unknown`、foreign user、cross-user target、policy拒否、ACL/reparse不一致はoperationを開始しない。
- `HandoffReady` は実装成功ではなく、operator/admin/user/supportの資料と証拠が同じreleaseへjoinした状態。

### 5.6 拒否、保持、idempotence

managed scopeが未決のままmachine-wide/HKLM/cross-userへ書かない。per-user旧generationがある場合、
managed probe失敗時にそれを削除・昇格・別userへ公開しない。role変更、同じdeployment再送、repair、
uninstall再入は`deployment_scope + user_or_machine_context + artifact_sha + operation_id`で一度だけ処理する。

RC-119のtoken/ACL/reparse failure、RC-120のunsupported matrix failure、installer RC-102〜106の
rollback/recoveryを下流へ委譲し、settings/history/last-good serverは保持する。

### 5.7 evidence schema（未決の候補）

`managed_deployment_responsibility_evidence`:

```text
source_release_id
artifact_sha256
support_tier
deployment_channel_or_explicit_non_support
audience_role = administrator|operator|end_user|support|unknown
user_or_machine_context
token_sid_pseudonymous
integrity_level_and_elevation_result
install_scope
HKCU_HKLM_mutation_counts
start_menu_scope
managed_detection_rule_ref
repair_or_supersedence_rule_ref
offline_network_policy
reboot_policy
cross_user_visibility_result
ACL_reparse_result
responsibility_owner
captured_at_utc
reviewer_identity
```

tool名非依存のper-user silent方式、EXE package、machine-wide非対応、SLA非提供を§14.3で採用した。
Microsoft資料や実装から別の対応範囲を推測しない。

### 5.8 影響IDと既存RC overlap

対象: `WIN-H-003..011`, `WIN-L-015`, `WIN-M-004`, `WIN-M-014..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

RC-119は実装上のtoken/ACL/reparse、RC-120はOS support matrix、RC-091/105はtransaction owner、
RC-061/063はserver bootstrapを扱う。本findingはmanaged deploymentの対応／非対応宣言とrole responsibility
へ限定する。SC-06/SC-07の権限・matrix値も再定義しない。

採用authority: managed tier、tool/channel、role、detection/repair、machine context、reboot/offline、
顧客責任、非対応境界は§14.3。製品証拠は未取得。

## 6. RC-124相当: release notes と known limitations

### 6.1 利用者の課題

release manifestにはartifact SHAと出荷holdがあるが、顧客が読む変更概要、既知制限、対応matrix、
upgrade/rollbackの互換性、前提条件を同じrelease identityで取得できない。テストされていないことと、
既知の非対応であることも区別できない。

### 6.2 目的と非目的

目的は、customer release notesとknown-limitationsをsource release、artifact、version、supported
matrix、support scopeへ結合し、顧客が未決を推測しないこと。

非目的はgit logから変更概要を自動生成すること、RC-120のmatrix値をこの文書で決めること、notesを
product Helpへ必ず埋め込むこと。

### 6.3 代替案と棄却理由

1. **READMEやrelease manifestをrelease notesの代用にする案**: audienceと目的が異なり、manifestには
   versioned known limitation schemaがないため棄却する。
2. **未記載の制限を「未検証」とだけ表示する案**: unsupported、unknown、customer responsibilityを
   区別できず、顧客の誤解を残すため棄却する。
3. **暫定採用案**: release notes、known limitations、support matrix reference、upgrade/rollback
   compatibility、security/privacy noticeを分離したcustomer document artifactへまとめ、未決項目は
   `OPEN`、非対応は`UNSUPPORTED`、実機未取得は`INCONCLUSIVE`としてversion joinする。

### 6.4 X版との関係とnon-scroll影響

notesはX版とのsemantic parity差分を説明するが、X版のquota、history、graph、auth意味論を変更しない。
Windows固有のinstall、Apps、matrix、display、managed scopeだけを差分として記載する。

Help/Legalにnotesやlimitationsを表示する場合、既存のchapter/page/Back/Close/non-scroll契約へ従い、
同じ制限をMain、Status、installerで重複表示しない。外部notesはUIの長文scrollを導入しない。

### 6.5 暫定状態機械

```text
SourceReleaseDeclared
  -> NotesCandidate
  -> ChangeAndLimitationClassified
  -> MatrixAndArtifactJoined
  -> CustomerReview
  -> PublishedForRelease
  -> SupersededOrWithdrawn
```

`artifact SHA/version/matrix/document hash`のいずれかが未確定なら`PublishedForRelease`へ進めない。
新releaseのnotesが未生成でも、旧release notesを新releaseの説明へ流用しない。

### 6.6 拒否、保持、idempotence

- notesとartifactのversion不一致、未定義limitation、source release不一致は公開を拒否し、直前の
  approved document artifactを変更しない。
- matrix外、未実機、offline/WSL/OpenSSH前提不明は、successやsupportedへ丸めず、それぞれの状態を保持する。
- notesの再生成・locale版生成・supersedeは`document_id + audience + locale + source_release + artifact_sha`
  で冪等にし、同一文書の二重公開・旧リンクの新release指示を0にする。

### 6.7 evidence schema（未決の候補）

`customer_release_notes_evidence`:

```text
source_release_id
document_id
document_version
audience
locale
artifact_sha256
installer_sha256
payload_sha256
product_version
source_commit_or_release_ref
change_summary_ref
known_limitations=[id,scope,severity,status,workaround_or_non_support]
supported_matrix_ref
upgrade_rollback_compatibility_ref
prerequisites_ref
security_privacy_notice_ref
source_paths_and_sha256
document_digest
supersedes_document_id
captured_at_utc
reviewer_identity
```

候補段階で未決だったseverity、workaround、support期限、顧客責任、locale、公開channelは§14.4を採用する。

### 6.8 影響IDと既存RC overlap

対象: `WIN-H-001..002`, `WIN-H-005`, `WIN-H-009`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-022`。

RC-024/032/043/047はevidence/source/freeze join、RC-093はcustomer update UX、RC-114〜120と
SC-01〜07はsupply-chain値を扱う。本findingは顧客向けnotesとknown-limit artifactのaudience・内容・
version joinだけを扱う。

採用authority: document owner、version、known-limit taxonomy、公開channel、matrix join、locale、
supersession、support期限、withdrawalは§14.4。製品証拠は未取得。

## 7. RC-125相当: audience別versioned guides

### 7.1 利用者の課題

READMEはrepo/Cargo/Rustupの開発導線、runbookはCargo不要の顧客導線、Windows client docsはinstaller
command未確定という異なる状態を持つ。admin、operator、end user、supportがどの資料をどのreleaseに
使うかを自力で判断しなければならない。

### 7.2 目的と非目的

目的は、audience、role、language、document version、source release、artifact SHA、前提条件、
command、exit code、support contactを一つのdocument manifestへ結合し、古いguideや開発者向け手順を
顧客導線へ混入させないこと。

非目的はREADMEの内容を削除すること、開発者向けquick startを廃止すること、未決installer commandを
発明すること。

### 7.3 代替案と棄却理由

1. **README一冊で全audienceを兼用する案**: 現行のCargo必須と顧客no-Cargo契約が衝突するため棄却する。
2. **URLリンクだけでversion joinを省略する案**: link先の変更、locale、artifact、roleを監査できず、
   stale guideを新releaseに使えるため棄却する。
3. **暫定採用案**: developer、administrator、operator、end user、supportのrole別guideを分け、
   release manifestからdocument manifestを生成する。guideがないroleまたはchannelは非対応と明記する。

### 7.4 X版との関係とnon-scroll影響

developer guideはX版のbuild/run知識を保ち、customer/operator guideはX版server/API lifecycleを
既存runbookへ参照する。Windows guideはX版のデータ意味論・auth/SSH boundaryを改変しない。

Help/Legalのguide link、章、ページは既存non-scrollを守り、同じ導入手順をSetup、Help、READMEへ重複展開しない。
role別外部guideの長文はUI viewportの制約外だが、UIからの参照名・versionは一箇所だけが所有する。

### 7.5 暫定状態機械

```text
GuideSourceDeclared
  -> AudienceAndRoleMapped
  -> LanguageAndVersionJoined
  -> CommandAndPrerequisiteValidated
  -> IndependentRoleReview
  -> DistributedWithRelease
  -> SupersededOrWithdrawn
```

role不明、release不一致、未確定command、古いartifact、リンク循環は配布対象から除外する。

### 7.6 拒否、保持、idempotence

guide versionがartifact/source releaseと一致しない場合、旧release guideを新releaseへ自動昇格しない。
roleが異なるguideをfallbackとして表示せず、該当roleを`UNSUPPORTED_DOCUMENTED`または`OPEN`として残す。
生成・翻訳・配布は`guide_id + role + locale + source_release + artifact_sha`で冪等にし、同一roleへ二重linkを作らない。

### 7.7 evidence schema（未決の候補）

`audience_guide_version_evidence`:

```text
guide_id
guide_version
audience_role
locale
source_release_id
artifact_sha256
installer_payload_sha256
supported_matrix_ref
prerequisite_ref
command_ref
exit_code_ref
privacy_telemetry_ref
support_contact_ref
document_path_and_sha256
supersedes_guide_id
role_reviewer
captured_at_utc
```

候補段階で未決だったrole、locale、channel、command/exit、翻訳review、supersessionは§14.3〜14.4を採用する。

### 7.8 影響IDと既存RC overlap

対象: `WIN-H-003`, `WIN-H-007`, `WIN-H-010..011`, `WIN-L-015`, `WIN-M-004`, `WIN-M-014..016`,
`GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

RC-061/063は顧客server導線、RC-032/043/047は要求・freeze source joinを扱う。本findingは、
README、runbook、Windows client、release notes、support資料のrole/version joinに限定する。

採用authority: role、guide owner、version、locale、release linkage、command/exit、非対応境界、channelは
§14.3〜14.4。製品証拠は未取得。

## 8. RC-126相当: privacy、telemetry、outbound flow

### 8.1 利用者の課題

現行資料はtoken/passwordを保存しないこと、loopback/SSH endpoint、support時に共有できるredacted項目を
示すが、telemetryが存在しないのか、何が外向き通信となるのか、privacy noticeと実trafficをどう結合するかを
顧客へ説明していない。

### 8.2 目的と非目的

目的は、product data、auth、server/API、diagnostic、support exportのoutbound flowとretentionを
列挙し、収集なし・限定収集・非対応のいずれかを明示すること。

非目的はanalytics/telemetryを新設すること、既存loopback/SSH通信をtelemetryと呼ぶこと、未確認trafficを
「安全」と推測すること。

### 8.3 代替案と棄却理由

1. **資料にtelemetry記載がないのでzero telemetryとみなす案**: absence of textはnetwork evidenceではなく、
   support exportや依存componentの外向き通信を説明できないため棄却する。
2. **全diagnosticを顧客supportへ自動送信する案**: 既存secret-safe境界と同意・保持が未定で、privacy scopeを
   拡大するため棄却する。
3. **暫定採用案**: `telemetry = none | declared_limited | unsupported | unknown`をreleaseごとに決め、
   endpoint、data class、identifier、retention、consent/opt-out、support exportをmanifest化する。
   `none`なら外向きtelemetry count=0を証拠化するが、値はauthority決定前に仮定しない。

### 8.4 X版との関係とnon-scroll影響

X版のCodex App Server、local session、SQLite、loopback API、SSH transportは既存のsource/data ownerを維持する。
Windows資料でX版への通常通信をtelemetryと誤分類せず、外向きflowのownerと境界を分ける。

privacy noticeやSettings/Helpのdata flow説明をUIに置く場合は、既存Legal/Helpのpage、focus、non-scroll、
semantic ownerを守る。長いprivacy textをMain/Statusへ重複表示しない。

### 8.5 暫定状態機械

```text
DataFlowUnknown
  -> TelemetryDecision(none / declared_limited / unsupported / unknown)
  -> EndpointAndDataClassInventory
  -> RetentionAndConsentDecision
  -> PrivacyStatementCandidate
  -> NetworkAndStorageEvidence
  -> PublishedOrHeld
```

unknown endpoint、raw response、secret-bearing export、statement/evidence mismatchは`PublishedOrHeld`の
published側へ進めない。既存local product dataとlast-goodを保持し、unknownをemptyへ変換しない。

### 8.6 拒否、保持、idempotence

許可endpoint外への通信、未承認identifier、同意不明のsupport export、redaction不能channelは送信・exportを拒否する。
既存のsettings、server history、DB、last-good snapshotは保持し、privacy判断の再試行で削除しない。
privacy statement生成、network capture、support exportは`source_release + artifact_sha + flow_id + policy_version`で冪等にする。
network traceのraw packet、DNS名、IP、certificate、payloadをそのまま顧客資料へ保存せず、許可された
flow identityとfield-level判定を再計算できる秘密非保持manifestへ変換する。aggregate event countだけを
allowlist適合の証拠にしない。

### 8.7 evidence schema（未決の候補）

`privacy_telemetry_flow_evidence`:

```text
source_release_id
artifact_sha256
flow_id
actor
source_data_class
destination_kind
destination_host_or_loopback_class_redacted
transport_owner
identifier_classes
telemetry_decision
consent_or_opt_out_decision
retention_policy
storage_surfaces
outbound_event_count
per_flow_event_identity_and_count
process_image_and_generation
destination_allowlist_rule_id
field_allowlist_result
raw_secret_occurrence_count
support_export_relation
privacy_statement_version_and_sha256
network_trace_sha256
captured_at_utc
reviewer_identity
```

候補段階で未決だったtelemetry、flow destination class、identifier、retention、consent、support export、
依存runtime境界は§14.5を採用する。

### 8.8 影響IDと既存RC overlap

対象: `WIN-E-016`, `WIN-I-001..016`, `WIN-L-015`, `WIN-M-015..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

RC-118/SC-05はdiagnostic channel redaction/retention、RC-115/SC-02はpayload provenance、RC-023/090は
error UXを扱う。本findingは、顧客privacy statementとoutbound flowの宣言・証跡だけを扱う。

採用authority: telemetry、closed flow集合、data/identifier、retention、consent、support export、ownerは
§14.5。製品network証拠は未取得。

## 9. RC-127相当: support bundle、contact、escalation

### 9.1 利用者の課題

B2B受入は問い合わせ先を必須とするが、runbookは共有してよい項目を列挙するだけで、顧客が安全なbundleを
生成する方法、supportが同一releaseとして受け付けるmanifest、severity/escalation、削除期限を定義していない。

### 9.2 目的と非目的

目的は、support bundleを任意のraw log収集ではなく、redacted、authorized、same-release、case-boundな
evidenceとして扱い、contactとescalation ownerを顧客へ伝えること。

非目的はsupport portal、SLA、24x7体制、追加ログを新設すること。提供しないcontact/escalationは非提供と
明示し、値を推測しない。

### 9.3 代替案と棄却理由

1. **顧客へ任意のlogsを送ってもらう案**: raw path/user/argv/stderr/secret混入を防げず、RC-118/SC-05と矛盾するため棄却する。
2. **runbookの共有可能項目だけをbundle仕様とする案**: file schema、version/SHA、ACL、retention、case identityがなく、
   support側で再現・突合できないため棄却する。
3. **暫定採用案**: support bundleを生成するか非提供とするか、許可field、redaction policy、manifest、contact、severity、
   escalation、retention、case ownerをreleaseごとに決める。未決ならexportを成功表示しない。

### 9.4 X版との関係とnon-scroll影響

X版のserver、DB、backup、SSH、RESTの各証拠は、それぞれのownerとsource releaseへ結合する。Windows support bundleが
Linux historyやsession本文を無断収集することはない。

Support UI、Help、Error CTAにbundle/contactを表示する場合、既存のprimary CTA一個、focus、non-scroll、redacted copyを守る。
詳細manifestを画面へ詰め込まず、利用者が次に行う操作と保持対象だけをviewport内に置く。

### 9.5 暫定状態機械

```text
SupportNeed
  -> BundleScopeSelected
  -> AuthorizationAndRoleValidated
  -> RedactionValidated
  -> BundleManifested
  -> SubmissionAllowedOrUnsupported
  -> CaseOpened
  -> EscalatedOrResolved
  -> RetainedOrDeleted
```

raw secret、権限不明、source/artifact不一致、contact未定義は`SubmissionAllowedOrUnsupported`のallowedへ進めない。
ローカルのproduct state、resumeに必要なinternal journal、直前の検証済みlast-good support artifactは保持する。
秘密混入またはredaction不能なcandidate bundleは隔離後に削除し、再送可能な未送信bundleとして保持せず、
削除結果を証跡化する。削除不能時は送信・成功表示を0にして安全な回復手順だけを示す。

### 9.6 拒否、保持、idempotence

redaction scanが一件でも失敗した場合、bundle write/exportを拒否し、検証済み旧support artifactとproduct stateを保持する。
失敗candidateのraw temporary capture、partial archive、manifestは送信不能領域へ隔離してbounded purgeし、
purge完了前に成功やshare CTAを表示しない。
同じoperationの再試行は`support_operation_id + bundle_digest + artifact_sha`で一度だけcaseへ結合する。
異なるcase ownerによる重複送信、stale releaseの再送、古いcontactへの自動forwardはno-opとする。

### 9.7 evidence schema（未決の候補）

`customer_support_bundle_evidence`:

```text
source_release_id
artifact_sha256
bundle_id
support_operation_id
case_id
bundle_schema_version
requested_by_role
authorized_by_role
included_artifact_refs
included_field_allowlist
forbidden_field_scan
secret_occurrence_count
redaction_policy_id
bundle_files_and_sha256
acl_and_access_result
contact_endpoint_or_explicit_non_support
severity
escalation_rule_ref
retention_and_deletion_deadline
candidate_purge_result_and_utc
submission_result
captured_at_utc
reviewer_identity
```

候補段階で未決だったcontact、case ID、severity、escalation/SLA、retention、authorization、bundle commandは§14.6を採用する。

### 9.8 影響IDと既存RC overlap

対象: `WIN-H-008..012`, `WIN-I-014..015`, `WIN-L-015`, `WIN-M-015..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

RC-118/SC-05は保存channel・redaction・retention、RC-023はfailure CTA、RC-061はrunbook操作を扱う。本findingは、
顧客支援のbundle/contact/escalation processとcase identityだけを扱う。

採用authority: bundle、schema/owner、authorization、contact/case、severity/escalation、retention、
release join、no-case文言は§14.6。製品証拠は未取得。

## 10. RC-128相当: accessibility conformance claim

### 10.1 利用者の課題

現行要求は、UIA name、focus、keyboard、contrast、DPI、text scale、locale、non-scrollなどの行動証拠を求めるが、
顧客が「どの標準のどの範囲を、どのartifactで評価したか」を確認できるconformance statementがない。

### 10.2 目的と非目的

目的は、accessibility claimの標準・版・対象platform・assistive technology・locale・state・scale・known limitation・
evidenceを明示し、評価済みと未評価を分離すること。

非目的はWCAG、VPAT、Microsoft標準、特定screen readerを既存要求から必須化すること。標準を採用しない場合は、
conformance claimを行わず、評価scopeと非提供範囲を明示する。

### 10.3 代替案と棄却理由

1. **UIが見えて操作できればaccessibility対応とする案**: UIA semantics、keyboard、contrast、text scale、localeの
   evidenceを欠き、顧客conformance claimを支えないため棄却する。
2. **Microsoft資料から自動的にWCAG/VPAT適合を宣言する案**: Microsoftの設計観点は根拠だが、製品のclaim scopeを
   決めないため棄却する。
3. **暫定採用案**: `claim = named_standard | internal_scope_only | no_claim | unknown`を決め、surface/state/locale/
   text-scale/DPI/theme/motion/assistive-tech matrixとlimitationsをartifactへjoinする。

### 10.4 X版との関係とnon-scroll影響

X版の機能・データ意味論を変更せず、Windows UIのaccessibility surfaceだけを評価する。X版の受入をWindows conformanceの
代用にしない。

conformance statement、Help、Legal、Settingsの説明は、既存の6 surface、Main内Help、non-scroll、focus restore、
ページ完全性を守る。支援技術向け名前と目視copyの二重ownerを作らない。

### 10.5 暫定状態機械

```text
ConformanceScopeUnknown
  -> StandardAndClaimDecision
  -> MatrixDeclared
  -> AccessibilityEvidenceCaptured
  -> LimitationAndExceptionReviewed
  -> StatementJoinedToArtifact
  -> PublishedOrNoClaim
```

standard未選択、matrix欠落、UIA/keyboard evidence欠落、known limitation未分類はclaimを公開しない。
`no_claim`は失敗ではなく、顧客向けにconformanceを約束しない決定として記録する。

### 10.6 拒否、保持、idempotence

一つのsurface/state/locale/matrix cellのevidence欠落で、全体conformanceを推測しない。直前のstatementがある場合も、
新artifactへ自動流用せず旧版として保持する。再測定、locale翻訳、statement生成は`artifact_sha + claim_scope + cell_id`
で冪等にし、同一cellの複数判定を成功へ合成しない。

### 10.7 evidence schema（未決の候補）

`accessibility_conformance_evidence`:

```text
source_release_id
artifact_sha256
claim_id
claim_type
standard_name_and_version_or_internal_scope
platform_and_windows_matrix_ref
surface_id
state_id
locale
text_scale_percent
dpi_and_theme_motion
assistive_technology_name_and_version
UIA_name_role_value_result
keyboard_focus_route_result
contrast_high_contrast_result
non_scroll_clip_overlap_result
known_limitation_ids
unsupported_scope_ids
raw_evidence_refs
statement_path_and_sha256
captured_at_utc
reviewer_identity
```

候補段階で未決だったclaim scope、AT集合、matrix、例外、wording、document ownerは§14.7を採用する。

### 10.8 影響IDと既存RC overlap

対象: `WIN-G-013..016`, `WIN-M-019..021`, `WIN-M-025..029`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-022`。

RC-035/084/085/087/088/110/113はcontrast、Help、locale、state、focus、text scaleのbehavior oracleを扱う。本findingは、
それらのbehavior evidenceを顧客向けconformance claimへ昇格する標準・scope・statement boundaryだけを扱う。

採用authority: internal-scope claim、参照標準、platform/AT/locale/state matrix、例外、statement owner、
formal no-claim境界は§14.7。製品証拠は未取得。

## 11. RC-129相当: disaster recovery とRPO/RTO

### 11.1 利用者の課題

runbookはbackup世代、quick_check、restore、migration、旧DB保持を示すが、顧客が何時点までのデータ損失を許容し、
どの時間で監視を戻せるかというRPO/RTOの約束、非提供範囲、顧客責任を示さない。

### 11.2 目的と非目的

目的は、host loss、DB corruption、backup failure、migration failure、update failure、service outageなどのscenarioごとに、
RPO/RTOを約束するか、明示的に提供しないかを決め、実測証拠と顧客資料へ結合すること。

非目的は、backup 3世代からRPOを推測すること、restore commandを新設すること、SLAや可用性を無断で約束すること。

### 11.3 代替案と棄却理由

1. **3世代backupをRPO/RTOとして扱う案**: backup age、作成頻度、restore時間、host障害を表さないため棄却する。
2. **restore手順があるのでDR対応と宣言する案**: RC-071..074のmechanicsはobjective値・顧客責任・scenario coverageを
   示さないため棄却する。
3. **暫定採用案**: `dr_claim = measured_rpo_rto | customer_procedure_only | not_offered | unknown`をscenarioごとに決める。
   `not_offered`でもbackup/restore mechanicsと保持条件は記録できるが、時間保証は宣言しない。

### 11.4 X版との関係とnon-scroll影響

X版のSQLite/history/backup semanticsを変更しない。Windows installer/update/uninstallがLinux historyを削除しない既存境界を
維持し、Windows UIからRPO/RTOを推測表示しない。

DR procedureやrestore結果をHelp/Settingsに表示する場合、既存のprimary CTA、current step、保持対象、non-scroll viewportを守る。
詳細なrunbookは外部資料へ置き、Mainのquota/statusと同じ復旧目標を重複表示しない。

### 11.5 暫定状態機械

```text
DRScopeUnknown
  -> ScenarioDeclared
  -> ObjectiveDecision(measured / customer-procedure-only / not-offered / unknown)
  -> BackupGenerationSelected
  -> RestoreMeasured
  -> RPOAndRTOJoined
  -> CustomerStatementPublishedOrHeld
```

backup generation、restore owner、前後SHA/row/fingerprint、scenario、objectiveが揃わない場合、
`CustomerStatementPublished`へ進めない。`not_offered`では未提供を成功や障害なしへ変換しない。

### 11.6 拒否、保持、idempotence

backup破損、quick_check failure、source/backup hash mismatch、restore timeout、migration candidate failureでは、source DB、
既存backup、old memory、last-good UI/APIを保持し、RPO/RTOを推定しない。restore再試行、migration再入、同じgenerationの再測定は
`source_release + scenario + backup_generation + operation_id`で冪等にする。

RPO/RTO測定値は、別scenarioや別artifactへ流用せず、同一releaseのraw timestampとhashへ結合する。
RPOは障害検出時刻だけから推定せず、当該scenarioで最後にdurableと検証されたsource event/cursor時刻と
復元後の最高連続cursor時刻との差を固定式で算出する。RTOはscenario triggerまたは検出のどちらを起点に
するかをauthority値とし、利用者が通常監視を再開できる最初のhealth/readiness時刻までを測る。
source logからのbackfillで回収できる範囲と、host-lossで回収不能な範囲を別結果として記録する。

### 11.7 evidence schema（未決の候補）

`disaster_recovery_objective_evidence`:

```text
source_release_id
artifact_sha256
scenario_id
dr_claim
declared_rpo_value_and_unit_or_not_offered
declared_rto_value_and_unit_or_not_offered
customer_responsibility_ref
backup_generation_id
backup_created_at_utc
last_verified_durable_source_cursor_and_utc
source_db_sha256_before
backup_sha256
restore_start_utc
restore_end_utc
source_db_sha256_after_or_recovered_sha256
row_count_and_fingerprint_before_after
service_recovery_observed_at_utc
rpo_formula_id_and_observed_value
rto_start_semantics_and_observed_value
backfill_recovered_cursor_range
unrecoverable_cursor_range
failure_class
retained_paths_and_hashes_redacted
runbook_version_and_sha256
customer_statement_path_and_sha256
captured_at_utc
reviewer_identity
```

候補段階で未決だったRPO/RTO、scenario、backup schedule非保証、顧客責任、support window、host loss、
statement ownerは§14.8を採用する。

### 11.8 影響IDと既存RC overlap

対象: `WIN-J-006`, `WIN-J-014..015`, `WIN-L-016`, `WIN-M-015..016`, `GLOBAL:DP-008`, `GLOBAL:AUD-011`。

RC-071/072/073/074はbackup rotation、restore、migration、fault retention mechanicsを扱う。本findingは、
顧客向けRPO/RTO objective、非提供宣言、scenario責任、実測timestampのsupportability境界に限定する。

採用authority: scenario、no numeric RPO/RTO、claim、schedule非保証、restore owner、顧客責任、support window、
host loss、statement wordingは§14.8。製品証拠は未取得。

## 12. 共通release/document gate（未実施）

8件は個別に決定し、同じsource releaseへ結合する。加えて、下記13章の10個の横断監査findingを
個別に閉じ、同じsource releaseへ結合する。各schemaはRC-136の共通lineage header
（Decision ID/version、source release、release manifest SHA、対象artifact SHA、document manifest SHA、
operation/state generation、parent generation、UTC、独立reviewerと独立性判定）を必須とし、本文の
個別fieldだけで別世代の証拠をjoinしてはならない。次のいずれかが未決なら顧客向け資料を出荷資料として扱わない。

1. RC-122のdeployment mode、operation identity、exit code、stdout/stderr境界が決まっていない。
2. RC-123のsupport tier、role、managed/non-supported scope、権限・責任者が決まっていない。
3. RC-124/125のrelease notes、known limitations、guide manifest、language/audience/version joinが揃っていない。
4. RC-126のtelemetry/outbound decision、privacy statement、network/storage evidenceが揃っていない。
5. RC-127のbundle schema、redaction、contact、case/escalation、retentionが揃っていない。
6. RC-128のclaim type、standard/scope、matrix、exception、artifact-specific statementが揃っていない。
7. RC-129のscenario、RPO/RTOまたはnot-offered、backup/restore raw timestamp、customer responsibilityが揃っていない。
8. 各evidence schemaが同一source release、artifact SHA、document SHA、UTC capture、独立reviewerへ結合していない。
9. README、runbook、Windows client、release manifest、B2B acceptanceの相互リンクとaudience表示が一致していない。
10. 既存RC-001..163、旧96 crosswalk、SC-01..07の値を新しいcustomer claimへ無断昇格している。
11. 各state machineのreject/held/last-good/retry/resume/cancel辺がRC-137のgeneration/token規則へjoinせず、
    dangling failure stateまたは二重publicationを残している。
12. customer documentのintegrity、配布bundle内path、署名対象、更新・withdrawal時の到達性がSC-01と
    document manifestで一意に結合していない。

各gateの状態は`OPEN_AUTHORITY_CONFLICT`、`PRODUCT_PENDING`、`INCONCLUSIVE`を区別し、未取得証拠を成功・
非対応解除・顧客保証へ変換しない。独立評価、実artifact、physical Windows host、顧客向け資料のraw joinは未取得であり、
本書は要求文書化のみを行う。

## 13. fresh audit atomic closure（RC-150..159）

本章は、直前のfresh読取監査で抽出した10個の意味欠落を、既存の8個の顧客delivery
findingへ横断joinするための要求契約である。ここで付ける名前は監査finding名であり、
ledgerで付与されたRC-150..159のjoin keyとして使用する。本章単独では追加のRC番号やauthority値を
決めず、後続§14が本書所有の採用値を上書きする。全findingの製品状態は`PRODUCT_PENDING`、
入力証拠不足時は`INCONCLUSIVE`を保持し、要求決定を実装・出荷・顧客保証へ昇格しない。

監査finding名とledger番号の対応は次のとおりである。

- `B2B_ACCEPTANCE_ELIGIBILITY_JOIN` → RC-150
- `CUSTOMER_DOCUMENT_LINEAGE_WITHDRAWAL` → RC-151
- `DEPLOYMENT_MODE_OWNER_JOURNAL_JOIN` → RC-152
- `ROLE_PROFILE_SERVICE_LISTENER_JOIN` → RC-153
- `PUBLIC_CLAIM_INVENTORY_HOLD_QUARANTINE` → RC-154
- `TELEMETRY_OPERATIONAL_FLOW_CLASSIFICATION` → RC-155
- `SUPPORT_UNSUPPORTED_NOCASE_TERMINAL` → RC-156
- `ACCESSIBILITY_DIRECT_PRODUCT_CELLS` → RC-157
- `DR_CLAIM_BRANCH_PREDICATES` → RC-158
- `CUSTOMER_DOCUMENT_UI_EXPOSURE_NON_SCROLL` → RC-159

### 13.1 `B2B_ACCEPTANCE_ELIGIBILITY_JOIN`

**対象と状態。** `B2B_RELEASE_ACCEPTANCE.md:5-27`の出荷停止条件と必須納品証拠を、
本書のdependency DAG末端であるcustomer delivery eligibilityへ原子的に結合する。
状態は次の順序とし、`AcceptanceObserved`を製品合格と解釈しない。

```text
CustomerDeliveryCandidate
  -> AcceptanceManifestCollected
  -> AcceptanceEvidenceJoined
  -> CustomerDeliveryEligibilityHeldOrEligible

AcceptanceManifestCollected --missing/open/unverified/inconclusive/hash-mismatch-->
  AcceptanceEligibilityHeld -> LastGoodCustomerPackageRetained
```

**入力・failure・保持。** 入力は同一source release、release manifest、artifact別SHA、
実Windowsのinstall/Start/更新/rollback/uninstall、UI状態、DB quick_check・backup・migration、
Linux daemon、security、OSS、運用問い合わせ先、独立評価の各row evidenceである。どれか一つでも
欠落・別SHA・未検証ならcustomer claim、配布可否、対応表示を生成せず、直前の承認済み
customer packageとproduct/data last-goodを保持する。静的リンク、旧画像、旧判定、READMEの記載を
acceptance evidenceへ変換しない。

**冪等性・identity・oracle。** gate処理は
`source_release_id + release_manifest_sha256 + artifact_sha256 + acceptance_manifest_sha256 +
document_manifest_sha256 + eligibility_generation`で一度だけ評価し、同じgenerationの再評価・遅延
判定はno-opとする。identityには独立reviewerとindependence resultを含める。oracleは受入基準の
各blockerを行単位で列挙し、本書の8 finding、customer document lineage、最終eligibilityのANDを
同一captureから再計算する。B2B acceptanceの実値・実機結果は本契約から推測しない。

既存の`B2B_RELEASE_ACCEPTANCE`、release manifest、`WIN-L-016`の全体gateへjoinするが、既存の
silent、managed、notes、privacy、support、accessibility、DR各findingを再定義しない。対象は
`WIN-H-001..012`, `WIN-E-016`, `WIN-I-014..015`, `WIN-J-006`, `WIN-J-014..015`,
`WIN-L-004`, `WIN-L-015..016`, `WIN-M-015..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`, `GLOBAL:AUD-022`, `GLOBAL:DP-008`。

### 13.2 `CUSTOMER_DOCUMENT_LINEAGE_WITHDRAWAL`

**対象と状態。** notes、guides、privacy、support、accessibility、DRの各candidateを、
供給網のartifact lineageとは別の顧客文書manifestへ結合し、公開後のsupersede/withdrawalまで閉じる。

```text
CustomerDocumentCandidate
  -> DocumentLineageBound
  -> PublicationReview
  -> PublishedForRelease
  -> SupersededOrWithdrawn

DocumentLineageUnbound / PublicationRejected
  -> CustomerDocumentHeld -> PreviousApprovedDocumentRetained
```

**入力・schema・failure。** 全顧客文書schemaは、継承を暗黙にせず次の共通objectを持つ。
fieldの採用値とdocument kindは§14.1、§14.4を参照する。

```text
customer_document_lineage = {
  decision_id,
  decision_version,
  source_release_id,
  release_manifest_sha256,
  installer_artifact_sha256,
  payload_artifact_sha256,
  installed_artifact_sha256,
  document_manifest_sha256,
  document_id,
  document_version,
  document_digest,
  document_path_or_bundle_ref,
  operation_generation,
  parent_generation,
  supersedes_document_id,
  withdrawal_ref_or_reason,
  captured_at_utc,
  independent_reviewer,
  reviewer_independence_result
}
```

別release、別artifact、stale document、署名/path不一致、撤回対象の到達不能は公開0とし、
前世代の承認済み文書と、再公開不能理由を保持する。privacy/support/accessibility/DRにも
`SupersededOrWithdrawn`または明示的なwithdrawal-hold終端を追加し、notes/guidesだけに存在する
撤回辺へ依存しない。

**冪等性・identity・oracle。** 文書生成・locale生成・公開・撤回は
`document_id + audience + locale + source_release_id + artifact_sha256 + document_manifest_sha256 +
document_generation`で一度だけ行い、旧generationのlink・callback・承認はno-opとする。oracleは
6種類の顧客文書すべてに共通objectが存在すること、manifestからartifact/path/digest/署名対象を
再計算できること、公開・supersede・withdrawalの到達性と二重publication=0を検査する。

既存のsupply-chain lineage、release notes、audience guide、privacy、support、accessibility、DR
の各契約へjoinする。署名方式・publisherはsupply-chain authorityを参照し、document ownerと
withdrawal ruleは§14.4を採用する。
対象は`WIN-H-001..012`, `WIN-L-004`, `WIN-L-015`, `WIN-M-015..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`, `GLOBAL:AUD-022`。

### 13.3 `DEPLOYMENT_MODE_OWNER_JOURNAL_JOIN`

**対象と状態。** `mode` enum、operation kind、installer/server owner、journal identityを一つの
invocation contractへ閉じる。§14.1どおり`unattended`はcanonical `silent`の入力alias、
`interactive|silent|unsupported|unknown`は別modeとし、evidenceへ記録する値を一つの
`ModeDecision`へ明示的にmapする。

```text
ModeUndeclared
  -> ModeDecision(mode, operation_kind, platform_owner)
ModeDecision(supported)
  -> InvocationValidated
  -> Preflight
  -> Staging
  -> CommitOrRollback
  -> Reconciled
  -> ExitReported
  -> EvidenceJoined

ModeDecision(unsupported / unknown) / UnmappedMode / UnknownOwner / UnknownOperation / ChildCrash
  -> RejectedBeforeMutation -> JournalOrLastGoodRetained -> ExitReported
```

**入力・failure・保持。** 入力はmode、operation kind、platform、argv contract、source/artifact、
target scope、owner、reboot/lock、child result、stdout/stderr classification、journalである。
server-install/update/restore/uninstallとWindows Setup/Apps operationの対応ownerが§14.1〜14.2と
一致しない場合、または`unattended`を§14.1のcanonical `silent`へmapできない場合はmutation=0、
成功exit=0、旧generation/journalを保持する。
child timeout/crash/unknown exit、reboot境界、foreign ownerでは親successを生成せず、partial successや
soft-successへ丸めない。

**冪等性・identity・oracle。** retry/resume/re-entry/late callbackは
`operation_id + operation_generation + journal_epoch + owner_generation`で一度だけcommitし、
identityは必要最小限のpseudonymous
`installer_pid + pid_start_token + process_image_identity + setup_hwnd + window_instance_generation +
listener_owner_process_identity + supervised_tunnel_generation + profile_service_generation`とする。
raw path/user/tokenは保存しない。oracleはenumの全値がModeDecisionへmapされること、operation kindと
platform ownerの対応、exit report count=1以下、journal epoch/generationの単調性、mutation/hidden
window/promptの結果をprocess/journal/registry/filesystem traceから再計算する。

既存のsilent/interactive deployment、installer lifecycle、crash/reboot/resume、listener owner契約へ
joinし、mode、exit、server command ownerは§14.1〜14.2を採用する。対象は
`WIN-H-001..012`, `WIN-L-015`, `WIN-M-015..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

### 13.4 `ROLE_PROFILE_SERVICE_LISTENER_JOIN`

**対象と状態。** administrator、operator、end_user、supportのcustomer roleを、
`none|wsl|sshConfigAlias` profile、setup step、managed tier、user/machine context、service/listener
ownerへjoinする。roleだけで全scopeを許可したり、per-userをmanaged対応へ昇格したりしない。

```text
RoleProfileScopeUndeclared
  -> RoleProfileMatrixObserved
  -> TokenAndOwnerValidated
  -> ProfileSpecificOperation
  -> DetectionAndReconciliation
  -> HandoffReady

UnknownRole / UnknownProfile / CrossUser / ForeignOwner / PolicyRejected
  -> OperationRejected -> SettingsServerHistoryLastGoodRetained
```

**入力・failure・保持。** 入力はaudience role、support tier、deployment channel、profile、step、
operation kind、token/SID、IL/elevation、scope、WSL distributionまたはSSH alias、service/bootstrap/
listener owner、reboot/offline policyである。role/profile/action/ownerのどれかが不明ならoperation、
repair、uninstall、handoffを開始せず、既存settings、server、history、last-good routeを保持する。
one-session raw recoveryをdurable selectorやcustomer completionへ昇格しない。

**冪等性・identity・oracle。** role変更、同一deployment再送、repair、uninstall re-entryは
`role + profile + deployment_scope + user_or_machine_context + artifact_sha256 + operation_id +
operation_generation`で一度だけ処理する。identityは token SID pseudonym、process PID/start/image、
Setup HWND/window generation、journal epoch、profile-specific service/bootstrap generation、listener
owner/tunnel generationを結合する。oracleはrole×profile×stepのvisible/enabled action、exact argv、
process/service/listener owner、foreign-user mutation=0、handoff documentの同一generationを突合する。

既存managed deployment、profile action semantics、server bootstrap、token/ACL/reparse、installer
owner契約へjoinし、managed tool非依存channelとrole責任は§14.3を採用する。対象は`WIN-H-003..011`,
`WIN-M-004`, `WIN-M-014..016`, `WIN-L-015`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

### 13.5 `PUBLIC_CLAIM_INVENTORY_HOLD_QUARANTINE`

**対象と状態。** README、README.en、Windows client、runbook、release notesにある顧客向け意味単位の
claimをinventory化し、unverifiedな公開claimをquarantineする。文書を削除したり、developer guideを
customer guideへ自動昇格したりしない。

```text
PublicClaimObserved
  -> AudienceAndPurposeClassified
  -> ClaimEvidenceJoined
  -> PublishedClaimOrHeld

Stale / Unverified / AudienceMismatch / ArtifactMismatch
  -> ClaimQuarantined -> PreviousApprovedClaimRetained
```

**入力・failure・保持。** 入力はclaim text、document path、language/audience、source release、
installer/payload/installed SHA、release manifest、supported matrix、actual evidence statusである。
Setup.exe、Start Menu、Apps uninstall、history retentionなど未取得・PRODUCT_PENDINGの主張は、
同じartifact evidenceがない限り顧客保証へ流さず、pending/non-supportの文言またはhold状態を保持する。

**冪等性・identity・oracle。** claimは
`document_id + locale + audience + claim_digest + source_release_id + artifact_sha256 + claim_generation`
で一度だけ分類し、古いlink/late publicationはno-opとする。oracleはREADME/README.en/Windows client/
runbook/release notesを意味単位でscanし、同一claimのowner、audience、release/artifact status、
quarantine理由、重複・反対claim=0を再計算する。

既存のrelease notes、audience guide、installer lifecycle、release manifestへjoinし、公開channelと
claim wordingは§14.4、§14.9を採用する。対象は`WIN-H-001..012`, `WIN-H-003`, `WIN-H-007`, `WIN-H-010..011`,
`WIN-L-015`, `WIN-M-004`, `WIN-M-014..016`, `GLOBAL:AUD-011`, `GLOBAL:AUD-020`, `GLOBAL:AUD-022`。

### 13.6 `TELEMETRY_OPERATIONAL_FLOW_CLASSIFICATION`

**対象と状態。** product telemetryと、認証、固定loopback、WSL/Remote payload transfer、SSH、明示的
support exportなどのoperational flowを同じ「送信なし」へ丸めず、flow purpose/trigger/ownerを
authority決定待ちのfieldとして要求する。

```text
DataFlowUnknown
  -> FlowPurposeAndTriggerObserved
  -> OperationalOrTelemetryClassDecision
  -> EndpointAndDataClassInventory
  -> RetentionConsentAndSecurityPolicyJoined
  -> PrivacyStatementCandidate
  -> NetworkStorageEvidence
  -> PublishedOrHeld

UnknownFlow / SecurityPolicyMismatch / UnknownEndpoint / RedactionFailure
  -> PrivacyStatementHeld -> LastGoodPrivacyArtifactRetained
```

**入力・failure・保持。** 入力はflow ID、purpose、trigger、actor、profile、source data class、
destination class、transport owner、security contract version、allowlist rule、proxy/redirect/cookie policy、
identifier、retention、consent、support-export relation、process image/generationである。`telemetry=none`
と§14.5のclosed operational-flow inventoryを採用し、unknown flowをzero networkと解釈しない。
不明endpoint、未承認identifier、policy不一致、raw response/secret-bearing exportは送信・statement公開を
拒否し、local data、settings、server history、last-good privacy artifactを保持する。

**冪等性・identity・oracle。** privacy statement、network capture、support exportは
`source_release_id + artifact_sha256 + flow_id + flow_generation + policy_version + security_contract_version`
で一度だけ評価し、同じflowの再capture・遅延exportはno-opとする。oracleはSecurityの固定endpoint、
SSH/WSL operation、auth、support export、依存runtime networkを全て列挙し、purpose/trigger別のtelemetry
count、allowlist、proxy/redirect/cookie、秘密 occurrence=0、statementとの差分を再計算する。

既存privacy/telemetry、security endpoint、diagnostic/export契約へjoinし、telemetry、closed flow集合、
retention、consentは§14.5を採用する。対象は`WIN-E-016`, `WIN-I-001..016`, `WIN-L-015`, `WIN-M-015..016`,
`GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

### 13.7 `SUPPORT_UNSUPPORTED_NOCASE_TERMINAL`

**対象と状態。** `SubmissionAllowedOrUnsupported`を一つの曖昧な状態にせず、allowedとunsupportedの
terminal pathを分岐させる。

```text
SupportNeed
  -> BundleScopeSelected
  -> AuthorizationAndRoleValidated
  -> RedactionValidated
  -> BundleManifested
  -> SubmissionDisposition

SubmissionDisposition(allowed)
  -> CaseOpened -> EscalatedOrResolved -> RetainedOrDeleted

SubmissionDisposition(unsupported)
  -> NoCase -> RetainedOrDeleted

RedactionFailed / UnknownChannel / PurgeBlocked
  -> EvidenceRejected -> RawCandidateQuarantined
RawCandidateQuarantined --purge succeeded--> RawCandidatePurged -> RetainedOrDeleted
RawCandidateQuarantined --purge blocked--> DeletionBlockedHeld
```

**入力・failure・保持。** allowed branchだけが`case_id`、contact、submission resultを要求し、
unsupported branchは`unsupported_reason`、`no_case_disposition`、bounded recovery/retentionを持つ。
raw secret、権限不明、stale release、contact不明、redaction不能ではshare CTA・submission・success=0、
内部journalと直前last-good support artifactを保持する。削除不能は削除済みと表示せず、削除待ち状態を
証跡化する。

**冪等性・identity・oracle。** `support_operation_id + bundle_digest + artifact_sha256 +
disposition_generation`につきcase、export、purge、deleteは各一回以下。identityはrequested/authorized
role、bundle manifest、source/release/artifact、caseまたはno-case reason、ACL、retention policyを結合する。
oracleはallowed/unsupported両方をwalkし、unsupportedでcase/submission=0、raw occurrence=0、purge/delete
result、retention terminal、古いcontactへのforward=0を再計算する。

既存support bundle、diagnostic journal/export、error CTA契約へjoinし、public best-effort contact、
private portal非提供、SLA非提供は§14.6を採用する。対象は`WIN-H-008..012`, `WIN-I-014..015`, `WIN-L-015`, `WIN-M-015..016`,
`GLOBAL:AUD-011`, `GLOBAL:AUD-020`。

### 13.8 `ACCESSIBILITY_DIRECT_PRODUCT_CELLS`

**対象と状態。** 顧客向けconformance claimのmatrixを、text scaleとDPIを別軸とするcell集合として
表現する。既存accessibility decisionの必須 fixture
`text_scale_percent=[100,125,150,175,200,225]`、`dpi=[96,144,192]`は値を変更せず引き継ぐ。

```text
ConformanceScopeUnknown
  -> StandardAndClaimDecision
  -> MatrixDeclared
  -> CellApplicabilityAssigned
  -> AccessibilityEvidenceCaptured
  -> LimitationAndExceptionReviewed
  -> StatementJoinedToArtifact
  -> PublishedOrNoClaim

Unassigned / Duplicate / UnknownCell / MissingEvidence
  -> ClaimHeldOrNoClaim -> PreviousStatementRetained
```

**入力・failure・保持。** `accessibility_cell`は少なくとも
`matrix_generation, surface_id, state_id, failure_id, locale, text_scale_percent, dpi, theme, motion,
assistive_technology, applicability, na_reason, evidence_status, statement_generation`を持つ。
§14.7の生成規則に入るcellは全て`applicable`であり、N/A cellを生成しない。cell未割当、重複、scale/DPI欠測、
UIA/keyboard/non-scroll evidence欠落は全体claimへ昇格せず、旧statementまたはno-claimを保持する。
`internal_scope_only`をnamed standard適合へ読み替えない。

**冪等性・identity・oracle。** 再測定・locale生成・statement生成は
`artifact_sha256 + claim_id + claim_scope + matrix_generation + cell_id`で一度だけ行う。oracleは
surface/state/locale/failure/scale/DPI/ATのapplicable/N/A集合を列挙し、missing/extra/duplicate=0、
clip/overlap/root-scroll/content-loss=0、200%と225%の機能保持、statement wordingとclaim typeの一致を
独立再計算する。

既存accessibility scale、keyboard、geometry、UIA、non-scroll契約へjoinし、claim、参照standard、AT集合、
wordingは§14.7を採用する。対象は`WIN-G-013..016`, `WIN-L-007..010`, `WIN-L-015`, `WIN-M-019..021`, `WIN-M-025..029`,
`GLOBAL:AUD-011`, `GLOBAL:AUD-022`。

### 13.9 `DR_CLAIM_BRANCH_PREDICATES`

**対象と状態。** `dr_claim`の各値を測定経路へ無条件に流さず、claimごとの必須・禁止fieldとstatement
経路を分ける。canonical tokenは§14.1の`customer_procedure_only`であり、hyphen表記
`customer-procedure-only`はinvalid tokenとして拒否する。

```text
DRScopeUnknown
  -> ScenarioDeclared
  -> ClaimBranchDecision

ClaimBranchDecision(measured_rpo_rto)
  -> BackupGenerationSelected -> RestoreMeasured -> RPOAndRTOJoined
  -> CustomerStatementPublishedOrHeld

ClaimBranchDecision(customer_procedure_only)
  -> ProcedureEvidenceJoined -> CustomerStatementPublishedOrHeld

ClaimBranchDecision(not_offered)
  -> NonOfferStatementJoined -> CustomerStatementPublishedOrHeld

ClaimBranchDecision(unknown)
  -> DRStatementHeld -> PreviousStatementAndBackupRetained
```

**入力・failure・保持。** measured branchはscenario、backup trigger/schedule、verified durable cursor、
restore start/end、formula、measurement host、customer responsibility等の必要fieldが揃うまで公開しない。
customer-procedure-onlyは時間保証値を生成せず、not-offeredは非提供文言を成功や障害なしへ変換せず、
unknownは旧DB・backup・statementを保持する。numeric declared/observed valueがclaim branchと矛盾、
source/backup hash mismatch、quick_check/migration/restore failureならRPO/RTOを推定しない。

**冪等性・identity・oracle。** scenarioごとに
`source_release_id + artifact_sha256 + scenario_id + backup_generation + operation_id + claim_generation`
で測定・statementを一度だけ結合し、別scenario/artifactへの流用をno-opとする。oracleはclaimごとの
required/forbidden field predicate、backup generation/hash/cursor、restore timestamp、statement wording、
customer responsibility、failure retentionを再計算する。

既存DB backup/restore/migration、runbook、RPO/RTO契約へjoinし、numeric RPO/RTOとschedule/SLA非提供、
owner、support window、host loss責任は§14.8を採用する。対象は`WIN-J-006`, `WIN-J-014..015`, `WIN-L-016`, `WIN-M-015..016`,
`GLOBAL:DP-008`, `GLOBAL:AUD-011`。

### 13.10 `CUSTOMER_DOCUMENT_UI_EXPOSURE_NON_SCROLL`

**対象と状態。** 長文の外部notes/guides/privacy/support/DRを無理にUIへ埋め込まず、UIに表示する場合は
link・summary・page/routeだけを既存Help/Legal/Settingsのnon-scroll/focus/semantic-owner契約へ結合する。

```text
CustomerDocumentCandidate
  -> UIExposureDeclaredOrExternalOnly
  -> UIExposureEvidenceCaptured
  -> PublishedOrHeld

ViewportOverflow / ScrollRequired / Clip / DuplicateOwner / FocusSteal / StaleLink
  -> UIExposureHeld -> PreviousRouteAndDocumentRetained
```

**入力・failure・保持。** `customer_document_ui_exposure`は少なくとも
`document_id, document_version, document_digest, surface_id, page_or_chapter_id, route_id,
semantic_owner_id, viewport_generation, focus_before, focus_after, link_target_generation,
scroll_input_count, clip_result, overlap_result, source_release_id`を持つ。外部資料本文の長さを
unsupported UI scrollで解決しない。主要操作・Back/Close・primary CTA・owner・version linkがviewport外、
scroll/clip/duplicate owner/focus steal、stale linkなら顧客資料公開とroute変更を保留し、現行route/dataを保持する。

**冪等性・identity・oracle。** link/summary exposureは
`document_id + document_digest + surface_id + route_id + viewport_generation + source_release_id`で一度だけ
更新し、旧document callback/linkはno-opとする。identityはsurface、Window/Main generation、caller/focus
generation、semantic ownerを含め、raw path/host/userは含めない。oracleはHelp/Legal/Settings/Setupのfresh UIA
routeを辿り、primary/Back/Close到達、scroll input=0、clip/overlap=0、focus restore、owner/version重複=0を再計算する。

既存non-scroll、Help/keyboard/focus、accessibility、notes/guides、support/error CTA契約へjoinするが、
新しいsurface、HWND、scroll、focus移動、顧客資料の本文内容を発明しない。対象は`WIN-G-013..016`,
`WIN-K-010..015`, `WIN-M-001..030`, `WIN-L-007..010`, `GLOBAL:AUD-020`, `GLOBAL:AUD-022`。

### 13.11 横断状態と未決境界

10個の監査findingは、次の共通境界にのみjoinする。

```text
SourceReleaseAndArtifactObserved
  -> CustomerScopeAndDocumentCandidates
  -> FindingSpecificValidation
  -> CommonLineageAndIndependentReview
  -> CustomerPackageHeldOrPublishedForRelease

AnyOpen / Unknown / Stale / Mismatch / EvidenceRejected
  -> Hold -> LastGoodOrNotInstalledRetained
```

同一operation・同一document・同一cell・同一claimのretry/resumeは、該当findingのidentityを再照合して
一度だけ適用する。別generationのcallback、古いowner、古いdocument、古いstatement、旧acceptanceはno-op。
各findingのauthority値は§14で要求決定する。実artifact、physical host、顧客向けraw join、独立評価は
未取得であり、本章の追加は実装・製品判定・出荷判定を意味しない。

## 14. 採用authority値（要求決定。製品証拠は未取得）

本章は§4〜13で未決としていたcustomer-delivery値を一意化する。これはユーザー承認済み、
実装済み、出荷可能、または実機PASSを意味しない。採用値と実装・証拠が異なる場合は実装へ
合わせて本章を緩和せず、製品をFAIL/HOLDにする。

```text
decision_id = UX-20260823-B2B-CUSTOMER-DELIVERY-001
decision_version = b2b-customer-delivery-v1
authority_status = REQUIREMENTS_SELECTED
product_status = PRODUCT_PENDING
customer_delivery_status = HOLD
```

### 14.1 canonical enumとidentity

| 項目 | 採用値 |
| --- | --- |
| Windows operation | `install`, `update`, `repair`, `rollback`, `uninstall`, `help`, `version` |
| Windows canonical mode | `interactive`, `silent`, `unsupported`, `unknown` |
| 入力alias | `--unattended`は`silent`へ一意mapする。`--silent`との同時指定はinvalid。evidenceはraw token classとcanonical modeを両方持つ |
| Linux operation | `server-install`, `server-update`, `server-restore`, `server-uninstall`。Windows Setupへ渡さず、versioned customer runbookのLinux ownerだけが扱う |
| deployment tier | `per_user_interactive`, `per_user_managed_silent`, `explicitly_unsupported` |
| customer role | `administrator`, `operator`, `end_user`, `support` |
| customer locale | `ja,en,zh-Hans,ko,es,fr,de,pt,it,ru`。製品locale正本と同じ集合を使い、unknown inputは一度だけ`en`へ解決する。input/resolved localeを別fieldで保持し、同一文書内のlocale混在は0 |
| support disposition | `public_best_effort`, `no_case`, `rejected` |
| accessibility claim | `internal_scope_only` |
| DR claim | canonical token `customer_procedure_only` |

全identityはRC-136の共通lineage headerに加え、対象operation/document/cell/scenarioの
generationとparent generationを持つ。別release、別artifact、stale generation、欠落identityは
成功・公開・case・claimを0件とし、旧last-goodまたは未導入状態を保持する。

### 14.2 Windows Setupのinvocationとexit contract

配布実行ファイル名は`CodexInfo.WindowsClient.Setup.exe`とする。文法は次のexact集合だけであり、位置引数、
短縮flag、`/S`、未知flag、重複operationを拒否する。

```text
CodexInfo.WindowsClient.Setup.exe --install [--silent|--unattended]
CodexInfo.WindowsClient.Setup.exe --update [--silent|--unattended]
CodexInfo.WindowsClient.Setup.exe --repair [--silent|--unattended]
CodexInfo.WindowsClient.Setup.exe --rollback [--silent|--unattended]
CodexInfo.WindowsClient.Setup.exe --uninstall [--silent|--unattended]
CodexInfo.WindowsClient.Setup.exe --help [--silent|--unattended]
CodexInfo.WindowsClient.Setup.exe --version [--silent|--unattended]
```

- operation flagはexactly one。flagなしは利用者がStart Menu/Explorerから起動した場合に限り
  `--install`のinteractive入口へmapし、既存導入を検出した場合は同じ画面で`update|repair|uninstall`
  を選ばせる。shell string、PowerShell/cmd wrapper、environment展開は使わない。
- `interactive`だけがSetup HWNDを1つ所有できる。`silent`ではtop-level window、dialog、toast、
  taskbar item、focus change、foreground activation、cursor move、promptがすべて0である。
- per-user自己完結payloadなので、成功operationはOS rebootを要求しない。pending reboot、使用中で
  rollback不能、またはrebootが必要な候補はmutation前に拒否する。`reboot_required`は常に`false`。
- Windows SetupはLinux server operationを実行しない。WSL/SSHのserver導入はcustomer runbookの
  明示操作とし、Windows Setupの成功exitへ混ぜない。

exit codeは次のexact整数だけを返す。`0`だけが成功であり、no-changeも`outcome=no_change`を伴う
`0`とする。非0をwarning successやsoft successへ読み替えない。

| code | canonical result | 必須結果 |
| ---: | --- | --- |
| 0 | `success_or_no_change` | commit済みまたは既に要求状態。journal reconciliationとevidence join済み |
| 2 | `invalid_invocation` | flag/token不正。mutation 0 |
| 3 | `unsupported_mode_scope_or_platform` | mode、scope、OS matrix、reboot前提が非対応。mutation 0 |
| 4 | `security_or_policy_rejected` | token/ACL/reparse/policy/trust拒否。mutation 0 |
| 5 | `busy_or_foreign_owner` | singleton/owner不一致。既存journalとlast-good保持 |
| 6 | `artifact_signature_provenance_or_version_rejected` | candidate隔離、現行版保持 |
| 7 | `staging_io_or_resource_failure` | commit前失敗、candidate cleanup、現行版保持 |
| 8 | `operation_failed_rollback_complete` | commit試行失敗、旧版への完全復帰を検証済み |
| 9 | `recovery_required` | rollback/reconciliation未完了。成功表示0、journal保持 |
| 10 | `owned_child_failed` | 所有child timeout/crash/nonzero/unknown。parent success 0 |
| 11 | `evidence_or_invariant_failure` | stateと観測が矛盾。公開・成功表示0 |

silentのstdoutはUTF-8、LF終端のcanonical JSON object 1行だけ、stderrは0 byteとする。exact keyは
`schemaVersion,operationId,operationGeneration,operation,mode,outcome,failureClass,exitCode,
rebootRequired,sourceReleaseId,artifactSha256`で、未知key、重複key、path、username、SID、host、alias、
token、raw argv、raw exceptionを含めない。`schemaVersion`は`codex-info-setup-result-v1`、
`rebootRequired=false`、`exitCode`はprocess exitと一致する。interactiveは同じresult objectを
owner-only journalへ1回記録し、console出力を必須にしない。

### 14.3 managed deployment、scope、role責任

採用する管理配布は`per_user_managed_silent`であり、任意のdeployment toolが対象利用者本人の
標準user token内で上記exact argvを直接実行し、JSONとexit codeを回収する方式だけをサポートする。
特定toolへの組込み、MSI/MSIX、Intune/ConfigMgr/GPOのnative detection package、SYSTEM/service account、
machine-wide、all-users、cross-user、HKLM、elevated tokenは`explicitly_unsupported`でmutation 0とする。

| role | 許可責任 | 禁止境界 |
| --- | --- | --- |
| administrator | artifact/signature/support-matrix/customer-doc manifestを検証し、対象user contextへ配布方針を設定 | elevated/SYSTEMでproduct mutation、別userデータ閲覧、SSH credential収集 |
| operator | 対象userの標準tokenでsilent operationを開始し、exit JSONとjournal generationを照合 | unknown flag、cross-user repair/uninstall、非対応cellをsuccessへ変換 |
| end_user | interactive install/update/repair/uninstall、初期Setup、profile設定、通常利用 | machine scope変更、他user設定/history操作 |
| support | 利用者が明示生成したredacted bundleと公開issue metadataを確認 | raw DB/log/token/path収集、自動upload、product mutation |

install scopeは`%LOCALAPPDATA%\Programs\Codex Info Monitor`、設定・journal scopeは
`%LOCALAPPDATA%\CodexInfo`、Start Menuはcurrent-userだけ、Apps登録はHKCUだけとする。
検出はSC-04のcanonical Apps identity、installed manifest、exe version、payload SHAのANDで行う。
repair/update/uninstallも同じuser SID pseudonym、operation generation、journal epochへbindする。
offline install/update/repair/uninstallをサポートし、Setup自体はnetwork access 0とする。server接続が
未設定・offlineでもWindows payload commitとは分離し、Setup成功後のclient状態はdisconnectedとして
正直に表示する。インストール/更新/削除にrebootは使わない。

### 14.4 release notes、known limitations、versioned guide

customer document kindは次の6種類に固定する。

```text
release_notes
administrator_guide
operator_guide
end_user_guide
support_and_privacy_guide
accessibility_and_dr_statement
```

各kindを`ja,en,zh-Hans,ko,es,fr,de,pt,it,ru`でrelease package内の
`customer-docs/<locale>/<document-kind>.md`へ同梱し、manifestにSHA-256とUTF-8 byte数を記録する。
unknown inputは文書入口でも一度だけ`en`へ解決し、別localeのsemantic itemを混在させない。
オンラインURLは補助であり正本にしない。Helpの文書入口は同梱版だけを開き、network downloadを
開始しない。`document_version=<product SemVer>+doc.<positive revision>`、release notesは
`<product SemVer>+notes.<positive revision>`とし、product SemVerが違う文書をfallbackしない。

known limitationはexact field
`id,scope,severity,status,affected_matrix_cells,customer_impact,workaround_or_explicit_none,
customer_responsibility,support_disposition,introduced_version,resolved_version_or_null`を持つ。
severityは`blocker|high|medium|low|informational`、statusは
`open|unsupported|mitigated|resolved`である。期限/SLAを約束しないため
`support_deadline=not_offered`を必須にする。`blocker`または安全性・データ完全性に関するopen項目が
1件でもあればcustomer deliveryはHOLDとする。

文書ownerはrelease owner、意味reviewerは該当領域owner、最終reviewerは両者と異なる独立reviewer
とする。同一person/agent identityはindependence FAILである。公開channelはrelease packageと
installed local customer-docsだけ。supersedeは新document manifestへ旧document IDを記録し、
withdrawalは現行package/indexからlinkを除外して署名済みwithdrawal recordを残す。既出packageを
書換えず、旧版はimmutable historical artifactとして保持する。

### 14.5 privacy、telemetry、operational flow

`telemetry_decision=none`を採用する。analytics、usage telemetry、crash report、diagnostic、update check、
support bundleの自動送信は0件であり、opt-in/opt-out UIは作らない。通常機能に必要な通信はtelemetryと
混同せず、次のclosed flow inventoryだけを許可する。

| flow_id | trigger / owner | destination | 許可data | retention / security |
| --- | --- | --- | --- | --- |
| `LOCAL_REST_V1` | client pollまたは明示更新 / Codex Info client | `127.0.0.1:8787`のみ | `/v1/health`、`/v1/status`、`/v1/details`のbounded JSON | healthはreachabilityだけを所有し、readyは同一cycleのstatusが`state=ready AND authenticated=true`で、details rootも検証済みの場合だけ。loopback、cookie/redirect/proxy/outbound DNS 0、client wire bodyの永続保存0 |
| `WSL_LOCAL_EXEC` | saved `wsl` profileの明示server準備、接続、復旧 / client supervisor | `wsl.exe`で選択distribution内のowner限定nonce stagingとversioned bootstrap | signed/hash-verified server bundle、非秘密profile selector、setup operation、service/readiness結果 | direct ArgumentList、shell 0、token/password 0、bundle/operation/owner generationをjournalへ保持。初回準備と通常再接続を別actionにする |
| `REMOTE_SCP_STAGING` | saved `sshConfigAlias` profileの明示server導入・更新 / client supervisor | `scp.exe`からOpenSSH configのliteral Host alias配下のowner限定nonce path | signed/hash-verified server bundleとmanifestだけ | 利用者の明示操作だけ。direct ArgumentList、credentialはOpenSSH owner、Codex Infoによるhost/user/key/token保存0。転送失敗時remote install起動0 |
| `REMOTE_SSH_SERVER_CONTROL` | verified staging後の明示server install/updateまたは既導入server start / client supervisor | `ssh.exe`とOpenSSH configのliteral Host alias | versioned setup operation、target start、health/status結果 | 初回導入・更新・通常startを別action/operation generationへbind。自動経路は`BatchMode=yes`、shell string 0、credential保存0、staging未検証ならmutation 0 |
| `MANAGED_SSH_TUNNEL` | saved `sshConfigAlias` profileの接続/復旧 / `ssh.exe` | OpenSSH configのliteral Host alias | tunnel framing、loopback REST payload。credentialはOpenSSH owner | `BatchMode=yes`、Codex Infoによるtoken/key/password保存0。ProxyJump等はOpenSSH設定ownerとしてstatementへ分類 |
| `CODEX_DELEGATED_AUTH_USAGE` | Linux recorderの明示auth/usage取得 / installed Codex app-server | Codex clientが自身の設定で所有するendpoint | Codex app-server protocol。Codex Infoはcredential bytesを受領・永続化しない | endpoint/credential policyはCodex client owner。unknown delegated flowはusage更新を拒否しlast-good保持 |
| `LOCAL_SUPPORT_EXPORT` | userの明示操作 / Codex Info client | userが選んだlocal fileだけ | §14.6 allowlist済みdiagnostic | network 0、自動upload 0、candidateは7日または64MiBの早い方でpurge対象 |

上表以外のnetwork flow、installer network、update download、remote logging、DNS、HTTP redirect、cookie、
embedded browserは許可しない。product settingsは利用者が削除またはuninstallで明示削除を選ぶまで、
Linux history/backupはDATA policy、installer internal journalは30日かつ合計16MiB（早い方で古い
terminal operationから削除）、support candidate/exportは7日かつ1 bundle 64MiBを上限とする。
retry/resumeに必要なactive journalは期限だけで削除せず、terminal化後にretentionを開始する。
privacy statement ownerはrelease owner、network evidence reviewerは実装者と異なるsecurity reviewerとする。

### 14.6 support、contact、case、retention

product内の自動送信とvendor private portalは提供しない。問い合わせ入口は
`https://github.com/salty919/codex_info_v2/issues`のpublic best-effort issueだけとし、SLA、応答期限、
24x7、private/confidential caseを約束しない。productはbrowserを自動起動せず、Helpにcopy可能なURLと
privacy警告を一か所だけ表示する。

Settingsの利用者操作`診断情報を保存` / `Save diagnostics`とCLI
`CodexInfo.WindowsClient.exe --export-diagnostics <absolute-output-zip>`をサポートする。CLIはstandard user、
shellなしdirect argv、output parentが既存のowner-writable local directory、reparse 0の場合だけ実行する。
bundle allowlistはproduct version、Windows edition/build/architecture、locale、redacted connection profile
kind、UTC event time、canonical failure class、setup exit code、artifact/manifest SHA、service/readiness state、
DB quick_check結果、row count、generation/hash、redacted state traceである。username、SID、home/install/DB
raw path、host/alias、IP/DNS、argv、environment、token、password、key、Codex response/body、session content、
raw exception、raw stdout/stderr、raw DB/logは常に0件とする。

public issueを利用者が明示送信し、positive integer issue numberを確認した場合だけ
`case_id=github-issue:<number>`、`support_disposition=public_best_effort`とする。未送信・contact不達・
confidential requestは`no_case`で、case生成やupload成功を表示しない。severityは
`blocker|high|medium|low|question`を利用者がissue templateで選ぶが、SLAに結び付けない。
escalationは`maintainer_triage_without_time_commitment`だけである。bundleは生成元に7日保持し、
product起動時のbounded purge対象とする。利用者が別pathへcopyしたfileとGitHub側retentionはproduct管理外と
明示する。redaction失敗はcandidateを隔離・purgeし、share CTA、case、successを0にする。

### 14.7 accessibility claim

`claim_type=internal_scope_only`を採用する。顧客向けexact意味は「Windows版は本releaseの宣言matrixで
keyboard、UIA、contrast、高contrast、text scale、DPI、reduced motion、non-scrollを製品内部基準として
評価した。第三者認証、VPAT/ACR、またはWCAG完全適合を表明しない」である。英語版も同じ意味を保持する。

評価基準はWCAG 2.2 Level AAの適用可能なcontrast/keyboard/focus/name-role-value観点とMicrosoft Windows
accessibility guidanceを参照するが、formal conformance claimへ昇格しない。assistive technology集合は
`Windows Narrator (host build付属version)`, `keyboard-only`, `Windows UI Automation tree inspection`。
localeは`ja,en,zh-Hans,ko,es,fr,de,pt,it,ru`とunknown→`en`、text scaleは`100,125,150,175,200,225`、
DPIは`96,144,192`、themeは`normal,high_contrast`、motionは`normal,reduced`、surface/state/failure集合は現行FULL-STATE、
NON-SCROLL、ACCESSIBILITY-SCALE Decisionの直積から生成する。

FULL-STATE Decisionの全surface×全state×全failureをenumerateし、各surfaceが所有するstate/failureだけを
`applicable`として§14.7のlocale/scale/DPI/theme/motion/AT直積を必須cellにする。非owner組合せは
`typed_n_a_reason=surface_does_not_own_state|surface_does_not_own_failure|control_absent_by_design`のいずれかを
1つだけ持ち、raw UIA/imageを要求しない。理由なしN/A、applicableへのN/A、未割当、unknown、missing、extra、
duplicate cellは0件とする。
cellのFAIL/HOLD/INCONCLUSIVEが1件でもあればaccessibility statementを公開せずcustomer deliveryをHOLDに
する。`internal_scope_only`は製品品質gateの緩和ではなく、法的・調達上のformal claim境界だけである。
statement ownerはrelease owner、cell evaluatorと最終statement reviewerは実装者および相互に異なる。

### 14.8 DR claimと顧客責任

全scenarioで`dr_claim=customer_procedure_only`を採用し、数値RPO、数値RTO、SLA、availability、
support windowを提供しない。exact scenario集合は次である。

```text
daemon_process_or_host_reboot
sqlite_busy_full_or_io_failure
sqlite_corruption_or_quick_check_failure
migration_failure
backup_rotation_failure
explicit_restore_failure
wsl_distribution_or_windows_host_loss
```

DBはDATA policyの検証済み3世代、maintenance前backup、明示restore、旧DB/backup保持を実装するが、
周期backupを暗黙に約束しない。顧客責任はWindows/WSL host、WSL distribution、Codex source log、DBと
verified backupを組織のbackup policyで保護すること、daemon/service状態を監視すること、restore前に
runbookを読み対象generationを確認することである。host/distro/source logを同時に失った場合の回収は
提供しない。

customer statementのexact意味は「検証済みbackupからの明示復旧手順を提供するが、復旧可能な時点と
復旧時間の数値保証はしない。実際の回収範囲は残存source log、verified backup、DB generationに依存する」
である。`declared_rpo_value_and_unit_or_not_offered=not_offered`、
`declared_rto_value_and_unit_or_not_offered=not_offered`を必須とし、数値fieldは存在してはならない。
procedure evidenceは各scenarioのbefore/after hash、quick_check、row/fingerprint、cursor、journal、
last-good保持を記録する。別scenarioの成功、3世代という件数、daemon poll間隔から数値保証を推測しない。

### 14.9 customer document UI exposureと公開条件

Main、Graph、Threads、Statusへcustomer document本文・version・support/DR値を追加しない。表示ownerは
次の一か所だけとする。

| customer fact/action | 唯一のUI owner | 表示契約 |
| --- | --- | --- |
| release notes / known limitations | Main内Helpの`このバージョン` page | local bundled documentを開くbutton、version、Back/Close。本文詰込み0 |
| administrator/operator/end-user guide | Main内Helpの`導入と運用` page | role別button、同一locale、stale link 0 |
| privacy / telemetry / support contact | Main内Helpの`プライバシーとサポート` page | telemetry none、public URL、秘密を送らない警告、診断保存へのroute |
| diagnostics export | Settingsの`診断情報を保存` action | user gesture時だけOS file picker。完了/失敗後focusを元controlへ返す |
| accessibility / DR statement | Main内Helpの`対応範囲` page | internal-scope/no numeric guaranteeの短いsummaryとlocal document button |
| license / third-party notices | Legal | 既存Legal pageだけ。customer guideと重複しない |

HelpはMain内で追加HWND 0、Settingsのfile pickerは利用者gestureで開くOS-owned transient dialogだけを許可し、
appがpointerを合成・移動しない。全pageは既存viewportでscroll input 0、clip/overlap 0、primary action、
Back/Close、keyboard/UIA route、focus restoreを満たす。長文本文はlocal external document viewerへ渡し、
app内にroot/page scrollを追加しない。document version/digestが現行releaseと不一致ならbutton公開0、
既存routeとlast-good documentを保持する。

### 14.10 closure mappingと抽出合否

| RC | 本章の採用owner | 要求段階の閉鎖条件 |
| --- | --- | --- |
| RC-122 | §14.1〜14.2 | mode/argv/exit/result/reboot/server-ownerを具体契約へ同値伝播 |
| RC-123 | §14.3 | tier/scope/role/detection/offline/rebootを具体契約へ同値伝播 |
| RC-124 | §14.4 | notes/limitation/version/locale/publication/withdrawalを同値伝播 |
| RC-125 | §14.3〜14.4 | role別guide、channel、version、ownerを同値伝播 |
| RC-126 | §14.5 | telemetry noneとclosed operational-flow inventoryを同値伝播 |
| RC-127 | §14.6 | public best-effort/no-case/bundle/redaction/retentionを同値伝播 |
| RC-128 | §14.7 | internal-scope claimと全matrix軸を同値伝播 |
| RC-129 | §14.8 | customer-procedure-only、scenario、no numeric RPO/RTO、責任を同値伝播 |
| RC-150 | §14.10 | 本章全行とB2B acceptance全blockerのANDをmachine gate化 |
| RC-151 | §14.4 | 6 document kindの共通lineage、supersede、withdrawalをmachine gate化 |
| RC-152 | §14.1〜14.3 | canonical mode×operation×owner×journalを全cell化 |
| RC-153 | §14.3 | role×profile×service/listener ownerを全cell化 |
| RC-154 | §14.4、§14.9 | public claim inventoryとHOLD/quarantineを全customer pathで検査 |
| RC-155 | §14.5 | 7 allowed flowとその他0、per-flow privacy evidenceを検査 |
| RC-156 | §14.6 | allowed public case/no-case/rejected/purge-blockedの全terminalを検査 |
| RC-157 | §14.7 | accessibility direct product cell集合のmissing/extra/duplicate 0 |
| RC-158 | §14.8 | customer-procedure-only branchのrequired/forbidden fieldを検査 |
| RC-159 | §14.9 | document kind×surface×locale×viewportのnon-scroll/focus/ownerを検査 |

要求抽出でのPASSは、上表18行が3具体契約・canonical index・freeze対象・machine gateへ同じ値で伝播し、
修正者と異なるfresh evaluatorがFAIL/INCONCLUSIVE 0とした場合だけである。製品段階はさらに同一source
release、artifact別SHA、physical Windows host、全document、network trace、accessibility cell、DR fault、
independent B2B acceptanceが揃うまで`PRODUCT_PENDING / HOLD`を維持する。

# UX Decision: Windows failure recovery

Decision ID: `UX-20260823-ERROR-001`

状態: `EXTRACTION_DECISION_RECORDED / PRODUCT_PENDING`

## 利用者の課題

接続・認証・設定・データ取得に失敗したとき、技術知識がない利用者でも「何が使えず、次に
どの1操作を行うか」を判断できなければならない。raw例外や複数の同格ボタンを並べるだけでは、
原因の推測と試行錯誤を利用者へ転嫁する。

## 目的

各失敗を一つのcanonical classへ分類し、Cause、Impact、primary recoveryを一意に表示する。
無関係なlast-goodを保持し、raw秘密情報を出さず、利用者が次の1操作を判断できるようにする。

## 検討した案

1. raw backend errorと汎用Retryだけを表示する。
   秘密情報漏えい、長文崩壊、誤ったretry先、技術知識依存が残るため棄却する。
2. すべての失敗をSetupへ戻す。
   API/認証/details/履歴の独立境界を失い、Welcome loopとlast-good消去を再発させるため棄却する。
3. failure classごとにCause、Impact、Primary recoveryを固定し、無関係なlast-goodを保持する。
   状態の意味と次操作を一意にできるため採用する。

## 採用する表示構造

Status ownerは同時に1 failure classだけを表示する。表示はcanonical localization keyで
`Cause`、`Impact`、`Recovery`の3意味へ分け、primary CTAは1個だけにする。SettingsやHelpへの
secondary navigationはメニューに残せるが、状態card内でprimaryと同じ強さにしない。

| failure class | Cause key | Impact key | Primary CTA key | action / 成功後 | failure時保持 |
| --- | --- | --- | --- | --- | --- |
| API_UNREACHABLE | `error.api.unreachable.cause` | `error.api.unreachable.impact` | `action.connection.recheck` | 固定loopback `/v1/health`→status/detailsをone-flightで再確認し、readyならMain | status/details各last-good、設定 |
| SSH_PROFILE_INVALID | `error.ssh.profile.invalid.cause` | `error.ssh.profile.invalid.impact` | `action.connection.settings` | profile/selector grammarを修正するConnection recoveryを開く。未検証のhost原因を表示しない | Main route、非機密設定、last-good |
| SSH_LOCAL_PORT_IN_USE | `error.ssh.local-port-in-use.cause` | `error.ssh.local-port-in-use.impact` | `action.connection.settings` | local listenerの使用中ポートを選び直すConnection recoveryを開く | Main route、非機密設定、last-good |
| SSH_INTERACTION_REQUIRED | `error.ssh.interaction-required.cause` | `error.ssh.interaction-required.impact` | `action.connection.settings` | hidden promptを開かず、interactionを解消するConnection recoveryを開く | Main route、非機密設定、last-good、秘密値0 |
| SSH_PROCESS_START_OR_EXIT | `error.ssh.process-start-or-exit.cause` | `error.ssh.process-start-or-exit.impact` | `action.connection.settings` | supervised processの開始失敗またはhealth前の終了をreapし、Connection recoveryへ移動 | Main route、loopback表示、last-good |
| SSH_HEALTH_UNAVAILABLE | `error.ssh.health-unavailable.cause` | `error.ssh.health-unavailable.impact` | `action.connection.recheck` | 固定loopback `/v1/health`を一度再確認し、不可ならConnection recoveryへ移動 | Main route、last-good、supervised processの状態 |
| AUTH_REQUIRED_OR_EXPIRED | `error.auth.required.cause` | `error.auth.required.impact` | `action.auth.start` | WSL/remote profile正本argvでlogin開始。完了は別の`action.auth.check`後だけ | 非account設定。旧account可視値はclear契約に従う |
| AUTH_LAUNCH_FAILED | `error.auth.launch.cause` | `error.auth.launch.impact` | `action.auth.retry` | 同じ一時profileで認証開始を再試行 | auth_required状態、設定、秘密値0 |
| SETTINGS_CORRUPT | `error.settings.corrupt.cause` | `error.settings.corrupt.impact` | `action.settings.open` | Main disconnectedを維持したままSettingsを開き、valid保存成功後だけmarker更新 | Main到達性、Linux履歴、旧primary設定file |
| SETTINGS_SAVE_FAILED | `error.settings.save.failed.cause` | `error.settings.save.failed.impact` | `action.settings.save.retry` | 同じvalidated candidateの保存を再試行する。`Cancel`はそのSettings instanceの未保存入力だけを破棄して戻る | 旧primary設定bytes、DB/history、connection process。未保存candidateは当該Settings instance内だけ |
| STATUS_INVALID | `error.status.invalid.cause` | `error.status.invalid.impact` | `action.refresh` | statusを手動再取得 | status last-goodとdetails last-good |
| DETAILS_INVALID | `error.details.invalid.cause` | `error.details.invalid.impact` | `action.details.recheck` | detailsを再取得し、shared-core一致後だけdetails root更新 | valid status、details last-good |
| HISTORY_UNAVAILABLE | `error.history.unavailable.cause` | `error.history.unavailable.impact` | `action.history.recheck` | details/historyをone-flightで再取得 | quota/status、history last-good |
| THREADS_UNAVAILABLE | `error.threads.unavailable.cause` | `error.threads.unavailable.impact` | `action.threads.recheck` | details/threadをone-flightで再取得 | status/quota、thread last-good |
| DB_SERVER_ERROR | `error.data.unavailable.cause` | `error.data.unavailable.impact` | `action.connection.recheck` | serverの非破壊復旧後に固定APIを再確認 | WindowsはDBへ触れず、全last-good |
| CLIPBOARD_WRITE_FAILED | `error.clipboard.cause` | `error.clipboard.impact` | `action.copy.retry` | 同じ安全なrendered commandをclipboardへ再送 | command preview、clipboard既存内容 |
| INSTALL_OR_UPDATE_FAILED | `error.install.cause` | `error.install.impact` | `action.install.retry` | staging/rollback検証後に同じinstallerから再試行 | installed旧version、settings/history、shortcut公開状態 |
| UNINSTALL_FAILED | `error.uninstall.cause` | `error.uninstall.impact` | `action.uninstall.retry` | 同じoperation journalとowner leaseを再検証し、未完了uninstallをidempotentに再開 | journal、settings/history、復元可能な旧版、既存の公開状態 |
| CLIENT_SHUTDOWN_TIMEOUT | `error.client.shutdown-timeout.cause` | `error.client.shutdown-timeout.impact` | `action.client.shutdown.retry` | 対象ProcessIdentityを再検証して停止を再試行し、成功するまでfile/shortcut/HKCU mutationを開始しない | 現行client、全file/shortcut/HKCU、settings/history |

`API_UNREACHABLE` の日本語 `error.api.unreachable.impact` は
`現在は更新できていません`、英語は `Unable to update now` とする。Settingsのconnection statusも
同じkeyを参照し、別の `connection.error` 文言やraw HTTP bodyを新設しない。failure class、Cause、
Impact、CTAは別fieldであり、一つの結合文字列へしない。

`SETTINGS_SAVE_FAILED`のCauseは「設定を保存できない」、Impactは「旧設定のまま」であり、primary CTAは
同じvalidated candidateの`action.settings.save.retry`、secondary CTAは`action.cancel`とする。Retry成功時だけ
atomic replaceで新しいvalid bytesを公開し、失敗・Cancelでは旧primary bytes、DB/history、connection processを
変更しない。未保存入力はそのSettings instance内のcandidateだけであり、再起動・別Window・Mainへ漏らさない。

### SSHの観測可能性と分類境界（RC-090）

SSH failureは、製品が直接観測できる入力・OS結果・health結果だけで分類する。次の5 classが
canonicalなSSH接続classであり、同一failure eventへ2 classを割り当てない。

| 観測された事実 | canonical class | 分類しない推測 |
| --- | --- | --- |
| profile/selector grammar、alias token、Include禁止条件の検証失敗 | `SSH_PROFILE_INVALID` | DNS、host key、passwordの原因を推測しない |
| Windows listenerのbindが使用中ポートを返す | `SSH_LOCAL_PORT_IN_USE` | remote側の原因を推測しない |
| 起動前の明示的なinteraction要求を製品の境界で検出 | `SSH_INTERACTION_REQUIRED` | prompt内容からpassword/keyの成功可否を推測しない |
| `CreateProcess`/WSL起動失敗、またはhealth確認前のsupervised process終了 | `SSH_PROCESS_START_OR_EXIT` | generic exitをDNS、host key、passwordへ分類しない |
| processは生存しているが固定loopback `/v1/health`を取得できない、またはhealth結果を受理できない | `SSH_HEALTH_UNAVAILABLE` | health不可からremote DNS/key/passwordを推測しない |

旧名 `SSH_DNS_FAILED`、`SSH_PROCESS_EXITED`、非canonical `SSH_FORWARD_FAILED` は現行の
canonical classではない。genericなOpenSSH exit、未分類stderr、exit codeだけでは
`SSH_PROCESS_START_OR_EXIT`へ留め、DNS/key/password classを新設・表示しない。raw stderrは
永続化せず、UIへ表示しない。

## セキュリティ・レイアウト境界

- HTTP body、stderr、exception、hostname/user入力、password、token、email、path、command lineを
  failure textへ連結しない。固定class/keyだけをloggerへ渡す。
- status/detailsが独立に受理できる場合、片方の失敗で他方の新しいvalid rootをrollbackしない。
- error cardが長くてもprimary CTA、Back/Close、Main主値をviewport外へ押し出さない。
- 10言語すべてでCause/Impact/CTA key集合を一致させ、未翻訳keyは英語へ一意fallbackする。
- background poll失敗はWindowを前面化せず、focus/cursorを変更しない。利用者がCTAを押した場合だけ
  その操作先へfocusを移せる。

## 影響要求

`RC-089`, `RC-090`, `WIN-C-007..009`, `WIN-E-006`, `WIN-E-010..015`, `WIN-F-004..011`,
`WIN-I-014..016`, `WIN-J-008..009`, `WIN-K-001..008`, `WIN-M-014..018`,
`WIN-M-024`, `WIN-M-028`, `WIN-M-030`。

## X版との関係

X版の状態所有者、last-good、認証clear、DB非破壊を維持する。Windows版ではfixed keyとprimary CTAを
追加するが、failure時に値を0へ置換したり、全失敗をSetupへ戻したり、別resourceを消去しない。

## 非スクロール影響

Cause、Impact、primary CTA、Back/Closeはcanonical viewport内に残す。長いraw errorを表示せずfixed
catalog keyを使い、error cardがMain主値や復旧操作をviewport外へ押し出さない。`SETTINGS_SAVE_FAILED`
では未保存candidateを画面内で保持しても、旧primary設定、DB/history、接続processを置換しない。

## 受入oracle（実装後）

各failure classについて、UIA/OCR text key、primary button count、activation後route/request、
resource別before/after root hash、secret sentinel occurrenceを同じartifact SHAへ結合する。

- primary CTA count = 1
- raw/secret/path occurrence = 0
- expected action/route count = 1
- unrelated last-good hash change = 0
- clip/overlap/root-scroll dependency = 0

いずれかを証明できない場合はFAILまたはINCONCLUSIVEであり、実装受入をPASSにしない。

## 証拠計画

19 canonical failure classを1件ずつ注入し、同一release artifact SHA/source freezeでUIA key、button count、route、
request、resource before/after hash、secret/raw occurrence、fresh画像、UTC capture時刻を取得する。
SSHについては、profile grammar invalid、local port in use、explicit interaction required、
process start-or-exit、health unavailableを別fixtureにし、generic exitがDNS/key/passwordへ変換されない
ことをraw分類logで確認する。installerはinstall/update、uninstall、client shutdown timeoutを別fixtureにし、
同じoperation journal/owner identityへ結合して旧公開世代を勝手に変更しないことを確認する。Settingsではsave failure、retry、Cancelを別fixtureにし、旧bytes/hashと
未保存candidateの所有範囲を比較する。実装者と異なる担当が全classを三値判定する。

## 未確定

class/key/CTA/route/保持は確定した。実artifact、raw trace、fresh画像、独立製品判定は未取得であり、
製品状態は`PRODUCT_PENDING`である。X版の状態・値・last-good意味論を変更する判断、OpenSSH詳細を
推測する分類、未登録のfailure class追加は未採用である。

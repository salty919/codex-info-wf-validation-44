# テストギャップ台帳（再発防止）

今回の「Codex Infoへようこそ」が毎回表示された事象を起点に、正常系テストだけでは検出できない状態を要求単位で棚卸しする。未確認は完了扱いにしない。

| Gap ID | 対象 | 見落とし | 必須試験 | 状態 |
| --- | --- | --- | --- | --- |
| TG-SET-01 | 設定JSON | 破損・空・途中書込み時に既定値へ戻り、歓迎画面が再表示される | 破損JSON→再起動→歓迎画面を出さず、未接続状態と設定復旧導線を表示 | 実装・固定SDK試験追加、ホスト再導入後raw未取得 |
| TG-SET-02 | 設定JSON | `ConnectionConfigured` / `SetupCompleted` の保存後再起動を実環境で確認していない | 保存→プロセス終了→再起動を2回、歓迎画面が出ないことを同一SHAで確認 | 未確認 |
| TG-INST-01 | 更新 | クライアント実行中の更新・ファイルロック境界を確認していない | 実行中、終了中、異常終了残留の3状態で更新・rollback・設定保持 | 未確認（現在プロセスがロック中） |
| TG-UX-01 | 初回導線 | Setup/通常画面/設定復旧の全遷移を連続操作していない | 初回、接続成功、認証待ち、接続失敗、再接続、設定変更、再起動 | 未確認 |
| TG-PAR-01 | グラフ | X版との画像同値・欠測・終端・分バケットを独立目視していない | 同一fixtureのX/Windows画像、0/中間/100%、欠測、idle、期間終端 | 未確認 |
| TG-WIN-01 | ウィンドウ | 全画面の実移動・閉じる・マルチモニタを現行SHAで確認していない | Main/Setup/Settings/Graph/Threads/Legalの各状態 | 未確認（物理入力は明示許可なしで実施禁止） |
| TG-DOC-01 | エビデンス | 旧SHA・旧テスト件数・旧PASS記録が現行証拠へ混入した | SHA、テスト件数、画像、インストール先ファイルを同一成果物で再照合 | 継続監査中 |
| TG-DATA-01 | DB/daemon | DB保護試験はあるが、現行Windows導入・UI表示と同一リリースでの横断証拠が不足 | daemon停止/再起動、複数writer、backup/migration、UI再読込を同一SHAで確認 | 未確認 |
| TG-PAR-02 | Windows状態 | initializing/reset-warning/transport failure/stale last-good のfresh画面がない | 各状態を新規PID・現行SHAで表示し、値保持・エラー文・レイアウトを目視 | 未確認 |
| TG-PAR-03 | Graph境界 | `now<start`、`now==reset`、`now>reset`、過去期間、no-history、欠測、系列切替のX/Windows同一fixture証拠がない | 同一入力のX/Windows画像と独立目視 | 未確認 |
| TG-THREAD-01 | Threads | empty/single/multi-root/ページ途中失敗/malformed/duplicate/RPC failure/stale retentionのfresh画面がない | 新規PIDで各fixtureを表示し、部分値を公開しないことを確認 | 未確認 |
| TG-THREAD-02 | Threads live-state設計 | DBの履歴inventory、active path、rollout終端状態、native childを同じ判定契約で組み合わせる表形式試験がなく、停止済みchildが3件混入した | root/child/mixed/empty × active-path有無 × running/terminal/partial/invalid × DB欠落/重複/cycle/dangling × process停止/再起動/複数server/RPC失敗/stale epochをテーブル駆動で全判定し、部分snapshot拒否を確認 | 実装・単体matrix・失敗→完全snapshot復帰・production secure-open境界・EOF全分類テスト追加済み（375 tests）、設計matrix独立監査・同一SHA実画面は未確認 |
| TG-REST-01 | REST | method/404/405/content-type/cache/redirect/cookie/proxy/compression/timeout/切断/oversize/redactionの実プロセス網羅がない | REST実プロセスの異常マトリクスとrawログ | 未確認 |
| TG-DAEMON-01 | daemon | malformed/empty/invalid UTF-8/oversize/途中行/rotation/partial appendとREST再起動の実daemon証拠がない | daemon E2E、REST停止・再起動、last-good再公開 | 未確認 |
| TG-DB-01 | SQLite | BUSY/LOCKED/I/O/disk full/read-only/permission/corrupt、backup中断、prune競合の実障害がない | 障害注入後のrow/file SHA、quick_check、旧世代保持 | 未確認 |
| TG-DB-02 | migration | switch直前クラッシュ、lock競合、atomic switch失敗、再起動後旧DB保持がない | candidate migrationの中断・再起動試験 | 未確認 |
| TG-INST-02 | installer | payload途中失敗、権限失敗、実行中ロック、registry/shortcut削除失敗、中断後復旧がない | install/update/rollback/uninstall異常系と設定・履歴保持 | 未確認 |
| TG-CI-01 | CI/証跡 | native/Windows/installer/画像の由来SHAが完全に連結されず、CI fresh目視もない | clean buildから全成果物manifest、fresh PID画像、独立目視 | 未確認 |

## 完了禁止条件

1. `open`、`partial`、`unverified`、`inconclusive` が1件でも残る場合は納品不可。
2. 実機を要求する項目を、コード存在やユニットテストだけでPASSにしない。
3. 物理入力を伴う試験は、ユーザーの明示許可なしに実行しない。未実行はSKIP/未確認のまま保持する。
4. この台帳を要求追跡台帳へ登録せずに修正を完了扱いにしない。

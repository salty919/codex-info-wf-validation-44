# Windows版 要件抽出ベースライン（実装凍結版）

## 0. 凍結宣言

この文書は、X版とWindows版の比較画像、現行のRust/Slint実装、Windows実装、
既存要求台帳、導入要求、回帰指摘を突合して作る**実装前の要求正本**である。
この文書の抽出状態が `EXTRACTION_COMPLETE` になるまで、Windows版に対する実装、
テスト、ビルド、インストール、画面評価、成果物差し替えを禁止する。

現状態: `EXTRACTION_INCOMPLETE`

抽出予定総数: **226件**

この226件を承認した後に、台帳へ明示されていない要求が1件でも発見された場合は、
抽出承認を失効させる。その時点の再抽出目標はユーザー契約により **226,000件
（226件×1,000）** とし、全件に原子ID・根拠・境界・失敗時保持・UX理由・証拠・独立評価・
合否式を割り当てるまで実装・評価を再開しない。誤記か漏れか判定できない場合も漏れとして扱う。

抽出途中で基準件数が増えた場合は、その時点の承認件数を `N` とする。以後の承認後に
漏れが発見された場合は、再抽出目標を `max(226,000, N × 100)` 件へ拡張する。再抽出を
承認した後の再発でも、その時点の `N` を用いて同じ100倍規則を再適用する。件数を増やした
後に確認を簡略化することはできない。

| 区分 | ID範囲 | 件数 | 抽出根拠 |
| --- | --- | ---: | --- |
| A.製品境界・機能同等性 | WIN-A-001..020 | 20 | `docs/WINDOWS_CLIENT_REQUIREMENTS.md`、`ui/app.slint` |
| B.グラフ・履歴意味論 | WIN-B-001..024 | 24 | `src/main.rs` graph functions、`ui/components.slint`、比較画像 |
| C.メイン画面・状態 | WIN-C-001..020 | 20 | `ui/components.slint`、`ui/app.slint`、MainWindow.axaml |
| D.スレッド・詳細・法的情報 | WIN-D-001..012 | 12 | native thread view、Threads/Legal AXAML |
| E.初期導入・SSH・認証 | WIN-E-001..016 | 16 | Setup AXAML/ViewModel、SSH境界文書 |
| F.設定・回復・再起動 | WIN-F-001..012 | 12 | ClientSettings、SettingsViewModel、完了プロトコル |
| G.多言語・日時・アクセシビリティ | WIN-G-001..016 | 16 | i18n、AutomationProperties、UI規約 |
| H.インストール・更新・削除 | WIN-H-001..012 | 12 | installer/Program.cs、導入要求 |
| I.通信・API・秘密情報 | WIN-I-001..016 | 16 | REST/API契約、SECURITY.md |
| J.履歴・DB・daemon連携 | WIN-J-001..016 | 16 | DATA_PROTECTION_POLICY、REQUIREMENTS_LEDGER |
| K.異常系・境界・同時実行 | WIN-K-001..016 | 16 | TEST_GAP_REGISTER、回帰指摘 |
| L.証拠・独立評価・納品 | WIN-L-001..016 | 16 | AGENTS.md、COMPLETION_PROTOCOL |
| M.UX設計・導線・非スクロール | WIN-M-001..030 | 30 | `docs/WINDOWS_UX_SPEC.md`、DESIGN.md、Windows作法要求 |

## 1. 抽出ルール

各行は、実装者が解釈でまとめられない最小の観測可能な要求とする。「画面が同じ」
は禁止し、値、順序、位置、色、軸、状態、入力、失敗動作、保存境界、証拠形式を分離する。
各行の受入は `PASS` / `FAIL` / `INCONCLUSIVE` の三値とし、証拠不足をPASSへ丸めない。

### 1.1 「十分な要求」とみなすための必須フィールド

226行の各行は、次のフィールドが埋まるまで抽出済みとはみなさない。1つでも空欄、
「適切に」「通常どおり」「画面が同じ」などの解釈語、または実装者の推測が残る行は
`open` とする。

| フィールド | 必須内容 |
| --- | --- |
| `requirement_id` | 変更されない一意ID。分割・統合時は旧IDとの対応を記録する |
| `actor` / `entry` | 利用者、Windowsクライアント、Linux API、SSH、daemon、DBの責務と開始条件 |
| `precondition` | 認証、接続、設定、権限、期間、locale、DPI、モニタ数などの前提 |
| `action` / `observable` | 入力操作と、画面・API・ファイル・プロセスで観測できる結果 |
| `data_contract` | 型、単位、桁、範囲、時刻基準、並び、欠測、重複、丸め、同一期間の扱い |
| `visual_contract` | 座標系、軸、線、色、太さ、塗り方向、余白、文字、アイコン、所有者、重複禁止 |
| `failure_contract` | 失敗分類、last-good保持、部分結果の公開可否、再試行、復旧導線、ログ境界 |
| `persistence_contract` | 保存するもの、保存しないもの、atomic性、再起動、更新、削除、migration、backup |
| `security_contract` | loopback/SSH境界、秘密情報、権限、サイズ上限、入力検証、redaction |
| `performance_contract` | 周期、上限、再入防止、CPU/メモリ増加の禁止条件、停止時の挙動 |
| `evidence_oracle` | 自動試験、実プロセス、fresh画像、SHA、rawログ、独立評価の合否式 |
| `owner_and_dependency` | 実装担当、独立評価担当、所有ファイル、前提要求、後続要求 |

### 1.2 抽出対象の状態・イベント・データ直積

「正常系を1枚確認した」だけでは要求を閉じない。各原子要求は、該当する全セルを
明示的に `applicable` / `not-applicable (根拠付き)` とする。未記載セルは未抽出である。

| 軸 | 必須値 |
| --- | --- |
| 起動・接続状態 | 初回、設定済み、SSH成功、SSH名前解決失敗、API停止、API復帰、認証要求、認証済み、期限切れ、再起動 |
| データ状態 | 空、単一、複数、親子、孤児、重複、同一分、欠測、遅延、無効JSON、巨大入力、境界値、期間切替、過去期間 |
| グラフ境界 | `now < start`、`now == start`、初回観測前、活動区間、アイドル区間、`now == reset`、`now > reset`、0/中間/100、系列ON/OFF |
| ウィンドウ環境 | 700x480、標準サイズ、最小サイズ、高DPI、DPI変更、単一モニタ、異なるDPIの複数モニタ、画面端、最大化/復元、キーボードのみ |
| 入力・ライフサイクル | メニュー遷移、戻る、再入、二重クリック、更新中、子画面二重起動、閉じる、再表示、更新、アンインストール |
| 永続化・競合 | 空/破損/途中書込み設定、DB read-only/full/busy/corrupt、2 client、2 server、2 daemon、backup中断、migration中断 |

直積の全セルを同じテストで実行する必要はないが、各セルに「どの要求のどの証拠で
閉じるか」を割り当てる。割当不能なセルは仕様未確定として実装を禁止する。

### 1.3 不十分な要求の禁止語と分割規則

次の表現を単独の要求文に使用しない。「同じ」「普通」「使いやすい」「今風」「安全」
「正しく」「適切」「十分」「問題ない」「対応する」「対応済み」「動く」は、必ず数値、
状態遷移、入力、保持契約、または合否式へ分解する。「機能同等性」「デザイン品質」も
親要求として残してよいが、子要求IDで全ての観測面を列挙し、親だけでPASSにしてはならない。

要求が複数の責務（例: SSH開始と認証完了、グラフ軸と残量値、インストールと削除）を
含む場合は別IDへ分割する。逆に同一の不変条件を複数画面へ適用する場合は共通契約IDを
持たせ、画面ごとの証拠IDを別途付ける。これにより、1箇所だけ直して関連面を放置する
回帰を検出する。

### 1.4 要求数の扱い

現時点の抽出候補は226件である。これは実装開始を許可する確定数ではなく、既存資料と
会話から得た**下限の原子要求数**である。上記フィールド、直積、未確定事項を埋めた
結果、1行に複数の責務が残る、または新しい境界が見つかる場合は、要求を分割して総数を
増やす。件数を維持するために条件を削ることは禁止する。最終数、増減理由、旧IDとの
対応表を同じ文書へ記録する。

### 1.5 ドメイン拡張の必須条件

共通11列へ収めたことで状態・イベント・失敗境界がテンプレート化される領域は、同じ要求IDのドメイン拡張を持つ。
WIN-D/K/Mのライフサイクル・stale・非スクロール・入力非奪取は `docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md`、
WIN-I/Jの通信・SQLite・daemon・backup・migration・負荷は `docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md` が正本である。
拡張は新しい要求数へ勝手に加算するためではなく、共通行の未抽出を隠さないための必須詳細であり、ID不一致や未作成は抽出FAILとする。
全226行の入力ベクトル、正確な観測、否定条件、型付き依存、証拠oracleは、ID範囲ごとに
`docs/atomic-contracts/WIN_A_D_CONCRETE_CONTRACTS_2026-08-22.md`、
`docs/atomic-contracts/WIN_E_I_CONCRETE_CONTRACTS_2026-08-22.md`、
`docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md` の同一ID行で一意に定義する。
旧 `WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md` とNormative Override V1..V14は履歴資料であり、
現行要求、値の正本、受入oracle、補助11列を上書きしない。

## 2. 要求台帳

### A. 製品境界・機能同等性（20件）

| ID | 要求 | X版根拠 | Windows受入証拠 |
| --- | --- | --- | --- |
| WIN-A-001 | Windows版はX版の監視対象機能を削減しない | native surface inventory | 全画面機能差分表 |
| WIN-A-002 | アカウント状態を表示する | Main native surface | 状態fixture画像 |
| WIN-A-003 | 認証状態を表示する | Main native surface | auth/ready画像 |
| WIN-A-004 | プランを表示する | Main native surface | status fixture |
| WIN-A-005 | 残量率を表示する | RemainingQuota | 0/中間/100画像 |
| WIN-A-006 | 期間種別（週/月）を保持する | quota period | 週/月fixture |
| WIN-A-007 | リセット時刻を表示する | reset label | timezone画像 |
| WIN-A-008 | カウントダウンをゲージ所有者だけに表示する | DESIGN ownership | 重複棚卸し |
| WIN-A-009 | 7日ゲージを表示する | WeekGauge | 0/中間/100画像 |
| WIN-A-010 | モデル別トークン・費用を表示する | ModelUsage | 数値fixture |
| WIN-A-011 | 実行中スレッド要約を表示する | AccountActivity | thread fixture |
| WIN-A-012 | グラフ入口を提供する | Header/Graph | UI操作証拠 |
| WIN-A-013 | スレッド詳細入口を提供する | Threads surface | UI操作証拠 |
| WIN-A-014 | 法的通知入口を提供する | Legal surface | UI操作証拠 |
| WIN-A-015 | 設定入口を提供する | Settings surface | UI操作証拠 |
| WIN-A-016 | 更新操作を提供する | refresh callback | API refresh trace |
| WIN-A-017 | 初回未認証画面を監視画面と混同しない | AuthPanel | auth画像 |
| WIN-A-018 | APIエラー時に最後の有効値を壊さない | SECURITY.md | failure fixture |
| WIN-A-019 | X版にない派生値を勝手に追加しない | DESIGN ownership | 文言棚卸し |
| WIN-A-020 | 既存機能の差分をリリースごとに台帳化する | completion protocol | traceability matrix |

### B. グラフ・履歴意味論（24件）

| ID | 要求 | X版根拠 | Windows受入証拠 |
| --- | --- | --- | --- |
| WIN-B-001 | 現在期間と過去期間を選択できる | GraphSelect | 同一fixture |
| WIN-B-002 | 現在期間の右端は `min(reset_at, now)` | `graph_period_end` | 単体+実画像 |
| WIN-B-003 | 過去期間は確定終了境界を使う | `HistoryPeriod::end` | 境界fixture |
| WIN-B-004 | 左端は期間開始アンカーである | `raw_graph_points` | 左端座標 |
| WIN-B-005 | 初期残量アンカーは100%である | native raw graph | reset fixture |
| WIN-B-006 | 初回観測までの未観測区間は水平である | remaining graph | gap fixture |
| WIN-B-007 | 初回観測は観測時刻で段差表示する | native remaining path | 同一画像 |
| WIN-B-008 | アイドル区間の残量線は水平である | `smooth_remaining_points_with_activity` | idle fixture |
| WIN-B-009 | モデル累積値が進んだ区間だけ残量を変化させる | active segments | data trace |
| WIN-B-010 | 欠測残量は直前有効値を保持する | carry-forward | missing fixture |
| WIN-B-011 | 残量を利用量から終端推測しない | terminal rule | terminal fixture |
| WIN-B-012 | 残量を100%超へ戻さない | monotonic clamp | noisy fixture |
| WIN-B-013 | 同一分bucket内のモデル値は最大値を保持する | minute bucket | duplicate fixture |
| WIN-B-014 | 同一分bucket内の残量値の採用規則を固定する | native bucket matching | raw point trace |
| WIN-B-015 | モデル系列は累積値として描画する | cumulative model paths | cumulative fixture |
| WIN-B-016 | SOL/TERRA/LUNAは個別系列である | GraphToggle | toggle fixture |
| WIN-B-017 | 系列順はRemaining→LUNA→TERRA→SOLである | native legend | image review |
| WIN-B-018 | 系列色はX版の色契約と一致する | theme.slint | pixel/color audit |
| WIN-B-019 | 残量は0–100%の独立意味である | `remaining_graph_y` | axis audit |
| WIN-B-020 | モデル費用軸は表示モデルの最大値で決まる | `dollar_max` | toggle scale fixture |
| WIN-B-021 | トークン軸はK/M/B書式を使う | token axis | formatting tests |
| WIN-B-022 | ドル終端値は2桁表示する | dollar labels | image text audit |
| WIN-B-023 | 未使用区間はモデル値から判定する | unused intervals | shading fixture |
| WIN-B-024 | グラフの線、軸、凡例、ラベルが同じデータ世代を参照する | graph paths | same-SHA evidence |

### C. メイン画面・状態（20件）

| ID | 要求 | X版根拠 | Windows受入証拠 |
| --- | --- | --- | --- |
| WIN-C-001 | 初期化中を明示する | startup state | fresh image |
| WIN-C-002 | 未認証状態を明示する | auth state | auth image |
| WIN-C-003 | 正常状態を明示する | status banner | normal image |
| WIN-C-004 | 残量警告を明示する | quota threshold | warning image |
| WIN-C-005 | 残量危険を明示する | quota threshold | danger image |
| WIN-C-006 | リセット間近を明示する | reset warning | boundary fixture |
| WIN-C-007 | APIエラーを明示する | error state | error image |
| WIN-C-008 | エラー時に認証済み値を誤って未取得へ消さない | stale retention | transition trace |
| WIN-C-009 | 初回エラーは未取得として表示する | failure isolation | first-failure fixture |
| WIN-C-010 | 残量ゲージは左から右へ塗る | RemainingQuota | pixel audit |
| WIN-C-011 | 0/中間/100でゲージ方向が変わらない | WeekGauge | three images |
| WIN-C-012 | 0日・0時間を不自然に表示しない | i18n duration | boundary text |
| WIN-C-013 | アカウント項目を重複表示しない | DESIGN table | semantic inventory |
| WIN-C-014 | メイン画面は主値を最初に視認できる | visual hierarchy | independent review |
| WIN-C-015 | 状態色だけに意味を依存しない | accessibility | text/status audit |
| WIN-C-016 | 接続先表示は固定loopback/SSH boundaryだけを示す | SECURITY | text audit |
| WIN-C-017 | 主要カードの左右端を共通グリッドへ揃える | UI checklist | measured image |
| WIN-C-018 | 最小幅でクリップ・横溢れしない | UI checklist | 700x480 image |
| WIN-C-019 | 高DPIで余白と文字サイズを破綻させない | Windows request | 1350x1080 image |
| WIN-C-020 | 更新中のボタン状態と再入防止を表示する | refresh state | interaction trace |

### D. スレッド・詳細・法的情報（12件）

| ID | 要求 | X版根拠 | Windows受入証拠 |
| --- | --- | --- | --- |
| WIN-D-001 | スレッド画面を単一インスタンスで開く | thread window | lifecycle test |
| WIN-D-002 | 空のスレッド状態を表示する | empty state | empty image |
| WIN-D-003 | 単一スレッドを表示する | thread row | fixture |
| WIN-D-004 | 複数スレッドを表示する | thread list | fixture |
| WIN-D-005 | 親→子の深さ優先順を維持する | thread_state | order test |
| WIN-D-006 | orphanを誤って親へ結合しない | orphan rule | orphan fixture |
| WIN-D-007 | role/depth/modelを表示する | thread row | text audit |
| WIN-D-008 | context使用率を表示する | thread row | numeric fixture |
| WIN-D-009 | 累積tokenを表示する | thread row | numeric fixture |
| WIN-D-010 | 経過時間と指示年齢を表示する | thread row | time fixture |
| WIN-D-011 | 法的通知を認証前後で開ける | Legal window | legal image |
| WIN-D-012 | GPL/third-party/font/schema/dependency通知を省略しない | notices | notice inventory |

### E. 初期導入・SSH・認証（16件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-E-001 | 初回起動で導入目的を説明する | SetupWindow | setup image |
| WIN-E-002 | Linux APIとSSH転送の関係を説明する | setup copy | text audit |
| WIN-E-003 | 認証情報を保存しないと明示する | security copy | text audit |
| WIN-E-004 | SSH user入力を提供する | Setup VM | setup fixture |
| WIN-E-005 | SSH host/IP入力を提供する | Setup VM | setup fixture |
| WIN-E-006 | SSH config Host alias選択を提供する | SSH config | config fixture |
| WIN-E-007 | Host aliasから危険な引数を生成しない | safety | negative tests |
| WIN-E-008 | `ssh.exe -N -L`の同等コマンドを表示する | setup guide | command audit |
| WIN-E-009 | コマンドをクリップボードへコピーできる | setup action | interaction trace |
| WIN-E-010 | Windows側からSSH転送を開始できる | setup action | process trace |
| WIN-E-011 | SSHプロセス失敗を汎用文言で表示する | failure contract | error fixture |
| WIN-E-012 | Linux側API到達性を再確認できる | refresh/check | transport trace |
| WIN-E-013 | API確認と認証完了を混同しない | auth boundary | state matrix |
| WIN-E-014 | ブラウザ認証開始を明示操作に限定する | auth command | command trace |
| WIN-E-015 | 接続成功後に毎回Setup画面を出さない | setup marker | two-launch trace |
| WIN-E-016 | SSH host/userを資格情報として保存しない | settings policy | filesystem audit |

### F. 設定・回復・再起動（12件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-F-001 | 設定画面をメニューから開ける | Settings | UI trace |
| WIN-F-002 | 言語を変更できる | localization | setting trace |
| WIN-F-003 | timezone表示設定を変更できる | Settings VM | timezone trace |
| WIN-F-004 | 接続状態を設定画面で確認できる | Settings | state image |
| WIN-F-005 | 認証確認を設定画面から再実行できる | Settings | command trace |
| WIN-F-006 | 法的通知へ到達できる | Settings | UI trace |
| WIN-F-007 | 初期設定へ戻る導線を提供する | Setup action | UI trace |
| WIN-F-008 | 正常設定をatomicに保存する | ClientSettingsStore | file/hash test |
| WIN-F-009 | 空JSONを安全な切断状態にする | settings policy | malformed fixture |
| WIN-F-010 | 途中書き込みJSONを安全に扱う | settings policy | truncated fixture |
| WIN-F-011 | 設定破損でWelcomeを無限表示しない | regression | two-restart trace |
| WIN-F-012 | 再起動後に履歴・表示値を勝手に消さない | persistence | before/after hash |

### G. 多言語・日時・アクセシビリティ（16件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-G-001 | X版の対応言語カタログを持つ | i18n catalog | catalog diff |
| WIN-G-002 | 未知言語は決定的fallbackを使う | i18n | unknown locale |
| WIN-G-003 | メイン画面を翻訳する | UiText | all-view images |
| WIN-G-004 | Setupを翻訳する | UiText | setup locales |
| WIN-G-005 | Settingsを翻訳する | UiText | settings locales |
| WIN-G-006 | Graphを翻訳する | UiText | graph locales |
| WIN-G-007 | Threadsを翻訳する | UiText | threads locales |
| WIN-G-008 | Legalを翻訳する | UiText | legal locales |
| WIN-G-009 | 状態・エラー文を翻訳する | UiText | state matrix |
| WIN-G-010 | 日付をlocale/timezoneに従わせる | datetime helper | timezone images |
| WIN-G-011 | 数値区切りをlocaleに従わせる | numeric formatting | locale tests |
| WIN-G-012 | ドル・tokenの意味をlocale変更で変えない | protocol | semantic fixture |
| WIN-G-013 | 全操作ボタンにAutomationProperties.Nameを付ける | AXAML | static inventory |
| WIN-G-014 | キーボードTab移動を成立させる | accessibility | keyboard smoke |
| WIN-G-015 | フォーカス状態を視認可能にする | design request | focus image |
| WIN-G-016 | 色だけで状態を表現しない | accessibility | status text audit |

### H. インストール・更新・削除（12件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-H-001 | runnable Windows installerを配布する | installer | artifact hash |
| WIN-H-002 | self-contained payloadを含む | installer | zip manifest |
| WIN-H-003 | 管理者権限なしのper-user導入を可能にする | installer | host install |
| WIN-H-004 | Start Menu shortcutを作る | installer | lnk inspection |
| WIN-H-005 | shortcutのtargetを検証する | installer | PowerShell |
| WIN-H-006 | working directoryを検証する | installer | PowerShell |
| WIN-H-007 | HKCU uninstall登録を作る | installer | registry |
| WIN-H-008 | 部分コピー時にshortcutを公開しない | installer rollback | failure fixture |
| WIN-H-009 | 上書き更新をstaging/rollbackで行う | installer | failure fixture |
| WIN-H-010 | アンインストールで機能を除去する | installer | uninstall trace |
| WIN-H-011 | 通常削除で設定/履歴を保持する | installer | hash audit |
| WIN-H-012 | 明示purge以外で履歴を削除しない | installer | purge trace |

### I. 通信・API・秘密情報（16件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-I-001 | 通信先は固定loopback APIだけにする | SECURITY | endpoint scan |
| WIN-I-002 | SSH local tunnel boundaryを維持する | security policy | source audit |
| WIN-I-003 | redirectを許可しない | REST client | negative test |
| WIN-I-004 | cookieを送受信しない | REST client | source/test |
| WIN-I-005 | proxy/decompressionを許可しない | REST client | source/test |
| WIN-I-006 | response sizeを制限する | API contract | oversize fixture |
| WIN-I-007 | JSON schemaを厳格検証する | API contract | malformed fixture |
| WIN-I-008 | unknown keyを拒否する | API contract | schema fixture |
| WIN-I-009 | duplicate keyを拒否する | API contract | schema fixture |
| WIN-I-010 | timestamp domainを検証する | API contract | boundary fixture |
| WIN-I-011 | percent domainを検証する | API contract | boundary fixture |
| WIN-I-012 | dollar/token finite性を検証する | API contract | NaN/overflow fixture |
| WIN-I-013 | model名をSOL/TERRA/LUNAに限定する | API contract | unknown model fixture |
| WIN-I-014 | raw backend errorを表示しない | SECURITY | error redaction |
| WIN-I-015 | token/password/email/pathを表示しない | SECURITY | secret scan |
| WIN-I-016 | invalid snapshotでlast-goodを置換しない | snapshot policy | before/after hash |

### J. 履歴・DB・daemon連携（16件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-J-001 | `/v1/details`をread-onlyで扱う | API contract | request trace |
| WIN-J-002 | history periodのid/start/end/currentを保持する | details contract | fixture |
| WIN-J-003 | history sampleのreset_atを保持する | details contract | fixture |
| WIN-J-004 | minute bucket semanticsを保持する | native store | duplicate fixture |
| WIN-J-005 | same period/minuteの最大値を保持する | store contract | upsert test |
| WIN-J-006 | 3か月prune以外で過去行を削除しない | DATA_PROTECTION | DB hash |
| WIN-J-007 | reloadで値を捏造しない | store contract | restart trace |
| WIN-J-008 | API停止中も既存last-goodを表示する | recovery | transport fixture |
| WIN-J-009 | local history errorを区別する | runtime states | error image |
| WIN-J-010 | daemonがUIと独立して記録する | daemon requirement | service trace |
| WIN-J-011 | daemon停止時の不可避gapを捏造しない | daemon policy | stop/restart trace |
| WIN-J-012 | 複数serverが同じDBへ二重登録しない | DB policy | concurrent writer |
| WIN-J-013 | singleton leaseでdaemonを一つに限定する | daemon policy | multi-start trace |
| WIN-J-014 | backup数世代を保持する | DB protection | backup inventory |
| WIN-J-015 | migration失敗時に旧DBを保持する | migration policy | failure injection |
| WIN-J-016 | clientはDBを破壊的再生成しない | data protection | source/audit |

### K. 異常系・境界・同時実行（16件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-K-001 | API未起動を表示する | runtime states | error fixture |
| WIN-K-002 | SSH名前解決失敗を説明する | setup error | error fixture |
| WIN-K-003 | SSHプロセス終了を検知する | process bridge | process trace |
| WIN-K-004 | 認証期限切れを区別する | auth state | auth fixture |
| WIN-K-005 | empty detailsを安全に表示する | details contract | empty fixture |
| WIN-K-006 | null quotaを未取得で表示する | API contract | null fixture |
| WIN-K-007 | malformed UTF-8を拒否する | security policy | malformed fixture |
| WIN-K-008 | oversized bodyを拒否する | security policy | oversize fixture |
| WIN-K-009 | stale thread rowsを表示しない | thread regression | stale fixture |
| WIN-K-010 | child window close後に購読を解除する | lifecycle | leak test |
| WIN-K-011 | 同じchild windowを二重生成しない | lifecycle | single-instance test |
| WIN-K-012 | main close時にchildを安全に閉じる | lifecycle | close trace |
| WIN-K-013 | monitor境界を跨ぐ座標を破綻させない | multi-monitor | geometry trace |
| WIN-K-014 | DPI変更で数px以上の中心ずれを出さない | centering | measured image |
| WIN-K-015 | window移動中にjitterを出さない | drag behavior | raw trace |
| WIN-K-016 | テストがユーザーのマウスを勝手に動かさない | user constraint | static/API scan |

### L. 証拠・独立評価・納品（16件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-L-001 | 全要求にownerを割り当てる | intake policy | ledger |
| WIN-L-002 | 全要求にtest oracleを割り当てる | intake policy | ledger |
| WIN-L-003 | 全要求に実画像または非画像証拠を割り当てる | AGENTS | evidence map |
| WIN-L-004 | 現行artifact SHAを証拠へ記録する | completion protocol | hash manifest |
| WIN-L-005 | 古い画像を現行証拠へ流用しない | AGENTS | freshness check |
| WIN-L-006 | X版とWindows版へ同じfixtureを入力する | parity rule | fixture hash |
| WIN-L-007 | 画面状態を正常/認証/警告/危険/0/100/エラーで確認する | AGENTS | image matrix |
| WIN-L-008 | 最小幅と高DPIを確認する | AGENTS | size matrix |
| WIN-L-009 | Setup/Settings/Graph/Threads/Legalを確認する | AGENTS | image matrix |
| WIN-L-010 | keyboard smokeを確認する | accessibility | raw log |
| WIN-L-011 | physical inputはユーザー許可なく実行しない | user constraint | explicit status |
| WIN-L-012 | 実装者と別の独立評価を実施する | completion protocol | audit report |
| WIN-L-013 | 独立評価FAIL/INCONCLUSIVEでHOLDにする | completion protocol | guard result |
| WIN-L-014 | completion guardを最後に実行する | AGENTS | command log |
| WIN-L-015 | installer install/uninstallを実機で確認する | customer requirement | host log |
| WIN-L-016 | 未達が一つでもあれば納品不可と記録する | AGENTS | release manifest |

### M. UX設計・導線・非スクロール（30件）

| ID | 要求 | 根拠 | 受入証拠 |
| --- | --- | --- | --- |
| WIN-M-001 | UXの目的、対象利用者、主要タスク、非目的を文書化する | WINDOWS_UX_SPEC §1 | UXレビュー台帳 |
| WIN-M-002 | 監視の主要値を一画面で理解できる情報階層を固定する | DESIGN 情報階層 | 主画面状態画像と視線順レビュー |
| WIN-M-003 | 主画面で主要値・状態・更新操作へ到達するためのページスクロールを要求しない | UX非スクロール規則 | 最小幅全状態画像、操作記録 |
| WIN-M-004 | Setupで接続を完了するための入力・説明・次操作を同一viewportに置く | UX導入導線 | Setup各状態画像 |
| WIN-M-005 | Settingsで主要設定と保存/取消/復旧操作を同一viewportに置く | UX設定導線 | Settings各状態画像 |
| WIN-M-006 | Graphで期間・metric・系列操作とグラフ本体を同時に認識できる | UXグラフ導線 | Graph各サイズ画像 |
| WIN-M-007 | Threadsで最初の比較対象と更新/閉じる操作をスクロールなしで認識できる | UXスレッド導線 | 0/1/2/3件画像 |
| WIN-M-008 | Legalは主監視導線を塞がず、戻る操作を常時提供する | UX法的情報導線 | Legal画像と遷移記録 |
| WIN-M-009 | ページ全体を縦スクロールさせる設計を採用しない | 非スクロール規則 | 全Window構造検査 |
| WIN-M-010 | 長い一覧・本文がある場合も主操作を隠さない固定viewport＋ページング/折りたたみを使う | 非スクロール規則 | overflow fixture |
| WIN-M-011 | 画面遷移はWindows標準のメニュー/ナビゲーションから開始できる | Windows作法 | メニュー操作記録 |
| WIN-M-012 | 現在画面、戻る、閉じる、設定、法的情報の位置と名称を全画面で一貫させる | UXナビゲーション | 全画面ナビゲーション表 |
| WIN-M-013 | 子画面は二重起動せず、既存画面を前面化する | lifecycle contract | single-instance trace |
| WIN-M-014 | 初回起動、接続済み再起動、接続失敗、認証要求の導線を別状態として定義する | Setup contract | state transition matrix |
| WIN-M-015 | ユーザーが次に行う操作を各失敗状態で一つ以上明示する | error UX | failure images/text inventory |
| WIN-M-016 | エラー表示は原因・影響・復旧操作を分離し、raw backend errorを出さない | SECURITY/UX | redaction/text audit |
| WIN-M-017 | 自動更新と手動更新の責務・待機・再入防止を可視化する | refresh contract | interaction trace |
| WIN-M-018 | 接続確認のたびにSetupを強制再表示しない | setup regression | two-launch trace |
| WIN-M-019 | メニュー項目にはアイコンだけでなく文字名とアクセシブル名を付ける | Windows UX/accessibility | menu inventory |
| WIN-M-020 | アイコンは意味・状態・クリック結果が一貫し、未対応glyphにフォールバックを持つ | icon rationale | asset/license/tooltip audit |
| WIN-M-021 | 文字の太さ・サイズ・コントラストは主値/補助値/状態/操作の役割差を示す | typography contract | visual review matrix |
| WIN-M-022 | Windows固有の見た目を採用する場合、X版との差分理由を文書化する | parity rationale | design decision record |
| WIN-M-023 | X版の欠点を根拠なく踏襲せず、意味論とデータ所有権だけを正本として継承する | parity rationale | native→Windows mapping |
| WIN-M-024 | 不要な装飾、派生値、同義説明を追加せず、表示所有者を一つにする | DESIGN ownership | semantic text inventory |
| WIN-M-025 | キーボードのみでメニュー、主要操作、戻る、閉じるへ到達できる | accessibility | keyboard traversal log |
| WIN-M-026 | フォーカスリング、hover、pressed、disabled、busyを視認できる | accessibility | state images |
| WIN-M-027 | DPI変更、異なるDPIの複数モニタ、画面端移動で中心・余白・文字を破綻させない | multi-monitor UX | geometry evidence |
| WIN-M-028 | アプリはユーザーのカーソル・フォーカス・入力デバイスを無断で奪わない | user constraint | static scan/default smoke |
| WIN-M-029 | 主画面のレイアウトは標準・最小・高DPIで同じ優先順位を保つ | responsive UX | size matrix |
| WIN-M-030 | UX文書の各判断に目的、代替案、採用理由、影響要求、受入証拠を紐付ける | UX governance | decision record audit |

## 3. 仕様未解決事項と証拠前提（実装禁止）

要求段階の既知差分は `WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md` で追跡しており、
全件の修正とfresh独立監査が終わるまでは仕様曖昧0件と宣言しない。製品証拠待ちU-01..U-05に対する
fixture/schema/oracle契約は `docs/evidence/GRAPH_PARITY_FIXTURE_CONTRACT_2026-08-22.md`、
`docs/evidence/UI_LABEL_INPUT_FIXTURE_CONTRACT_2026-08-22.md`、`docs/evidence/ARTIFACT_EVIDENCE_MANIFEST_CONTRACT_2026-08-22.md`
で固定した。これらの契約と独立抽出突合を閉じるまで `EXTRACTION_COMPLETE`へ変更しない。比較画像や3071点データを、
出所不明のままPASS根拠にしない。製品実装後の同一SHA実証は別の受入ゲートであり、抽出完了と混同しない。

1. U-01（契約定義済み／製品証拠待ち）: 比較画像2枚をX/Windows・artifact SHA・fixture・取得日時へ結び付ける。
2. U-02（契約定義済み／製品証拠待ち）: 3071点の同一期間・reset・now・timezone・raw sampleを再現fixture化し、
   時刻・値・折れ点・終端を正規化列で比較する。画像だけで同値としない。
3. U-03（方向確定／契約定義済み／製品証拠待ち）: Remainingは独立0–100%系列、モデル系列はドル/トークンscaleという
   方向は `DESIGN.md`/UX正本で固定済みだが、同一fixtureでのtransform証拠が未取得である。
4. U-04（Decision記録済み／独立抽出監査待ち／製品証拠待ち）: `UX-20260822-GRAPH-001` に記録した
   locale/Windows mappingによる意味同値を抽出候補とする。IDの存在をユーザー承認の証拠にせず、
   未登録の言換えを許可せず、raw label列・fresh画像・独立評価を揃える。
5. U-05（契約定義済み／製品証拠待ち）: 実マウスを動かさずにdrag/jitterを受入する方法を、静的API検査・隔離OS automation・
   opt-in物理試験の扱いへ分離し、通常のユーザー環境を操作しない。

次の項目は、ユーザー明示要求または既存正本により抽出上の方針を確定した。ただし実装後の
受入証拠はまだ存在せず、抽出完了と製品合格を混同しない。

- 多言語: `docs/LOCALIZATION.md` の10言語をWindows対応範囲とし、未知localeは決定的fallback。
- daemon/DB責務: Linux/WSL側daemonが記録・singleton・backup/migrationを担当し、Windowsは
  SSH local forwarding越しのread-only API clientとする。WindowsがDBを直接変更しない。
- スクロール: ユーザー指定により、主要情報・主要操作をスクロール必須にする設計を不採用とし、
  ページング/章切替/選択詳細で分割する。既存 `DESIGN.md` の許容記述はこの方針へ整合させる。
- Graph軸: `DESIGN.md` のGraph軸契約を正本とし、Remainingは独立0–100%系列、モデル系列は
  ドル/トークンmetricのscaleを使う。Windows固有に同一軸へ押し込む変更は不可とする。
- 表示文言差分: locale翻訳とWindows作法上のタイトル/メニュー差分は許容するが、意味単位、
  系列順、単位、所有者、状態表現、操作結果はX版と同一とする。
- 入力受入: ユーザーの実カーソルを動かさず、静的なカーソルAPI不使用確認と、隔離されたOS
  automation/message入力による実プロセス操作を別証拠とする。通常のユーザー環境でカーソルを奪う試験は行わない。

## 4. 実装開始ゲート

この文書の全226行に、根拠、所有者、実装対象、テスト、実機証拠、独立評価者、
合否条件が埋まり、U-01..U-05のfixture/schema/oracle契約が閉じ、独立抽出突合がPASSした時点で状態を
`EXTRACTION_COMPLETE`へ変更する。同一SHAの実機証拠は抽出後の製品受入ゲートで取得する。それまではコード変更を禁止する。

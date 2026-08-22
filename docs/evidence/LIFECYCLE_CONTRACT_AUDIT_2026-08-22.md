# Lifecycle Contract Audit — 2026-08-22

判定: **FAIL**（要求契約の行固有化が未完了）

証拠取得判定: **INCONCLUSIVE**（各行の独立評価欄が `証拠未取得` のまま）

## 監査範囲と方法

SOL から渡された task-local の次の3文書だけを read-only で監査した。ソース変更、テスト、ビルド、実行、画面取得は行っていない。

- `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md`

対象は `WIN-D-001..012`、`WIN-K-001..016`、`WIN-M-001..030` の58行である。

## 監査の根拠

- ベースラインは全行について actor/entry、precondition、action/observable、failure、persistence、evidence 等を具体化し、該当する状態・イベント直積を `applicable` または根拠付き `not-applicable` にすることを要求している（`WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md:48-82`）。
- トレーサビリティ設計は、WIN-D に active snapshot/epoch/stale/last-good と Legal の notice version/owner を、WIN-K に entry/re-entry/cancel/close/singleton/異常終了/再開を行固有 observable として割り当て、WIN-M の非スクロール行に viewport/overflow/clip/focus/DPI/状態別受入セルを要求している（`WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md:49-54`）。
- 行契約の抽出ゲート自身も、ライフサイクル条件を全該当行へ割り当て、カテゴリ既定値だけで閉じないよう要求している（`WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md:240-258`）。

## 共通欠落（全58行） — FAIL

`WINDOWS_REQUIREMENTS_ROW_CONTRACTS` の各行は11列の形だけは埋まっているが、次の値が行固有ではない。

- `precondition` は「起動/接続/データ/期間/サイズ/DPI/locale/再入の該当セルを明示」とする未展開のプレースホルダーで、`applicable`/`not-applicable`、入力値、状態遷移、モニタ/DPI条件がない。
- `observable` は「操作後に要求の結果を観測」「正本field・順序・状態が一致」とするテンプレートで、entry/re-entry/cancel/close/singleton/異常終了/再開のトリガー、期待状態、否定条件、観測値がない。
- `failure` は全行同じ last-good/再試行/再起動/キャンセル等の列挙で、どの失敗がその行に適用され、何を保持・除外・復旧するかがない。
- `test_oracle` は fixture 名と抽象的な expected だけで、境界入力、比較対象、合否式、依存IDがない。
- `independent_reviewer` は全行 `INCONCLUSIVE (証拠未取得)` であり、独立評価の raw/fresh-image/process/hash を満たす証拠はない。

従って、カテゴリ表（同文書:249-259）の既定値を継承するだけでは行契約を閉じられず、共通欠落だけで対象58行は抽出 `PASS` にできない。

## 行別欠落

### WIN-D（12行）

| ID | 行固有に欠ける契約 |
| --- | --- |
| WIN-D-001 | singleton の重複起動時の既存窓前面化、entry/re-entry、close/cancel、異常終了後の再開を observable/evidence にしていない。 |
| WIN-D-002 | empty snapshot の epoch/stale 判定、Failed rows=0、last-good、取得失敗からの完全復帰と entry/再表示を定義していない。 |
| WIN-D-003 | 単一行の canonical active snapshot、epoch/stale 除外、再入・取得失敗・再開時の保持境界がない。 |
| WIN-D-004 | 複数行の snapshot 世代、stale/failed 行除外、順序の入力・否定 fixture、再入/再開境界がない。 |
| WIN-D-005 | 親子の root/child/cycle の入力と child lifecycle（生成・更新・close・再開）、epoch/stale の観測がない。 |
| WIN-D-006 | orphan を独立保持する canonical 規則、親への誤結合の否定 oracle、epoch/stale/再取得・再開境界がない。 |
| WIN-D-007 | role/depth/model の正本列・欠測/invalid 表示、snapshot 世代と stale 除外、表示の再入/失敗保持がない。 |
| WIN-D-008 | context 使用率の型・単位・範囲・境界値、epoch/stale、null/取得失敗時の last-good がない。 |
| WIN-D-009 | 累積 token の正本・世代・欠測/重複・再起動保持と stale 除外がない。 |
| WIN-D-010 | 経過時間/指示年齢の基準時刻・再起動/再取得の保持境界、stale epoch と失敗復帰がない。 |
| WIN-D-011 | 認証前後の Legal entry/re-entry、notice version/owner、close/back、認証失敗時の保持がない。 |
| WIN-D-012 | GPL/third-party/font/schema/dependency の notice inventory、版・表示所有者・欠落時の失敗が行固有でない。 |

### WIN-K（16行）

| ID | 行固有に欠ける契約 |
| --- | --- |
| WIN-K-001 | API 未起動の entry、再接続/re-entry、cancel/close、異常終了・再開後状態と last-good がない。 |
| WIN-K-002 | SSH 名前解決失敗の入力/再試行/取消/close、復旧後再開、秘密情報を含まない具体的 observable がない。 |
| WIN-K-003 | SSH child process の異常終了イベント、購読解除、close/re-entry、再起動・再開順序がない。 |
| WIN-K-004 | 認証期限切れの epoch/世代、再認証 entry、cancel/close、失効中の表示保持と復旧がない。 |
| WIN-K-005 | empty details の snapshot/stale/child 境界、close/re-entry、失敗時 last-good がない。 |
| WIN-K-006 | null quota の null/未取得/復帰値、epoch、再取得/再開、失敗保持の具体値がない。 |
| WIN-K-007 | malformed UTF-8 の拒否境界、部分結果を出さない条件、再試行/cancel/close/復旧の oracle がない。 |
| WIN-K-008 | oversized body の数値上限、拒否前後の last-good、再試行/cancel/再開と証拠がない。 |
| WIN-K-009 | stale row の epoch 生成・比較（どの世代を stale とするか）、除外規則、Failed rows=0、last-good の observable がない。 |
| WIN-K-010 | child close → unsubscribe の child ID、イベント順、二重解除、異常終了/再開、singleton/re-entry の oracle がない。 |
| WIN-K-011 | child 二重生成防止の singleton key、既存窓前面化、close 後再生成、cancel/異常終了/再開がない。 |
| WIN-K-012 | main close → child close の順序・待機・失敗時保持、child 異常終了/re-entry/再開の証拠がない。 |
| WIN-K-013 | モニタ境界の topology、負座標/端/最大化・復元、DPI 混在時の座標系と許容値がない。 |
| WIN-K-014 | DPI 変更の前後値、スケール変換、中心ずれの数値許容値、再配置中の状態/再開証拠がない。 |
| WIN-K-015 | 移動中 jitter のサンプリング、閾値、DPI/モニタ境界、入力を奪わない条件の raw trace がない。 |
| WIN-K-016 | cursor API の write=0、入力デバイス/フォーカス非奪取の隔離 automation と否定証拠がなく、static scan/default smoke だけである。 |

### WIN-M（30行）

| ID | 行固有に欠ける契約 |
| --- | --- |
| WIN-M-001 | 目的・対象・主要タスク・非目的の decision record と entry/導線/証拠IDがない。 |
| WIN-M-002 | 情報階層の主値/補助値/状態の視線順、viewport、overflow/clip/focus/DPI の状態別セルがない。 |
| WIN-M-003 | 最小幅の具体的 viewport、スクロール位置、overflow/clip/focus、状態別 keyboard/re-entry がない。 |
| WIN-M-004 | Setup の同一viewport境界、入力/説明/次操作、失敗/cancel/back/re-entry の状態表がない。 |
| WIN-M-005 | Settings の保存/取消/復旧の entry/close/re-entry、viewport/overflow/focus/DPI がない。 |
| WIN-M-006 | Graph の操作待機/再入/cancel/close とサイズ/DPI/viewport/overflow のセルがない。 |
| WIN-M-007 | Threads の0/1/2/3件、更新/close、最初の比較対象、viewport/clip/focus の具体 oracle がない。 |
| WIN-M-008 | Legal の主導線非阻害、back/close、認証前後 entry と viewport/focus の証拠がない。 |
| WIN-M-009 | ページ非スクロールの viewport/overflow/clip/focus/DPI を具体化していない。 |
| WIN-M-010 | 固定 viewport、長文/一覧の overflow、paging/collapse、主操作保持と DPI 状態がない。 |
| WIN-M-011 | Windows menu entry、keyboard、re-entry、close/back の操作列と証拠がない。 |
| WIN-M-012 | 全画面の current/back/close/settings/legal の所有者・位置・名称表と singleton/re-entry がない。 |
| WIN-M-013 | child singleton key、既存窓前面化、close 後再生成、異常終了/再開の具体 oracle がない。 |
| WIN-M-014 | 初回/接続済み再起動/接続失敗/認証要求の遷移条件、cancel/close/re-entry がない。 |
| WIN-M-015 | 各失敗状態の一つの次操作の具体値、失敗中の last-good、cancel/再開がない。 |
| WIN-M-016 | 原因/影響/復旧の分離、raw error の否定 fixture、再試行/cancel/close と redaction 証拠がない。 |
| WIN-M-017 | auto/manual refresh の trigger、busy/wait、再入防止、cancel/stop/close、異常終了・再開がない。 |
| WIN-M-018 | 二回の接続確認/再起動の前提、Setup を出さない判定、失敗/re-entry/recovery がない。 |
| WIN-M-019 | menu の文字名/accessible name の正本、keyboard/focus/disabled と entry/close の証拠がない。 |
| WIN-M-020 | icon の意味/状態/クリック結果、未対応 glyph fallback の状態別 oracle と DPI/focus がない。 |
| WIN-M-021 | 主値/補助値/状態/操作の具体的サイズ・太さ・コントラスト、DPI/viewport 測定値がない。 |
| WIN-M-022 | Windows/X 差分の目的・代替案・採用理由・影響ID・受入証拠の decision record がない。 |
| WIN-M-023 | native→Windows の意味論/データ所有権 mapping と否定条件/依存IDがない。 |
| WIN-M-024 | 文言の semantic inventory、表示所有者1つの表、追加禁止の否定証拠がない。 |
| WIN-M-025 | keyboard traversal の順序、focus/viewport、戻る/閉じるの entry/re-entry と状態別証拠がない。 |
| WIN-M-026 | focus/hover/pressed/disabled/busy 各状態の入力、DPI、画像、re-entry/close がない。 |
| WIN-M-027 | DPI変更・異なるDPI複数モニタ・端移動の具体 topology/geometry/許容値、overflow/clip/focus の証拠がない。 |
| WIN-M-028 | cursor/focus/input non-steal の API write=0、隔離 automation、フォーカス遷移 raw trace がない。 |
| WIN-M-029 | 標準/最小/高DPIの具体サイズ、優先順位、viewport/overflow/clip/focus の比較 oracle がない。 |
| WIN-M-030 | 各UX判断の目的・代替案・理由・影響要求・受入証拠を結ぶ decision/trace ID がない。 |

## 必要な文書修正（最小）

1. `ROW_CONTRACTS` の各対象行に、`applicable/not-applicable(reason)` 付きの行固有イベント表を追加する。最低限、`entry → re-entry → cancel → close → singleton → abnormal termination → restart/resume` の trigger、observable、expected、否定条件、evidence ID、依存IDを列挙する。
2. WIN-D 各行に、canonical snapshot の世代/epoch、stale 比較、root/child/orphan/cycle、Failed rows=0、last-good、取得失敗時の完全復帰を要求本文と oracle に具体化する。Legal 行は notice inventory/version/owner を分離する。
3. WIN-K 各行に malformed/duplicate/unknown/oversize の具体境界を置き、K-010..012 は child の識別子・購読解除・close 順序・異常終了/再開、K-013..015 はモニタ topology/DPI/座標/許容値、K-016 は cursor API write=0 と隔離入力証拠を置く。
4. WIN-M の非スクロール・DPI・キーボード・状態行に viewport/overflow/clip/focus/DPI/状態別の受入セルを置き、M-013/014/017/018/027/028 には上記 lifecycle/geometry/non-steal 契約を行固有化する。
5. 修正後に各行の independent reviewer、raw log/fresh image/process/hash、PASS/FAIL/INCONCLUSIVE 式を artifact SHA と結び、未取得のまま `verified`/PASS にしない。

## 結論

対象58行は、テンプレート上の11列は存在するが、要求された lifecycle、stale epoch、child lifecycle、DPI/multi-monitor、cursor non-steal の行固有追跡が閉じていない。したがって本監査は **FAIL**。独立証拠については各行が `INCONCLUSIVE` のため、実装・テスト・評価へ進める根拠はない。要求抽出状態は変更していない。

## 追補（文書修正後）

上記FAILを受け、`docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md` にD/K/Mの58行を追加し、
各行へentry、イベント列、observable、否定条件、oracle、evidence ID、依存IDを割り当てた。
これは要求契約の修正であり、製品証拠を取得したものではない。新しい独立監査で58行を再突合するまで、
本監査の総合状態は `FAIL / INCONCLUSIVE / HOLD` として維持する。

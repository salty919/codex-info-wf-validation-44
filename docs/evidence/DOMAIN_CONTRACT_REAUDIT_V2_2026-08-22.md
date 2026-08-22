# Domain contract re-audit V2 (2026-08-22)

## 判定

総合判定: **FAIL**（行契約の具体性とDecision突合が未完了）

製品証拠判定: **INCONCLUSIVE**（今回の監査は文書だけを対象とし、実装・テスト・ビルド・実機を実行していない。両拡張文書自身も製品証拠未取得／`INCONCLUSIVE` と明記している。）

抽出状態は変更していない。コード、テスト、ビルド、実機、Windowsプロセスには触れていない。

## 監査範囲と突合結果

対象は次の文書と、そこから直接参照される行範囲だけとした。

- `docs/WINDOWS_LIFECYCLE_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_DATA_PROTECTION_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`
- `docs/WINDOWS_REQUIREMENTS_TRACEABILITY_DESIGN.md`
- `docs/LIVE_STATE_DECISION_MATRIX.md`
- `docs/TRACEABILITY_MATRIX.md`
- `docs/UX_DECISION_GRAPH_LABELS_2026-08-22.md`
- `docs/UX_DECISION_NON_SCROLL_2026-08-22.md`

`wc -l` の物理行数は lifecycle=72、data-protection=47 だった。要求上の「58行／32行」は表のID行数としては一致する。

| 拡張 | ID集合 | 件数 | BASELINEとの突合 |
| --- | --- | ---: | --- |
| lifecycle | `WIN-D-001..012`, `WIN-K-001..016`, `WIN-M-001..030` | 58 | BASELINE §カテゴリ表の12/16/30と一致 |
| data protection | `WIN-I-001..016`, `WIN-J-001..016` | 32 | BASELINE §カテゴリ表の16/16と一致 |

`WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md:8-9` は両拡張文書の同一ID行参照を要求しており、BASELINE `:107-108` と TRACEABILITY_DESIGN `:91-94` も拡張文書のカテゴリ参照を登録している。確認できた範囲では、IDの範囲外・重複・欠番はない。

Decisionとの突合は部分的である。`UX_DECISION_NON_SCROLL_2026-08-22.md:41-42` は D-002..004、K-013..015、M-003..010、M-025..029 を、`UX_DECISION_GRAPH_LABELS_2026-08-22.md:47-50` は M-019..024/M-030 を明示する。しかし、残りのD/K/M行とI/J全行には、対応するDecision IDまたはDecisionの影響行リストがない。`LIVE_STATE_DECISION_MATRIX.md:49` も `LIVE-001` をHOLDする記述であり、拡張WIN行への行単位リンクではない。したがって全拡張行のDecision突合は閉じていない。

## 行固有契約の評価

### lifecycle拡張

表の列自体（actor/entry/precondition、event sequence、observable/expected、failure/persistence/negative、oracle/dependency、evidence/review）は存在する（同文書 `:7-8`）。ただし、行固有の受入契約としては不十分である。

- D行（`:9-20`）は actor/entry と `active snapshot epoch e、root/child/orphan/cycle、0/1/複数、認証状態、再表示` が全行で同じで、各fixtureに対応する具体的な入力値、時刻、graph、認証状態が定義されていない。
- K行（`:21-36`）も `malformed/null/oversize/duplicate、認証epoch、child id、monitor topology/DPI、cursor policy` が全行で同じであり、境界値、失敗イベントの実データ、再入順序の識別子が行別にない。
- M行（`:37-66`）も同じ lifecycle/re-entry/monitor/DPI 前提を共有し、非スクロール・入力非奪取の各行について、対象Window、viewport、キーボード列、focus owner、測定値が行別に固定されていない。
- `open→snapshot(e)取得→表示→refresh再入→close→再open`、`child生成/更新/terminal/異常終了→次cycle再開` といったイベント列は、ほぼ全行でそのまま反復される。対象要求の差分を表すイベント入力・状態遷移・観測時点がないため、テンプレートを埋めただけである。
- failure/persistence 列の `snapshot不完全/epoch不一致/DB履歴のみ/孤児結合/role矛盾`、または K行の `旧完全snapshot保持、二重購読/二重起動/再入/秘密漏えい/partial publish` も、行ごとの発生条件、保持対象のハッシュ、復帰条件がない。
- oracle/dependency は `same snapshot manifest`、`fixture`、`expected=...`、`WIN-I/J→WIN-D/K/M` のようなラベル／カテゴリ矢印に留まり、依存する具体的な行ID、入力manifestのキー、比較式、許容値が定義されていない。

### data-protection拡張

I/Jの全32行（I `:9-24`、J `:25-40`）に7列はあるが、同様の反復が残る。特に正本field、unique key、row count/hash/fingerprint、snapshot世代、DB/API read-only境界、失敗時保持が多数の行で同じ定型文であり、行ごとの対象field・キー・保持境界・比較式が具体化されていない。

同文書 `:42-46` の必須保持値も、`response_bytes_max` 等を「契約値」「manifestへ記録」とするだけで、数値・単位・fixture値を定義していない。`backup世代は3` は具体化されているが、各行の期待hash、failure injection入力、atomic switchの観測結果への結合はない。これは列を埋めたテンプレートであり、各拡張行の具体的な oracle/依存/evidence を満たさない。

## 証拠状態

各行の evidence 列は `raw=WIN-*.jsonl`、`fresh-image=WIN-*.png`、process/window/keyboard/geometry/hash manifest、artifact SHA、独立判定を「必要」と列挙するだけで、今回の文書監査内に実データ、SHA、取得時刻、実行主体、独立レビュー結果は提示されていない。したがって製品受入をPASSへ進める根拠はなく、証拠については `INCONCLUSIVE` とする。

## 最小修正タスク

1. 58+32の各行について、entry/preconditionをfixtureの実入力値・境界値・状態に分解し、eventを具体的な入力→状態遷移→観測時点として行別に定義する。
2. observable/failure/oracleを、field/key/順序/数値境界/保持hash/復帰条件/否定条件を含む行別の判定式にする。カテゴリ名や同じfamily矢印だけを依存欄に置かない。
3. 全拡張IDをBASELINE、ROW_CONTRACTS、TRACEABILITY_DESIGN、関連Decision（またはDecision不要の理由を記録したDecision台帳）へ一意にリンクする。少なくとも現状Decision影響表にない行を放置しない。
4. 同一artifact SHAに結び付いたraw/image/process/DB/manifestと独立レビューを取得し、取得できない行はPASSに昇格させない。

この監査の結論は、文書のID範囲は整合しているが、行固有契約とDecision・製品証拠の閉包は未達である、というものである。

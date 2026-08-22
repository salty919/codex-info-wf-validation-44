# DOMAIN CONTRACT REAUDIT V12（2026-08-22）

## 判定概要

| 対象 | 判定 | 根拠 |
|---|---|---|
| 最新V11のID・行・列 | PASS | V11を226行として抽出、226 unique ID、重複ID0、未解析行0。requirement/input/observable/negative/dependency/evidenceの欠落0。 |
| 主台帳とのID突合 | PASS | `WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md` の226 IDとV11の差分はmissing/extraとも0。 |
| case / expected | PASS | 226/226行でinputにcase、observableとevidenceにexpectedまたはexpected_caseを検出。 |
| dependency | PASS | 依存参照440件、未知ID0、自己依存0。 |
| synthetic marker | PASS | V11行内のsynthetic系marker0。 |
| observableの共有field | PASS | 全カテゴリでfield種類数が行数と一致し、カテゴリ内の共有field0。Graph/Main/Threads/UXを含めID別field化を確認。 |
| negativeの要求・keyword固有性 | FAIL | 226行中178行が同一の `invalid requirement-specific counterpart => reject;retain safe state`。文言中に要求keyword、対象field、閾値または操作順がなく、共通テンプレートのまま。 |
| 製品実環境証拠 | INCONCLUSIVE | evidence欄のoracle/raw/fresh artifact/same SHAは要求宣言であり、実取得物は本監査範囲で未確認。 |

## 対象と方法

対象は `docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md` の末尾にある `行固有Normative Override V11（A–M）` セクションだけである。V10以前の履歴は判定対象から除外した。ファイルは2625行、V11開始は2397行、V11セクションは228行、行は226件である。読み取り専用の正規表現集計で、ID、必須列、case/expected、依存ID、synthetic marker、カテゴリ内field、negativeの重複を確認した。コード、テスト、ビルド、実機操作は行っていない。

## ID・列・依存の結果

- V11: 226行、226 unique ID、重複0、未解析行0。
- baseline: 226 ID。V11との差分はmissing 0、extra 0。
- 必須列欠落: 0。
- input `case=` 欠落: 0。
- observable/evidenceの expected/expected_case 欠落: 0。
- dependency参照: 440件。V11集合外0、自己依存0。
- `synthetic`、`synthetic_field`、`category_synthetic`、`template_fallback` のmarker: 0。

## 共有fieldの再判定

observableの `field=` をカテゴリ別に集計した結果は次のとおり。全カテゴリで `field_unique == rows` となり、V10/V9で確認されたカテゴリ共有fieldはV11では残っていない。

| Category | 行数 | field種類数 | 共有field |
|---|---:|---:|---:|
| A | 20 | 20 | 0 |
| B | 24 | 24 | 0 |
| C | 20 | 20 | 0 |
| D | 12 | 12 | 0 |
| E | 16 | 16 | 0 |
| F | 12 | 12 | 0 |
| G | 16 | 16 | 0 |
| H | 12 | 12 | 0 |
| I | 16 | 16 | 0 |
| J | 16 | 16 | 0 |
| K | 16 | 16 | 0 |
| L | 16 | 16 | 0 |
| M | 30 | 30 | 0 |

この集計では、Graph/Main/Threads/UXのfieldもID別の実在fieldとして扱われ、同一カテゴリ内のfield再利用は検出されなかった。したがって共有observable fieldについてはPASSとする。

## negative重複の再判定

negative文字列は全体で11種類しかなく、最大の共通文が178/226行である。

| negative | 件数 | 判定上の問題 |
|---|---:|---|
| `invalid requirement-specific counterpart => reject;retain safe state` | 178 | 「requirement-specific」と称するだけで、要求keyword、対象field、値域/閾値、操作順、保持対象を記録しない。 |
| `DNS/argv injection => reject;credential redacted` | 10 | DNS/argv系要求には対応するが、異なるカテゴリへ横断反復し、ID別の引数境界・保持対象がない。 |
| `transport/process failure => localized recovery;raw hidden` | 9 | 原因別の分類はあるが、各IDの具体的失敗条件・保持値がない。 |
| `double-open => focus singleton;no duplicate` | 6 | singleton要求には適用できるが、別IDへの共通再利用で操作順・対象windowがない。 |
| `write interruption => atomic old-or-new;old data retained` | 5 | 書込み系要求に対応するが、対象path/version/保持値がID別でない。 |

特に178行の文言は、要求本文をinput/observable/evidenceへ埋め込んだこととは独立して同一であり、V11が掲げる「keyword別negative分岐」を満たす証拠にならない。従って、共有fieldは解消したが、negativeの要求固有性はFAILを継続する。

補足として、IDを除き要求本文を正規化した集計では inputに共通値域（auth、percent、argv等）の反復があり、evidenceには `Setup.ssh_002` と `argv/host/user/listening` の同一正規化グループが3件ある。これは主FAILであるnegative重複とは分け、oracleの対象field/valueが実装取得物で一致するかを製品証拠段階で確認すべき事項として記録する。

## 製品証拠の分離

V11のevidence欄は各IDのoracle JSON、rawログ、fresh image/process/DB/host、same SHA、独立reviewerを指定している。しかし本監査ではそれらの実ファイル、画面、プロセス、DB/hostログ、SHA一致を取得・確認していない。コード、テスト、ビルド、実機が禁止された範囲のため、製品実環境の判定は構造判定と混ぜず **INCONCLUSIVE** とする。

## 結論

- **構造（ID/列/case/expected/dependency/synthetic）:** PASS。
- **共有observable field:** PASS（226/226がカテゴリ内一意）。
- **negativeの要求固有性:** FAIL（共通文178/226、他の共通分岐も残存）。
- **製品実環境証拠:** INCONCLUSIVE（未取得・未検証）。

最新V11のみを判定し、atomic assertions本文、抽出状態、製品実装、テスト、ビルド、実機証拠は変更していない。

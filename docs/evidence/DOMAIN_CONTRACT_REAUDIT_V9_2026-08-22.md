# DOMAIN CONTRACT REAUDIT V9（2026-08-22）

## 判定概要

| 判定対象 | 結果 | 根拠 |
|---|---|---|
| 最新 V8 の行・ID・列構造 | PASS | V8 226行、ID 226件・重複0、未解析行0。要求本文、input、observable、negative、dependency、evidence の欠落0。 |
| 主台帳との ID 突合 | PASS | `WINDOWS_REQUIREMENTS_EXTRACTION_BASELINE_2026-08-22.md`、`WINDOWS_REQUIREMENTS_ROW_REGISTER_2026-08-22.md`、`WINDOWS_REQUIREMENTS_ROW_CONTRACTS_2026-08-22.md` が各226 IDで、V8との差分は missing/extra とも0。 |
| case / expected の存在 | PASS | 226/226行で input に `case=`、observable/evidence に expected（または expected_case）を検出。 |
| dependency の実在性 | PASS | 依存参照440件、未知ID0、自己依存0。 |
| synthetic marker | PASS | V8行内の `synthetic` / `synthetic_field` / `category_synthetic` マーカー0。WIN-G-002の `fallback` は要求本文のドメイン語であり、synthetic marker には算入していない。 |
| カテゴリ内の要求固有性 | FAIL | A/Hの実在サブfieldは一意だが、B/C/D/E/F/G/I/J/K/L/Mでは同一semantic fieldを複数IDが共有する残存がある。negativeも共通分岐が反復する。 |
| 製品実環境証拠 | INCONCLUSIVE | V8の evidence 欄はoracle・raw・fresh artifact・同一SHAを要求する宣言であり、実画像、実プロセス、DB/hostログ、同一SHAの取得物は本監査範囲で未取得。 |

## 監査対象と方法

対象は `docs/WINDOWS_DOMAIN_ATOMIC_ASSERTIONS_2026-08-22.md` の `行固有Normative Override V8（A–M）` セクションだけとした。V1–V7の履歴行は判定対象から除外した。ファイルは2165行、V8セクション開始は243行、V8の行は226件である。読み取り専用の正規表現集計で、ID、列、case/expected、依存参照、marker、カテゴリ内field重複を確認した。コード、テスト、ビルド、実機操作は行っていない。

## ID・列の突合

- V8抽出: 226行、226 unique ID、重複ID 0、未解析override行 0。
- 必須列の欠落: requirement 0、input 0、observable 0、negative 0、dependency 0、evidence 0。
- baseline/register/row-contracts の各ID集合は226件で、V8との missing 0、extra 0。
- input、observable、dependency、evidence の文字列は各226 unique。これは各欄にIDまたは要求本文が含まれることも含むため、下記のsemantic field/normalized 判定と分離した。

## case / expected / dependency / marker

- 226/226行に要求別 input `case=` がある。
- 226/226行に observable の expected/expected_case がある。
- 226/226行に evidence の expected/expected_case がある。
- dependency参照は440件。V8 ID集合外参照0、行自身への参照0。
- `synthetic` 系markerは0。要求語としての `fallback` は WIN-G-002 の「未知言語は決定的fallbackを使う」に限られ、合成fieldを示すmarkerではない。

## カテゴリ内重複（要求固有性の残存FAIL）

observable の `field=` を抽出した結果は次のとおり。括弧内は「カテゴリ行数 / field種類数」、続く値は反復fieldの最大例である。

| Category | 行数 / field種類数 | 反復field |
|---|---:|---|
| A | 20 / 20 | なし |
| B | 24 / 1 | `Graph.rule` 24 |
| C | 20 / 3 | `Main.state` 18 |
| D | 12 / 1 | `Threads.lifecycle` 12 |
| E | 16 / 2 | `Setup.flow` 9、`Setup.ssh_tunnel` 7 |
| F | 12 / 2 | `Settings.persistence` 10 |
| G | 16 / 2 | `I18n.accessibility` 15 |
| H | 12 / 12 | なし |
| I | 16 / 3 | `Api.validation` 14 |
| J | 16 / 3 | `UsageStore.protection` 11、`Daemon.lifecycle` 3 |
| K | 16 / 4 | `Error.recovery` 12 |
| L | 16 / 2 | `Evidence.gate` 15 |
| M | 30 / 4 | `UX.decision` 26 |

したがって、A/Hで Parity、Account、Quota、Installer 等のID別fieldを明示した改善は確認できる。一方、V8全体を「全行のobservableが要求固有のfield境界を持つ」と判定するには、B/C/D/E/F/G/I/J/K/L/Mの共有fieldが残るため不足する。要求本文、case、expected exact式自体はIDごとに変わっていても、field境界の再利用はテンプレート残存としてFAILに分類する。

### negative の重複

negative文字列は226行中158種類で、行ごとの差分は増えている。しかし、次の共通分岐が複数IDに反復する。

- `null/invalid => explicit unavailable;no fabricated value`: 35行
- `classify cause;localized recovery;raw secret hidden`: 11行
- `DNS/argv injection => reject;credential redacted`: 8行
- `double-open => focus singleton;no duplicate`: 6行
- `drift/jitter threshold exceeded => FAIL;data unchanged`: 5行

これらは一部の同系統要求には妥当な共有oracleとなり得るが、V8の受入条件を「negativeも要求語・対象field・失敗保持まで行固有」と読む限り、少なくとも上記共通文は要求固有性の証明にならない。case/expectedが要求本文を含むことと、negativeが要求固有であることは別条件として扱う。

## 製品証拠の分離判定

V8の各 evidence 欄には `oracle_<ID>.json`、rawログ、fresh image/process/DB/host、同一SHA、独立reviewerが指定されている。ただし、これは証拠の要求仕様であって取得済み証拠ではない。本監査ではコード、テスト、ビルド、実機を禁止しているため、取得有無・内容・SHA一致を確認できない。よって製品実環境の判定は構造FAILとは混ぜず、独立に **INCONCLUSIVE** とする。

## 結論

- **構造（ID/列/case/expected/dependency/marker）:** PASS。
- **カテゴリ内の要求固有field・negative:** FAIL（共有semantic fieldと共通negativeが残存）。
- **製品実環境証拠:** INCONCLUSIVE（未取得・未検証）。

上記は最新V8だけの再監査結果であり、atomic assertions本文、抽出状態、製品実装、テスト、ビルド、実機証拠は変更していない。

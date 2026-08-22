# Domain contract 再監査 V5（2026-08-22）

## 判定

**FAIL（V3 Overrideに残る共通テンプレート）**

**INCONCLUSIVE（製品証拠）**

最新文書はV3 OverrideをA–DとE–Mに分けて226件持つ。V3だけを正本として再読込した。
V1/V2/V1の履歴は判定対象から除外した。入力・観測・否定には要求固有値が大幅に追加されて
いるが、依存と証拠oracleはカテゴリ共通テンプレートのままであり、全5列の行固有契約を
満たさない。製品証拠はコード等を実行していないため別ゲートのINCONCLUSIVEとする。

## 1. ID・列・要求本文の完全性

### PASS（V3の構造）

- V3 Override行数: 226。
- V3 ID unique: 226。A–D/E–Mの全IDに欠落・重複なし。
- baseline ID集合: 226。V3との差分なし。
- baseline要求本文とV3の requirement 値の不一致: 0。
- 全V3行に requirement、input、observable、exact-assert、negative/retention、dependency、
  evidence の7項目が存在。
- 旧6列主表のID行数は226。抽出状態 EXTRACTION_INCOMPLETE は保持。

旧V1/V2行を正本として再利用していないこと、V3のセクション境界を確認した上で計数した。

## 2. V3の行固有性と残存テンプレート

ID、fixture名、observed/expected式、要求本文のcase文字列、oracleファイル名などを正規化し、
カテゴリ内の実質的な契約値を比較した。

| カテゴリ | 行数 | input unique | observable unique | negative unique | dependency unique | evidence unique |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A | 20 | 15 | 15 | 15 | 1 | 1 |
| B | 24 | 18 | 18 | 18 | 1 | 1 |
| C | 20 | 11 | 11 | 11 | 1 | 1 |
| D | 12 | 12 | 12 | 12 | 1 | 1 |
| E | 16 | 13 | 13 | 13 | 2 | 1 |
| F | 12 | 7 | 7 | 7 | 1 | 1 |
| G | 16 | 9 | 9 | 9 | 1 | 1 |
| H | 12 | 10 | 10 | 10 | 1 | 1 |
| I | 16 | 16 | 16 | 16 | 1 | 1 |
| J | 16 | 13 | 13 | 13 | 2 | 1 |
| K | 16 | 16 | 16 | 16 | 1 | 1 |
| L | 16 | 15 | 15 | 13 | 1 | 1 |
| M | 30 | 12 | 12 | 12 | 1 | 1 |

### 改善を確認した点

- WIN-A-001〜008は、parity ledger、account/auth enum、quota percent、period、reset
  timestamp、owner、各要求に対応するfixtureと境界を持つ。
- WIN-J-001〜003はAPI/DB field、read-only、period tuple、reset_atを要求本文に結合している。
- WIN-E-004〜015はSSH user/host、alias、argv、clipboard、PID/listening、timeout、auth、
  reconnectなど要求語に応じた入力値・観測field・失敗条件を持つ。
- V3 requirement本文とのID対応は226件すべて一致した。

### 残存FAIL

1. dependencyはほぼ全カテゴリで semantic_source_WIN-X-NNN（explicit owner）という同一形で、
   実在する依存要求ID、依存先field、依存の順序・成立条件を示していない。IDと要求語を
   差し替えた識別子であり、dependency列の行固有契約になっていない。
2. evidenceは全カテゴリで oracle_WIN-X-NNN.json with concrete input/output、raw trace、
   fresh image/process/DB/host、same artifact SHA、independent reviewer という同一テンプレート。
   要求ごとの期待field、閾値、negative証拠、画像/API/DB oracleの判定式がない。
3. input/observable/negativeも全行が一意ではない。例えばAは15/20、Bは18/24、Cは11/20、
   Fは7/12、Gは9/16、Mは12/30に留まり、同じ操作・境界・保持文を要求本文caseだけで
   区別する行が残る。
4. rawのexact-assertは要求本文を含むが、expected_WIN-X-NNNはopaqueな期待名であり、
   実際の対象field・単位・閾値・順序・値を代替しない。

したがって、V3はV1/V2より具体化しているものの、「各行のinput/observable/negative/
dependency/evidenceがテンプレートだけでなく要求固有」という受入条件にはFAILである。

## 3. 製品証拠

### INCONCLUSIVE（構造FAILとは分離）

- 文書状態はEXTRACTION_INCOMPLETE。
- 文書は製品実証未取得・全行INCONCLUSIVEを明記している。
- fresh image、process/host/DB/API trace、artifact SHA同一性、独立reviewerの実取得結果はない。
- コード、テスト、ビルド、インストール、実機確認は実施していない。

## 4. 実施したread-only確認

atomic文書の行数・セクション境界をwc/rgで確認し、V3セクションだけを静的パースした。
V3 ID集合をbaselineと突合し、7項目のキー存在、要求本文一致、カテゴリ内正規化unique数を
計算した。対象atomic文書、baseline、主行契約台帳、抽出状態は変更していない。

## 5. 最小修正タスク

dependencyを実在IDと成立条件へ展開し、evidenceを要求別のexact oracleへ置き換える。
さらに重複しているinput/observable/negativeについて、要求ごとの数値・時刻・viewport・
状態・操作順・失敗保持をcase文字列以外の構造化値として固定する。その後、新規独立Lunaで
V3のみを再監査し、製品証拠を別ゲートで取得する。

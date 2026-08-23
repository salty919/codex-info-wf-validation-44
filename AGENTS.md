# Codex Info 開発ガイド

- ユーザーの変更と無関係な差分を戻さない。依頼なしにcommit、push、PR作成をしない。
- 製品要件の正本は`docs/PRODUCT_REQUIREMENTS.md`とし、wire、データ、UIの詳細は同文書が参照する仕様へ置く。
- 監査版ごとのEvidence、文書SHA一覧、サブエージェント作業台帳をrepositoryへ追加しない。再現可能なtestと必要最小限の仕様を残す。
- 要件漏れを理由にN倍、N二乗、N階乗、全直積へ展開しない。同じ観測結果は既存要件へ統合し、因果関係のある有限caseだけを追加する。
- 文書ごとのSHAを完了ブロッカーにしない。製品artifactを一意に識別するhashは記録できるが、内容評価の代用にしない。
- 外部authority値を推測しない。publisher、certificate、対応OS、保証値が未指定なら、公開・対応表明・mutationを行わないfail-closed動作を要件化する。
- lockfileと既存package managerを守る。検索は`rg`を優先する。
- 変更後は対象に最も近いformatter、check、testを実行する。testが0件の結果をPASSにしない。
- UI変更は実画面、Windows固有動作は実Windowsで確認する。環境上確認できない項目をPASSと報告しない。

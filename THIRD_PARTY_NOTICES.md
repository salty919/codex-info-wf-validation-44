<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Third-party notices

Copyright (C) 2026 salty919

このリポジトリの独自コードと文書は、別途記載した第三者物を除き GPL-3.0-only で提供します。第三者物は、以下に記載する各上流ライセンスのままです。

## Noto Sans JP

- 対象: `assets/NotoSansJP.ttf`
- 著作権: Copyright 2022 The Noto Project Authors
- フォント内の著作権・予約フォント名: `(c) 2014-2021 Adobe (http://www.adobe.com/), with Reserved Font Name 'Source'.`
- ライセンス: SIL Open Font License, Version 1.1 (OFL-1.1)
- このフォントは GPL-3.0-only の対象に取り込まず、OFL-1.1 のまま再配布します。全文は [LICENSES/OFL-1.1.txt](LICENSES/OFL-1.1.txt) を参照してください。
- 出所: [Noto CJK](https://github.com/notofonts/noto-cjk)

## Codex CLI 生成プロトコルスキーマ

- 対象: `protocol/v2/GetAccountRateLimitsResponse.json`、`protocol/v2/GetAccountResponse.json`、`protocol/v2/PlanType.canonical.json`、`protocol/thread/ThreadListParams.canonical.json`、`protocol/thread/ThreadListResponse.canonical.json`
- 出所: OpenAI Codex CLI 0.147.0 の `codex app-server generate-json-schema --out <empty-output-directory>`（`--experimental` なし）で生成したスキーマを、このプロジェクトの契約用に保存したものです。
- 上流: [OpenAI Codex の `rust-v0.147.0` タグ](https://github.com/openai/codex/tree/rust-v0.147.0)
- 著作権: Copyright 2025 OpenAI
- ライセンス: OpenAI Codex の [Apache-2.0 ライセンス](https://github.com/openai/codex/blob/rust-v0.147.0/LICENSE)。保存した生成物は GPL-3.0-only に再ライセンスせず、Apache-2.0 の出所を保持します。全文は [LICENSES/Apache-2.0.txt](LICENSES/Apache-2.0.txt) を参照してください。
- 著作権通知の控え: [LICENSES/OPENAI-CODEX-NOTICE.txt](LICENSES/OPENAI-CODEX-NOTICE.txt)

### OpenAI Codex notice

`protocol/`に保存した上記5件の生成スキーマについて、上流の著作権表示を保持します。Apache-2.0本文とこの著作権表示を、スキーマを含むソース・バイナリ配布物から分離しないでください。

## Slint

- 対象: Slint `1.17.1`（`slint`、`slint-build`）
- Slint が提示するライセンス選択肢のうち、このプロジェクトは GPL-3.0-only を選択して利用します（商用ライセンス選択肢は選択していません）。
- 上流: [slint-ui/slint v1.17.1](https://github.com/slint-ui/slint/tree/v1.17.1)

## Cargo 依存クレート

`Cargo.toml` / `Cargo.lock` に記録された直接・間接の Cargo 依存クレートは、それぞれの上流が指定するライセンスと著作権表示を保持します。これらを本プロジェクトの GPL-3.0-only として再ライセンスするものではありません。各クレートのライセンスと出所は、ロックされたパッケージの上流メタデータで確認でき、リポジトリでは `cargo deny check licenses sources` により検査します。

Linux向けの固定依存グラフで許可される SPDX ライセンス集合は `deny.toml` に固定しています。バイナリを配布する場合は、`cargo deny list --format tsv --layout license` の結果と各クレートの上流 `LICENSE` / `NOTICE` / 著作権表示をリリース成果物へ同梱してください。`rusqlite` の `bundled` 機能でリンクされる SQLite 本体は Public Domain ですが、`libsqlite3-sys` ラッパーのMIT表示とSQLiteの出所も保持します。現リポジトリは依存クレートのソースをvendorしていないため、依存取得不能なバイナリ配布ではこの通知手順を満たすまでリリースしません。

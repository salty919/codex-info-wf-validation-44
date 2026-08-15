# Third-party notices

このリポジトリの独自コードと文書は、別途記載した第三者物を除き GPL-3.0-only で提供します。第三者物は、以下に記載する各上流ライセンスのままです。

## Noto Sans JP

- 対象: `assets/NotoSansJP.ttf`
- 著作権: Copyright 2022 The Noto Project Authors
- ライセンス: SIL Open Font License, Version 1.1 (OFL-1.1)
- このフォントは GPL-3.0-only の対象に取り込まず、OFL-1.1 のまま再配布します。全文は [LICENSES/OFL-1.1.txt](LICENSES/OFL-1.1.txt) を参照してください。
- 出所: [Noto CJK](https://github.com/notofonts/noto-cjk)

## Codex CLI 生成プロトコルスキーマ

- 対象: `protocol/v2/GetAccountRateLimitsResponse.json`、`protocol/v2/GetAccountResponse.json`、`protocol/v2/PlanType.canonical.json`、`protocol/thread/ThreadListParams.canonical.json`、`protocol/thread/ThreadListResponse.canonical.json`
- 出所: OpenAI Codex CLI 0.147.0 の `codex app-server generate-json-schema --out <empty-output-directory>`（`--experimental` なし）で生成したスキーマを、このプロジェクトの契約用に保存したものです。
- 上流: [OpenAI Codex の `rust-v0.147.0` タグ](https://github.com/openai/codex/tree/rust-v0.147.0)
- ライセンス: OpenAI Codex の [Apache-2.0 ライセンス](https://github.com/openai/codex/blob/rust-v0.147.0/LICENSE)。保存した生成物は GPL-3.0-only に再ライセンスせず、Apache-2.0 の出所を保持します。全文は [LICENSES/Apache-2.0.txt](LICENSES/Apache-2.0.txt) を参照してください。

## Slint

- 対象: Slint `1.17.1`（`slint`、`slint-build`）
- Slint が提示するライセンス選択肢のうち、このプロジェクトは GPL-3.0-only を選択して利用します（商用ライセンス選択肢は選択していません）。
- 上流: [slint-ui/slint v1.17.1](https://github.com/slint-ui/slint/tree/v1.17.1)

## Cargo 依存クレート

`Cargo.toml` / `Cargo.lock` に記録された直接・間接の Cargo 依存クレートは、それぞれの上流が指定するライセンスと著作権表示を保持します。これらを本プロジェクトの GPL-3.0-only として再ライセンスするものではありません。各クレートのライセンスと出所は、ロックされたパッケージの上流メタデータで確認でき、リポジトリでは `cargo deny check licenses sources` により検査します。

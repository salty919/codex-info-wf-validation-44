<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Third-party notices

Copyright (C) 2026 salty919

このリポジトリの独自コードと文書は、別途記載した第三者物を除き GPL-3.0-only で提供します。第三者物は、以下に記載する各上流ライセンスのままです。

## Noto Sans JP / Noto Sans CJK KR

- 対象: `assets/NotoSansJP.ttf`、`assets/NotoSansKR.otf`
- フォント内の著作権表示: `NotoSansJP.ttf`は`(c) 2014-2021 Adobe (http://www.adobe.com/), with Reserved Font Name 'Source'.`、`NotoSansKR.otf`は`© 2014-2021 Adobe (http://www.adobe.com/).`
- ライセンス: SIL Open Font License, Version 1.1 (OFL-1.1)
- フォントファイル自体は GPL-3.0-only へ再ライセンスせず、OFL-1.1 のまま同梱・再配布します。全文は [LICENSES/OFL-1.1.txt](LICENSES/OFL-1.1.txt) を参照してください。
- `NotoSansKR.otf`は韓国語catalogの欠字を避けるために追加したNoto Sans CJK KRのRegularフォントです。日本語・韓国語以外のcatalogも、同梱フォントのLatin/CJKグリフを使用します。
- 出所: [Noto Sans JP（Google Fonts）](https://github.com/google/fonts/tree/main/ofl/notosansjp)、[Noto Sans CJK KR 2.004](https://github.com/notofonts/noto-cjk/tree/Sans2.004)

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

## Windows client / .NET NuGet runtime

`windows-client/` の独自コードは本リポジトリの GPL-3.0-only に従う。一方、
`windows-client/**/packages.lock.json` で固定する NuGet パッケージは、それぞれの
上流ライセンスのままであり、GPL-3.0-only に再ライセンスしない。通常顧客向けWindows
配布はself-containedであるため、成果物に実際に含まれる.NET runtimeの版、著作権表示、
ライセンス本文、第三者通知もartifact manifestから収集し、同じinstaller payloadへ含める。
収集結果が一つでも未確定のself-contained成果物は配布しない。

- Avalonia UI: `Avalonia`、`Avalonia.Desktop`、`Avalonia.Themes.Fluent` と、同じ
  `12.1.1` 系の `Avalonia.FreeDesktop`、`Avalonia.FreeDesktop.AtSpi`、
  `Avalonia.HarfBuzz`、`Avalonia.Native`、`Avalonia.Remote.Protocol`、
  `Avalonia.Skia`、`Avalonia.Win32`、`Avalonia.X11`。著作権は Copyright 2013-2026
  The AvaloniaUI Project、ライセンスは MIT。上流は
  [AvaloniaUI/Avalonia](https://github.com/AvaloniaUI/Avalonia) である。
- `Avalonia.BuildServices` `11.3.2` は build-time のみの MIT 依存であり、著作権は
  Copyright 2023-2025 The AvaloniaUI Project。リリースの runtime payload には含めない。
- `MicroCom.Runtime` `0.11.6`（Copyright 2021 Nikita Tsukanov）、
  `HarfBuzzSharp` `8.3.1.3`、`SkiaSharp` `3.119.4`、および target RID に対応する
  `HarfBuzzSharp.NativeAssets.*` / `SkiaSharp.NativeAssets.*` は MIT である。後二者の
  native package にはさらに上流の第三者通知があるため、以下の収集手順で package 内の
  `LICENSE.txt` と `THIRD-PARTY-NOTICES.txt` を必ず配布物へコピーする。
- `Avalonia.Angle.Windows.Natives` `2.1.27548.20260419` は ANGLE Project Authors の
  BSD-3-Clause である。本文は [LICENSES/BSD-3-Clause-ANGLE.txt](LICENSES/BSD-3-Clause-ANGLE.txt)
  にも保持する。
- `ScottPlot` / `ScottPlot.Avalonia` `5.1.59` は、Windowsグラフのローカル描画に
  使用するMIT依存である。著作権は Copyright (c) 2018 Scott Harden / Harden
  Technologies, LLC、上流は [ScottPlot/ScottPlot](https://github.com/ScottPlot/ScottPlot)
  である。実行時のネットワークサービスやテレメトリには使用しない。
- Windowsの標準セットアップウィザードは `Inno Setup 7.1.0` で生成する。
  Copyright (C) 1997-2026 Jordan Russell、Portions Copyright (C) 2000-2026
  Martijn Laan。配布条件は [Inno-Setup.txt](LICENSES/Inno-Setup.txt) に保持する。
- Linux 向けに restore される `Tmds.DBus.Protocol` `0.94.1` と
  `System.IO.Pipelines` `8.0.0` は MIT である。Windows の publish payload には
  target RID が選んだ runtime asset だけを入れる。

MIT 本文は [LICENSES/MIT.txt](LICENSES/MIT.txt) に保持する。テスト専用の
`Microsoft.NET.Test.Sdk`、xUnit、coverage/test-host package は製品 payload へ含めない。
それでも lock file を変更する場合は、実行時・テスト時を問わず上流メタデータを再確認する。

Windows バイナリを配布する前に、`dotnet restore --locked-mode` を済ませた Windows
環境で、次を publish 出力ディレクトリへ実行する。

```powershell
.\windows-client\tools\Collect-ThirdPartyNotices.ps1 -Destination <publish-directory>
```

このスクリプトとinstaller build gateは root の本通知、`LICENSES/`、埋め込みフォントの
通知、Windows native asset package、およびself-contained publishへ実際に含まれた.NET
runtimeの通知を収集する責務を持つ。必要なruntime/packageまたは通知ファイルが見つからなければ
失敗する。通知のない配布物はリリースしない。

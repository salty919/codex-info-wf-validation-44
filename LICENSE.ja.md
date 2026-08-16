<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# GNU GPLv3 日本語案内

このファイルは、`LICENSE`に収録したGNU General Public License version 3（GPLv3）を日本語で参照するための案内です。ここにある説明やリンクは法的な正文・契約条件ではありません。ライセンスの解釈、配布条件、免責、特許条項、追加許可を含むすべての正式な条件は、同梱の英語正文[LICENSE](LICENSE)を優先します。

## 正式な本文

- 英語正文: [LICENSE](LICENSE)
- FSF/GNUプロジェクトの日本語翻訳（非公式翻訳）: <https://www.gnu.org/licenses/gpl-3.0.ja.html>
- 翻訳が法的正文ではないことの説明: <https://www.gnu.org/licenses/translations.html>

## このリポジトリでの適用範囲

- 著作権表示のない独自コードと文書は、`GPL-3.0-only`で提供します。これは「GPL version 3のみ」を意味し、後のGPLバージョンへ自動的に切り替わる指定ではありません。
- 改変・再配布時は、GPLv3の条件に従い、著作権表示、ライセンス本文、対応するソース提供条件を保持してください。改変ファイルには変更を明示してください。
- `protocol/`に保存したCodex CLI生成スキーマ、同梱フォント、Slint、Cargo依存クレートは独自コードとは別の第三者物です。各上流ライセンスを再ライセンスせず、[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)、[assets/NOTICE.txt](assets/NOTICE.txt)、[LICENSES/](LICENSES/)を配布物へ含めてください。
- Noto Sans JPとNoto Sans CJK KRはSIL Open Font License 1.1、生成スキーマはApache-2.0です。これらの条項をGPL-3.0-onlyの説明で置き換えないでください。

## 配布時の確認

配布物には、少なくとも次を同梱してください。

1. GPLv3英語正文（`LICENSE`）とこの著作権表示。
2. 第三者通知（`THIRD_PARTY_NOTICES.md`、`assets/NOTICE.txt`）。
3. `LICENSES/`内の上流ライセンス本文と、生成スキーマのOpenAI著作権通知。
4. 改変したソース、またはGPLv3が要求するソース提供方法の案内。

依存クレートの一覧と許可されるSPDX集合は`deny.toml`および`cargo deny check licenses sources`で確認します。依存物のライセンスを確認できないバイナリは配布しないでください。


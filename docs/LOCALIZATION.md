<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# 多言語化・日本語表示仕様

Codex Infoの固定UI文言、時刻表示、フォント、配布ライセンスに関する仕様です。GPLv3の正式な条件は[LICENSE](../LICENSE)に定めます。

## 対応言語とフォント

起動時localeのprimary language subtagに対応するcatalogとフォントを選びます。

| primary subtag | 表示言語 | フォント |
| --- | --- | --- |
| `ja` | 日本語 | `Noto Sans JP` |
| `en` | English | `Noto Sans JP` |
| `zh` | 简体中文 | `Noto Sans CJK KR`（共通Han字形） |
| `ko` | 한국어 | `Noto Sans CJK KR` |
| `es` | Español | `Noto Sans JP` |
| `fr` | Français | `Noto Sans JP` |
| `de` | Deutsch | `Noto Sans JP` |
| `pt` | Português | `Noto Sans JP` |
| `it` | Italiano | `Noto Sans JP` |
| `ru` | Русский | `Noto Sans JP` |

localeは`LC_ALL`、`LC_MESSAGES`、`LANG`の順に最初の非空値を採用します。encoding suffix（`.UTF-8`）とmodifier（`@...`）を除き、`-`と`_`を同じ区切りとしてprimary subtagを判定します。`C`、`POSIX`、不正値、未対応言語は英語catalogへ対応付け、`zh_TW`と`zh-Hant`は簡体字catalogへ対応付けます。

localeとtimezoneはプロセス起動時に確定します。`TZ`には`Asia/Tokyo`や`Europe/Berlin`などのIANA IDを指定でき、無効値はUTCへ対応付けます。

## 時刻基準

- epoch秒、並び順、期間の境界はUTCで保持します。
- 絶対時刻、履歴期間、グラフ横軸は起動時timezoneへ変換し、数値UTC offset（例`+09:00`）を付けます。
- 経過時間と残り時間はUTC秒の差分を各言語の単位へ変換します。
- 無効epochは値欄を`—`、グラフ軸を空値、履歴選択肢を除外として表示します。

## 固定文言

固定UI文言はcatalogで管理します。thread title、email、モデル名、製品名、ライセンス名、ログ生値は原文を表示し、数値とepoch秒だけを表示時のlocale・timezoneへ変換します。

日本語・韓国語フォントは`assets/NotoSansJP.ttf`と`assets/NotoSansKR.otf`をSlintへ埋め込み、起動時localeのフォントを各Windowへ適用します。フォントのOFL-1.1通知は[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)と[assets/NOTICE.txt](../assets/NOTICE.txt)に記載します。

ネイティブタイトルバーは使用しません。各Windowのボタン以外の画面領域を移動用に使い、画面内タイトル領域、操作記号、固定文言は埋め込みフォントでlocale表示します。Graphだけは四隅の広い対角領域と辺から枠をリサイズし、最大化／復元も提供します。

## 配布ライセンス

英語の[LICENSE](../LICENSE)がGPLv3の正文です。[LICENSE.ja.md](../LICENSE.ja.md)は日本語案内です。独自コードと文書は`GPL-3.0-only`、生成スキーマはApache-2.0、同梱フォントはOFL-1.1、SlintとCargo依存クレートは各上流ライセンスで提供します。

ソース・バイナリ配布物には`LICENSE`、[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)、[assets/NOTICE.txt](../assets/NOTICE.txt)、[LICENSES/](../LICENSES/)を同梱します。

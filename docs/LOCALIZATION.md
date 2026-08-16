<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# 多言語化・日本語表示仕様

この文書は、Codex Infoの固定UI文言、時刻表示、同梱フォントを変更する人向けの短い運用仕様です。実際のライセンス条件は[LICENSE](../LICENSE)を参照してください。

## 対応言語

起動時のlocaleから、次のprimary language subtagを選びます。

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

`LC_ALL`、`LC_MESSAGES`、`LANG`の順に、最初の空でない値だけを使います。`C`、`POSIX`、不正な値、未対応言語は英語として扱い、下位の変数へ進みません。encoding suffix（`.UTF-8`）とmodifier（`@...`）を除外し、`-`と`_`を同じ区切りとして扱います。`zh_TW`や`zh-Hant`も、今回のcatalogでは簡体字へ安全にフォールバックします。`LANGUAGE`は参照しません。

表示確認の例:

```bash
LANG=ja_JP.UTF-8 LC_ALL= TZ=Asia/Tokyo CODEX_INFO_PREVIEW=normal ./run.sh
LANG=en_US.UTF-8 LC_ALL= TZ=Europe/Berlin CODEX_INFO_PREVIEW=normal ./run.sh
LANG=ko_KR.UTF-8 LC_ALL= CODEX_INFO_PREVIEW=legal ./run.sh
```

localeは`I18n`へ起動時に保存されます。実行中に環境変数を変更しても、既存Windowの言語やtimezoneを再選択しません。変更後は再起動してください。

## 時刻基準

- 内部のepoch秒、並び順、期間の境界はUTCのまま保持します。
- 絶対時刻、履歴期間、グラフ横軸は起動時に解決したIANA timezoneへ変換し、数値UTC offset（例`+09:00`）を付けます。
- 経過時間・残り時間は`now_utc - timestamp_utc`の差分で計算し、DSTによる時計の飛びを混ぜません。
- `TZ`にIANA ID（例`Asia/Tokyo`、`Europe/Berlin`）を指定できます。POSIX rule文字列（`JST-9`）や裸のoffset（`+09:00`）は受け付けず、無効値はUTCへフォールバックします。
- 無効なepochは、値欄では`—`、グラフ軸では空値、履歴選択肢では除外します。推測した時刻を表示しません。

## 日本語表示に必要なもの

`ui/app.slint`が`assets/NotoSansJP.ttf`と`assets/NotoSansKR.otf`を直接importし、`UiStrings.font-family`をWindow全体の`default-font-family`へ設定します。そのため、OSのfontconfig設定やホスト側の日本語フォント有無に表示が依存しません。

変更時は次を守ってください。

1. 固定文言をSlint/Rustへ直接追加せず、`TextKey`と`UiStrings`へ登録する。
2. `TextKey::ALL`の全キーについて10 catalogが空文字にならないことを単体テストで確認する。
3. ユーザー入力のthread title、email、モデル名、製品名、ライセンス名、ログ生値は翻訳しない。数値とepoch秒は表示時だけlocale/timezoneへ変換する。
4. 日本語の追加文言はglyph manifest（この表と`src/i18n.rs`の日本語catalog）へ反映し、`CODEX_INFO_PREVIEW=normal|auth|warning|error|legal`の新しい画像で欠字・文字化け・clipを目視する。
5. 韓国語を変更した場合は`NotoSansKR.otf`のOFL-1.1通知も同時に確認する。

UIの受入では、900×480のMain/Threads、700×480以上のGraph、Legal Windowを少なくとも日本語で確認します。状態別プレビューはリポジトリ直下のREADMEに記載した`CODEX_INFO_PREVIEW`値を使い、最後の編集後に新しいプロセスからキャプチャしてください。

## ライセンス文書の扱い

`LICENSE`はGNU GPLv3の英語正文であり、変更しません。`LICENSE.ja.md`は日本語の参照導線です。フォント、生成スキーマ、Slint、Cargo依存クレートは各上流ライセンスのまま再配布し、`THIRD_PARTY_NOTICES.md`、`assets/NOTICE.txt`、`LICENSES/`を配布物から省略しないでください。

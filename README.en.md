<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Codex Info

Codex Info reads rate limits, reset periods, local usage history, and running
threads from the Codex App Server and displays them in a Rust/Slint X11 window.
The Japanese README is the primary setup guide: [README.md](README.md).

## Quick start

```bash
git clone https://github.com/salty919/codex_info_v2.git
cd codex_info_v2
./run.sh
```

The host needs Rust/Cargo, an X11 display (WSLg is supported), and a `codex`
CLI that can run `codex app-server --stdio`. Authentication remains owned by
the Codex CLI; this application does not save passwords, API keys, or tokens.

## Localization and time zones

Fixed UI copy follows the first non-empty value in `LC_ALL`, `LC_MESSAGES`, and
`LANG`. Japanese, English, Simplified Chinese, Korean, Spanish, French,
German, Portuguese, Italian, and Russian are included. Unsupported, `C`, and
`POSIX` locales fall back to English. The selected language and IANA time zone
are pinned at process startup. Absolute timestamps use that time zone, while
elapsed and remaining durations use UTC seconds.

To review Japanese rendering in WSL:

```bash
LANG=ja_JP.UTF-8 LC_ALL= TZ=Asia/Tokyo CODEX_INFO_PREVIEW=normal ./run.sh
```

Japanese and Korean fonts are embedded in `assets/`; no host font install is
normally required. See [LOCALIZATION.md](docs/LOCALIZATION.md) for the locale,
time-zone, glyph, and visual-review contract.

## Licensing

Original source and documentation are GPL-3.0-only; the authoritative license
text is [LICENSE](LICENSE). The Japanese file [LICENSE.ja.md](LICENSE.ja.md)
is only a reference guide. Noto fonts, generated protocol schemas, Slint, and
Cargo dependencies retain their upstream licenses. Keep
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [assets/NOTICE.txt](assets/NOTICE.txt),
and [LICENSES/](LICENSES/) with source or binary distributions.


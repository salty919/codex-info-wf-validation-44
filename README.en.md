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

Fixed UI copy selects the first non-empty value in `LC_ALL`, `LC_MESSAGES`,
and `LANG`. Japanese, English, Simplified Chinese, Korean, Spanish, French,
German, Portuguese, Italian, and Russian are included; `C`, `POSIX`, and
unsupported locales use English. The selected language and IANA time zone are
pinned at process startup. Absolute timestamps use that time zone, while
elapsed and remaining durations use UTC seconds. `TZ` accepts an IANA ID and
uses UTC for invalid values.

Japanese and Korean fonts are embedded from `assets/`. See
[LOCALIZATION.md](docs/LOCALIZATION.md) for the locale, time-zone, and font
specification.

## Usage display

Running threads show context usage and its limit as `usage% / limit tokens`,
derived from the cumulative token count and `model_context_window`.
The graph offers individual visibility controls for remaining quota and
LUNA/TERRA/SOL; all series start visible, and hidden series use muted color and
labels. Intervals where no model's cumulative usage changes appear as faint
background bands, and right-edge values use series-colored leader lines to the
corresponding endpoints. Native title bars are disabled; each window provides
its own embedded-font title area with move, minimize, and close controls.
All frameless windows can be moved by dragging any non-button surface. Graph is
resizable from 700x480 pixels with no upper bound; its enlarged corner hit
areas and directional cursors make diagonal resizing easier. Main, Threads,
and Legal remain fixed-size and do not expose maximize or resize controls.

## Licensing

Original source and documentation are GPL-3.0-only. The authoritative GPLv3
text is [LICENSE](LICENSE), and [LICENSE.ja.md](LICENSE.ja.md) provides a
Japanese guide. Noto fonts, generated protocol schemas, Slint, and Cargo
dependencies retain their upstream licenses. Source and binary distributions
include [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md),
[assets/NOTICE.txt](assets/NOTICE.txt), and [LICENSES/](LICENSES/).

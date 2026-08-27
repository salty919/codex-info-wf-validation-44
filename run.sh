#!/usr/bin/env bash
# Copyright (C) 2026 salty919
# SPDX-License-Identifier: GPL-3.0-only

set -euo pipefail

BASE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# WSLg exposes both Wayland and X11.  winit 0.30 selects Wayland whenever
# WAYLAND_DISPLAY or WAYLAND_SOCKET is set; WINIT_UNIX_BACKEND was removed in
# winit 0.29 and therefore cannot force the backend anymore.  This project
# intentionally builds only the X11 backend, so make that choice explicit.
# Leave the X11 scale factor unset so winit follows the OS Xft.dpi setting
# (and falls back to XRandR when no Xft.dpi value is available).
unset WAYLAND_DISPLAY WAYLAND_SOCKET WINIT_X11_SCALE_FACTOR
export LIBGL_ALWAYS_SOFTWARE="1"
export MESA_LOADER_DRIVER_OVERRIDE="llvmpipe"

CODEX_INFO_CARGO="$(command -v cargo 2>/dev/null || true)"
if [[ -z "$CODEX_INFO_CARGO" && -n "${HOME:-}" && -x "$HOME/.cargo/bin/cargo" ]]; then
    # Rustup is commonly installed here, but non-login shells do not always
    # source ~/.cargo/env before executing a repository script.
    CODEX_INFO_CARGO="$HOME/.cargo/bin/cargo"
fi
if [[ -z "$CODEX_INFO_CARGO" ]] && command -v rustup >/dev/null 2>&1; then
    # A system rustup installation can still locate the active toolchain even
    # when its cargo proxy is not present in PATH.
    CODEX_INFO_CARGO="$(rustup which cargo 2>/dev/null || true)"
fi

if [[ -z "$CODEX_INFO_CARGO" || ! -x "$CODEX_INFO_CARGO" ]]; then
    # Keep launcher diagnostics language-neutral; all user-facing product copy
    # (including CLI help) is owned by the Rust i18n catalog.
    echo "run.sh: E_CARGO_NOT_FOUND" >&2
    exit 127
fi

exec "$CODEX_INFO_CARGO" run --manifest-path "$BASE_DIR/Cargo.toml" --release --locked -- "$@"

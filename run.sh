#!/usr/bin/env bash
# Copyright (C) 2026 salty919
# SPDX-License-Identifier: GPL-3.0-only

set -euo pipefail

BASE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# WSLg exposes both Wayland and X11. Force X11 so winit does not select a
# broken Wayland/EGL path on installations where the GPU driver is unavailable.
export WINIT_UNIX_BACKEND="x11"
export WINIT_X11_SCALE_FACTOR="1"
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
    echo "run.sh: cargo が見つかりません。Rust/Cargoをインストールするか、PATHを設定してください。" >&2
    echo "例: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
    echo "インストール後に source \"\$HOME/.cargo/env\" を実行してから再試行してください。" >&2
    exit 127
fi

exec "$CODEX_INFO_CARGO" run --manifest-path "$BASE_DIR/Cargo.toml" --release --locked

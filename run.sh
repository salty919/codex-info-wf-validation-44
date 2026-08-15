#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# WSLg exposes both Wayland and X11. Force X11 so winit does not select a
# broken Wayland/EGL path on installations where the GPU driver is unavailable.
export WINIT_UNIX_BACKEND="x11"
export WINIT_X11_SCALE_FACTOR="1"
export LIBGL_ALWAYS_SOFTWARE="1"
export MESA_LOADER_DRIVER_OVERRIDE="llvmpipe"

exec cargo run --manifest-path "$BASE_DIR/Cargo.toml" --release --locked

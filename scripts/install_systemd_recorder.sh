#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$ROOT_DIR/target/release/codex_info"
if [[ "${1:-}" == "--binary" ]]; then
    [[ $# -eq 2 ]] || { echo 'usage: install_systemd_recorder.sh [--binary /absolute/path]' >&2; exit 2; }
    binary="$2"
fi

[[ "$binary" = /* ]] || { echo 'binary must be an absolute path' >&2; exit 2; }
[[ -f "$binary" && -x "$binary" ]] || { echo "release binary is not executable: $binary" >&2; exit 1; }
command -v systemctl >/dev/null 2>&1 || { echo 'systemctl is required' >&2; exit 1; }
systemctl --user show-environment >/dev/null 2>&1 || {
    echo 'systemd user manager is unavailable' >&2
    exit 1
}

local_bin="$HOME/.local/bin"
unit_dir="$HOME/.config/systemd/user"
mkdir -p "$local_bin" "$unit_dir"
install -m 0755 "$binary" "$local_bin/codex_info"
install -m 0755 "$ROOT_DIR/packaging/codex-info-recorder-cleanup" "$local_bin/codex-info-recorder-cleanup"
install -m 0644 "$ROOT_DIR/packaging/codex-info-recorder.service" "$unit_dir/codex-info-recorder.service"

systemctl --user daemon-reload
systemctl --user enable --now codex-info-recorder.service
systemctl --user is-active --quiet codex-info-recorder.service
echo "installed and active: codex-info-recorder.service"

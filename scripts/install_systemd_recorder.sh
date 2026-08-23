#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$ROOT_DIR/target/release/codex_info"
action="install"
if [[ "${1:-}" == "--remove" ]]; then
    [[ $# -eq 1 ]] || { echo 'usage: install_systemd_recorder.sh [--remove|--binary /absolute/path]' >&2; exit 2; }
    action="remove"
elif [[ "${1:-}" == "--binary" ]]; then
    [[ $# -eq 2 ]] || { echo 'usage: install_systemd_recorder.sh [--remove|--binary /absolute/path]' >&2; exit 2; }
    binary="$2"
fi

command -v systemctl >/dev/null 2>&1 || { echo 'systemctl is required' >&2; exit 1; }
systemctl --user show-environment >/dev/null 2>&1 || {
    echo 'systemd user manager is unavailable' >&2
    exit 1
}

unit_dir="$HOME/.config/systemd/user"
if [[ "$action" == "remove" ]]; then
    # Autostart removal deliberately preserves the executable, SQLite history,
    # backups, reset hint and source JSONL.
    systemctl --user disable --now codex-info.service 2>/dev/null || true
    systemctl --user disable --now codex-info-recorder.service 2>/dev/null || true
    systemctl --user disable --now codex-info-api.service 2>/dev/null || true
    rm -f -- \
        "$unit_dir/codex-info.service" \
        "$unit_dir/codex-info-recorder.service" \
        "$unit_dir/codex-info-api.service"
    systemctl --user daemon-reload
    systemctl --user reset-failed codex-info.service 2>/dev/null || true
    echo "removed autostart: codex-info.service (history and executable preserved)"
    exit 0
fi

[[ "$binary" = /* ]] || { echo 'binary must be an absolute path' >&2; exit 2; }
[[ -f "$binary" && -x "$binary" ]] || { echo "release binary is not executable: $binary" >&2; exit 1; }

local_bin="$HOME/.local/bin"
mkdir -p "$local_bin" "$unit_dir"
install -m 0755 "$binary" "$local_bin/codex_info"
install -m 0644 "$ROOT_DIR/packaging/codex-info.service" "$unit_dir/codex-info.service"

systemctl --user disable --now codex-info-recorder.service 2>/dev/null || true
systemctl --user disable --now codex-info-api.service 2>/dev/null || true
rm -f -- "$unit_dir/codex-info-recorder.service" "$unit_dir/codex-info-api.service"
systemctl --user daemon-reload
systemctl --user enable codex-info.service
# `enable --now` leaves an already-running old generation untouched. Always
# restart after the executable/unit replacement so the installed SHA is live.
systemctl --user restart codex-info.service
systemctl --user is-active --quiet codex-info.service
echo "installed and active: codex-info.service"

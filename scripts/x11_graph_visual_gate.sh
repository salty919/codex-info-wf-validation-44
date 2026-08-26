#!/usr/bin/env bash
set -euo pipefail

# This is a local X11 visual acceptance probe for the native client. It uses
# only an isolated preview fixture and never touches the user's database or
# running client. A missing display/tool is HOLD, never PASS.
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

hold() { echo "x11-graph-visual-gate: HOLD: $*" >&2; exit 2; }
fail() { echo "x11-graph-visual-gate: FAIL: $*" >&2; exit 1; }

[[ -n "${DISPLAY:-}" ]] || hold 'DISPLAY is unavailable; X11 image was not rendered'
for command in xwininfo xprop xwd python3; do
    command -v "$command" >/dev/null || hold "$command is unavailable; X11 image was not rendered"
done
BINARY="$ROOT_DIR/target/release/codex_info"
[[ -x "$BINARY" ]] || fail 'build target/release/codex_info first'

temp_parent="${TMPDIR:-/tmp}"
temp_root="$(mktemp -d "$temp_parent/codex-info-x11-graph.XXXXXX")"
preview_pid=""
cleanup() {
    if [[ -n "$preview_pid" ]] && kill -0 "$preview_pid" 2>/dev/null; then
        kill "$preview_pid" 2>/dev/null || true
        wait "$preview_pid" 2>/dev/null || true
    fi
    case "$temp_root" in
        "$temp_parent"/codex-info-x11-graph.*) rm -rf -- "$temp_root" ;;
        *) echo 'x11-graph-visual-gate: refusing to clean an unexpected path' >&2 ;;
    esac
}
trap cleanup EXIT
mkdir -p "$temp_root"/{home,config,data,cache,state,runtime}
chmod 700 "$temp_root/runtime"

env HOME="$temp_root/home" \
    XDG_CONFIG_HOME="$temp_root/config" \
    XDG_DATA_HOME="$temp_root/data" \
    XDG_CACHE_HOME="$temp_root/cache" \
    XDG_STATE_HOME="$temp_root/state" \
    XDG_RUNTIME_DIR="$temp_root/runtime" \
    CODEX_INFO_PREVIEW=graph-collision \
    CODEX_INFO_PREVIEW_SIZE=940x640 \
    "$BINARY" --ui-only >"$temp_root/client.log" 2>&1 &
preview_pid="$!"

graph_id=""
main_id=""
for _ in $(seq 1 80); do
    while read -r window_id; do
        [[ -n "$window_id" ]] || continue
        pid_line="$(xprop -id "$window_id" _NET_WM_PID 2>/dev/null || true)"
        window_pid="$(awk -F'= ' '{print $2}' <<<"$pid_line" | tr -d '[:space:]')"
        [[ "$window_pid" == "$preview_pid" ]] || continue
        name_line="$(xprop -id "$window_id" WM_NAME 2>/dev/null || true)"
        if [[ "$name_line" == *Graph* ]]; then
            graph_id="$window_id"
        else
            main_id="$window_id"
        fi
    done < <(xwininfo -root -tree 2>/dev/null | awk '/^ +0x[0-9a-f]+/ {print $1}')
    [[ -n "$graph_id" ]] && break
    sleep 0.125
done
[[ -n "$graph_id" ]] || {
    sed -n '1,120p' "$temp_root/client.log" >&2 || true
    fail 'graph preview window did not render'
}

# Keep the graph and its main owner from overlapping on multi-monitor X11
# sessions. This is test-only window arrangement; the product never moves a
# user's cursor or other application window.
python3 - "$graph_id" "$main_id" <<'PY'
import ctypes, sys
lib = ctypes.CDLL('libX11.so.6')
lib.XOpenDisplay.argtypes = [ctypes.c_char_p]
lib.XOpenDisplay.restype = ctypes.c_void_p
lib.XMoveWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_int]
lib.XRaiseWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
lib.XFlush.argtypes = [ctypes.c_void_p]
display = lib.XOpenDisplay(None)
if not display:
    raise SystemExit('XOpenDisplay failed')
graph = int(sys.argv[1], 16)
main = int(sys.argv[2], 16) if len(sys.argv) > 2 and sys.argv[2] else 0
if main:
    lib.XMoveWindow(display, main, 1000, 100)
lib.XMoveWindow(display, graph, 3000, 100)
lib.XRaiseWindow(display, graph)
lib.XFlush(display)
PY
sleep 0.5
xwd -silent -id "$graph_id" -out "$temp_root/graph.xwd"

python3 - "$temp_root/graph.xwd" <<'PY'
import struct, sys
from math import sqrt

path = sys.argv[1]
data = open(path, 'rb').read()
if len(data) < 100:
    raise SystemExit('XWD header is truncated')
h = struct.unpack('>25I', data[:100])
header_size, width, height, bytes_per_line, colors = h[0], h[4], h[5], h[12], h[19]
if (width, height) != (940, 640):
    raise SystemExit(f'unexpected graph image size: {width}x{height}')
offset = header_size + colors * 12
if offset + bytes_per_line * height > len(data):
    raise SystemExit('XWD pixel payload is truncated')
stride = bytes_per_line // width
if stride not in (3, 4):
    raise SystemExit(f'unsupported XWD pixel stride: {stride}')

def rgb(x, y):
    line = offset + y * bytes_per_line
    i = line + x * stride
    # X11 capture is BGR or BGRX on the supported visuals.
    return data[i + 2], data[i + 1], data[i]

def near(actual, expected, tolerance=15):
    return sqrt(sum((actual[i] - expected[i]) ** 2 for i in range(3))) <= tolerance

plot = [(x, y) for y in range(230, 590) for x in range(90, 830)]
idle_band = [(x, y) for x, y in plot if near(rgb(x, y), (27, 41, 61), 4)]
if len(idle_band) < 1000:
    raise SystemExit(f'dedicated idle-band pixels are missing: {len(idle_band)}')
if max(x for x, _ in idle_band) - min(x for x, _ in idle_band) < 200:
    raise SystemExit('dedicated idle band does not span an observed quiet interval')

remaining = [(x, y) for x, y in plot if near(rgb(x, y), (86, 178, 245))]
if len(remaining) < 300:
    raise SystemExit(f'remaining line pixels are insufficient: {len(remaining)}')
if max(x for x, _ in remaining) - min(x for x, _ in remaining) < 500:
    raise SystemExit('remaining line does not span the plot')
ys = [y for _, y in remaining]
if max(ys) - min(ys) > 25 or max(ys) >= 400:
    raise SystemExit('remaining line contains an unexplained vertical quota drop')

for name, expected in (
    ('SOL', (168, 140, 245)),
    ('TERRA', (93, 201, 138)),
    ('LUNA', (230, 162, 60)),
):
    count = sum(near(rgb(x, y), expected) for x, y in plot)
    if count < 3:
        raise SystemExit(f'{name} model line pixels are missing: {count}')

print('x11-graph-visual-gate: PASS (940x640 image, remaining 88->87 without 14% drop, idle band, SOL/TERRA/LUNA pixels present)')
PY

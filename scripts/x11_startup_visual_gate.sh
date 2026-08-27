#!/usr/bin/env bash
set -euo pipefail
root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"
hold() { echo "x11-startup-visual-gate: HOLD: $*" >&2; exit 2; }
fail() { echo "x11-startup-visual-gate: FAIL: $*" >&2; exit 1; }
[[ -n "${DISPLAY:-}" ]] || hold 'DISPLAY is unavailable'
for command in xwininfo xprop xwd python3; do command -v "$command" >/dev/null || hold "$command is unavailable"; done
binary="$root_dir/target/release/codex_info"
[[ -x "$binary" ]] || fail 'build target/release/codex_info first'
temp_parent="${TMPDIR:-/tmp}"
temp_root="$(mktemp -d "$temp_parent/codex-info-x11-startup.XXXXXX")"
preview_pid=""
cleanup() {
    if [[ -n "$preview_pid" ]] && kill -0 "$preview_pid" 2>/dev/null; then kill "$preview_pid" 2>/dev/null || true; wait "$preview_pid" 2>/dev/null || true; fi
    case "$temp_root" in
        "$temp_parent"/codex-info-x11-startup.*) rm -rf -- "$temp_root" ;;
        *) echo 'x11-startup-visual-gate: refusing unexpected cleanup' >&2 ;;
    esac
}
trap cleanup EXIT
mkdir -p "$temp_root"/{home,config,data,cache,state,runtime}
chmod 700 "$temp_root/runtime"
env HOME="$temp_root/home" XDG_CONFIG_HOME="$temp_root/config" XDG_DATA_HOME="$temp_root/data" XDG_CACHE_HOME="$temp_root/cache" XDG_STATE_HOME="$temp_root/state" XDG_RUNTIME_DIR="$temp_root/runtime" CODEX_INFO_PREVIEW=startup-loading CODEX_INFO_PREVIEW_SIZE=900x480 "$binary" --ui >"$temp_root/client.log" 2>&1 &
preview_pid="$!"
window_id=""
for _ in $(seq 1 80); do
    while read -r candidate; do
        window_pid="$(xprop -id "$candidate" _NET_WM_PID 2>/dev/null | awk -F'= ' '{print $2}' | tr -d '[:space:]')"
        if [[ "$window_pid" == "$preview_pid" ]]; then window_id="$candidate"; break; fi
    done < <(xwininfo -root -tree 2>/dev/null | awk '/^ +0x[0-9a-f]+/ {print $1}')
    [[ -n "$window_id" ]] && break
    sleep 0.125
done
[[ -n "$window_id" ]] || { sed -n '1,120p' "$temp_root/client.log" >&2 || true; fail 'startup preview window did not render'; }
root_geometry="$(xwininfo -root 2>/dev/null)"
window_geometry="$(xwininfo -id "$window_id" 2>/dev/null)"
root_width="$(awk '/Width:/ {print $2; exit}' <<<"$root_geometry")"
root_height="$(awk '/Height:/ {print $2; exit}' <<<"$root_geometry")"
window_x="$(awk '/Absolute upper-left X:/ {print $4; exit}' <<<"$window_geometry")"
window_y="$(awk '/Absolute upper-left Y:/ {print $4; exit}' <<<"$window_geometry")"
window_width="$(awk '/Width:/ {print $2; exit}' <<<"$window_geometry")"
window_height="$(awk '/Height:/ {print $2; exit}' <<<"$window_geometry")"
[[ "$root_width" =~ ^[0-9]+$ && "$root_height" =~ ^[0-9]+$ && "$window_x" =~ ^-?[0-9]+$ && "$window_y" =~ ^-?[0-9]+$ && "$window_width" =~ ^[0-9]+$ && "$window_height" =~ ^[0-9]+$ ]] || fail 'window geometry could not be read'
(( window_x >= 0 && window_y >= 0 && window_x + window_width <= root_width && window_y + window_height <= root_height )) ||
    fail "startup window is outside the visible X11 desktop: ${window_x},${window_y} ${window_width}x${window_height} on ${root_width}x${root_height}"
xwd -silent -id "$window_id" -out "$temp_root/startup.xwd"
python3 - "$temp_root/startup.xwd" <<'PY'
import struct, sys
from math import sqrt
data = open(sys.argv[1], 'rb').read()
h = struct.unpack('>25I', data[:100])
header_size, width, height, bytes_per_line, colors = h[0], h[4], h[5], h[12], h[19]
if (width, height) != (900, 480):
    raise SystemExit(f'unexpected startup image size: {width}x{height}')
offset = header_size + colors * 12
stride = bytes_per_line // width
def rgb(x, y):
    i = offset + y * bytes_per_line + x * stride
    return data[i + 2], data[i + 1], data[i]
def near(a, b, tolerance=20):
    return sqrt(sum((a[i] - b[i]) ** 2 for i in range(3))) <= tolerance
header_pixels = [(x, y) for y in range(14, 48) for x in range(12, 420) if min(rgb(x, y)) > 150]
if len(header_pixels) < 80:
    raise SystemExit(f'header/version pixels missing: {len(header_pixels)}')
canvas = rgb(10, 100)
center_pixels = [(x, y) for y in range(190, 300) for x in range(330, 570) if not near(rgb(x, y), canvas, 8)]
if len(center_pixels) < 20:
    raise SystemExit(f'center spinner/status missing: {len(center_pixels)}')
# The chrome and status text legitimately contain a few quota-blue pixels.  A
# rendered payload, however, contains a connected horizontal gauge/series in
# the content area.  Detect that observable shape instead of rejecting
# harmless antialiasing noise (which made this gate flaky across X11 servers).
quota_blue = {(x, y) for y in range(80, height) for x in range(width) if near(rgb(x, y), (86, 178, 245), 16)}
seen = set()
largest = (0, 0, 0, 0)  # area, width, height, pixels
for start in quota_blue:
    if start in seen:
        continue
    stack = [start]
    seen.add(start)
    component = []
    while stack:
        x, y = stack.pop()
        component.append((x, y))
        for nx in range(x - 1, x + 2):
            for ny in range(y - 1, y + 2):
                point = (nx, ny)
                if point in quota_blue and point not in seen:
                    seen.add(point)
                    stack.append(point)
    xs = [x for x, _ in component]
    ys = [y for _, y in component]
    shape = (len(component), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1, len(component))
    if shape[:3] > largest[:3]:
        largest = shape
if largest[1] >= 100 and largest[0] >= 100:
    raise SystemExit(f'partial quota payload leaked: component area={largest[0]} width={largest[1]} height={largest[2]}')
print('x11-startup-visual-gate: PASS (900x480, header/version visible, centered spinner visible, partial payload hidden)')
PY

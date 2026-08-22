#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail() { echo "regression-guard: FAIL: $*" >&2; exit 1; }
require_text() { rg -q --fixed-strings -- "$2" "$1" || fail "missing contract $1: $2"; }

require_text docs/REGRESSION_PREVENTION_POLICY.md 'REG-WIN-DRAG'
require_text docs/REGRESSION_PREVENTION_POLICY.md 'REG-GRAPH-END'
require_text docs/REGRESSION_PREVENTION_POLICY.md 'REG-SETUP-ONCE'
require_text docs/REGRESSION_PREVENTION_POLICY.md 'REG-NO-MOUSE-STEAL'
require_text windows-client/src/CodexInfo.WindowsClient/WindowDragBehavior.cs 'window.BeginMoveDrag(eventArgs)'
require_text windows-client/src/CodexInfo.WindowsClient/WindowDragBehavior.cs 'public static void Attach'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/DetailsWindowViewModels.cs 'EffectiveGraphEnd'
require_text windows-client/src/CodexInfo.WindowsClient/Controls/GraphPlotControl.cs 'unobservedStart'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ClientSettings.cs 'ConnectionConfigured'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml.cs 'ShouldOpenSetup'
if rg -q 'SetCursorPos|mouse_event|SendInput' windows-client/src; then
    fail 'product source contains physical cursor or synthetic mouse API'
fi
if [[ ! -f docs/INDEPENDENT_AUDIT_LATEST.md ]] || ! rg -q '^status:[[:space:]]*PASS[[:space:]]*$' docs/INDEPENDENT_AUDIT_LATEST.md; then
    fail 'latest independent subagent audit is not PASS'
fi
audit_sha="$(awk '/^artifact_sha256:[[:space:]]*/ {print $2; exit}' docs/INDEPENDENT_AUDIT_LATEST.md)"
[[ -n "$audit_sha" ]] || fail 'independent audit artifact SHA is missing'
rg -q --fixed-strings -- "$audit_sha" docs/AGENT_REQUIREMENTS_TRACKER.md || fail 'subagent tracker SHA does not match independent audit artifact'

echo 'regression-guard: PASS (fixed contracts and independent audit are present)'

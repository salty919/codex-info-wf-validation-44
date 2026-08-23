#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail() { echo "regression-guard: FAIL: $*" >&2; exit 1; }
require_text() { rg -q --fixed-strings -- "$2" "$1" || fail "missing $1: $2"; }

require_text docs/PRODUCT_REQUIREMENTS.md '全直積、N倍、N二乗、N階乗のcase生成を行わない'
require_text docs/REGRESSION_PREVENTION_POLICY.md 'REG-WIN-DRAG'
require_text windows-client/src/CodexInfo.WindowsClient/WindowDragBehavior.cs 'window.BeginMoveDrag(eventArgs)'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/DetailsWindowViewModels.cs 'EffectiveGraphEnd'
require_text windows-client/src/CodexInfo.WindowsClient/Graphing/GraphPlotProjection.cs 'IsSyntheticFirstObservation'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/GraphPlotControlTests.cs 'PlotProjectionStartsAtFirstObservationWithoutSyntheticVerticalJump'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ClientSettings.cs 'ConnectionConfigured'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SetupFlow.cs 'SetupLaunchPolicy'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/WindowDragGeometryTests.cs 'SetupLaunchPolicy.ShouldOpen'

if rg -q 'SetCursorPos|mouse_event|SendInput' windows-client/src; then
    fail 'product source contains physical cursor or synthetic mouse API'
fi

echo 'regression-guard: PASS'

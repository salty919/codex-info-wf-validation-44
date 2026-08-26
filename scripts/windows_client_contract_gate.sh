#!/usr/bin/env bash
# These single-quoted patterns intentionally match literal PowerShell '$'
# variables in workflow/source fixtures; they must not be shell-expanded.
# shellcheck disable=SC2016
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() { echo "windows-client-contract-gate: $*" >&2; exit 1; }
require_file() {
    [[ -f "$1" ]] || fail "missing file: $1"
}
require_text() {
    local file="$1" pattern="$2"
    rg -q --fixed-strings -- "$pattern" "$file" || fail "missing: $file: $pattern"
}
require_window_text() {
    local file="$1" pattern="$2"
    if ! sed -n '/<Window /,/Background=/p' "$file" | rg -q --fixed-strings -- "$pattern"; then
        fail "missing top-level Window geometry: $file: $pattern"
    fi
}

require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenGraph"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenThreads"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenLegal"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenSettings"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.QuotaPeriodGauge"'
require_window_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Width="900"'
require_window_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Height="480"'
require_window_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'Width="900"'
require_window_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'Height="480"'
require_window_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'Width="900"'
require_window_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'Height="480"'
for fixed_window in MainWindow SetupWindow SettingsWindow ThreadsWindow LegalNoticesWindow; do
    fixed_path="windows-client/src/CodexInfo.WindowsClient/${fixed_window}.axaml"
    require_window_text "$fixed_path" 'Width="900"'
    require_window_text "$fixed_path" 'Height="480"'
    require_window_text "$fixed_path" 'MinWidth="900"'
    require_window_text "$fixed_path" 'MinHeight="480"'
    require_window_text "$fixed_path" 'MaxWidth="900"'
    require_window_text "$fixed_path" 'MaxHeight="480"'
    require_window_text "$fixed_path" 'CanResize="False"'
done
require_window_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'Width="940"'
require_window_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'Height="640"'
require_window_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'MinWidth="700"'
require_window_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'MinHeight="480"'
require_window_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'CanResize="True"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Classes="quota-segment"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowRemaining}"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowSol}"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowTerra}"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowLuna}"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'AutomationProperties.AutomationId="Graph.PeriodMenu"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'AutomationProperties.AutomationId="Graph.MetricMenu"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsCheckedChanged="OnPeriodSelectorCheckedChanged"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsCheckedChanged="OnMetricSelectorCheckedChanged"'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/DetailsWindowViewModels.cs 'private IReadOnlyList<string> metricOptions'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml.cs 'periodSelectionAtOpen'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml.cs 'metricSelectionAtOpen'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/DetailsWindowViewModels.cs 'MaxRenderedGraphPoints = 2_048'
require_text windows-client/src/CodexInfo.WindowsClient/ThreadsWindow.axaml 'ThreadTreeControl'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'CurrentNoticeText'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'ScrollViewer'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'Command="{Binding BackCommand}"'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'Command="{Binding NextCommand}"'
require_file windows-client/src/CodexInfo.WindowsClient/LegalNoticeCatalog.cs
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticeCatalog.cs 'Legal/LICENSE'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticeCatalog.cs 'Legal/THIRD_PARTY_NOTICES.md'
require_text windows-client/src/CodexInfo.WindowsClient/CodexInfo.WindowsClient.csproj 'Link="Legal\LICENSE"'
require_text windows-client/src/CodexInfo.WindowsClient/CodexInfo.WindowsClient.csproj 'Link="Legal\LICENSES\OFL-1.1.txt"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshCommand'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshHost'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshConfigAliases'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'OnStartOrStopSsh'
require_file windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsSetupConnectionEnvironment.cs
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsSetupConnectionEnvironment.cs 'FileName = "ssh.exe"'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsSetupConnectionEnvironment.cs 'ArgumentList.Add("-L")'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsSetupConnectionEnvironment.cs 'UseShellExecute = false'
require_file windows-client/src/CodexInfo.WindowsClient.Core/HealthContracts.cs
require_text windows-client/src/CodexInfo.WindowsClient.Core/HealthContracts.cs 'interface ILoopbackHealthClient'
require_text windows-client/src/CodexInfo.WindowsClient.Core/LoopbackStatusClient.cs 'HealthEndpoint = "http://127.0.0.1:8787/v1/health"'
require_text windows-client/src/CodexInfo.WindowsClient.Core/LoopbackStatusClient.cs 'response.StatusCode != HttpStatusCode.OK'
status_200_count="$(rg -o --fixed-strings 'response.StatusCode != HttpStatusCode.OK' windows-client/src/CodexInfo.WindowsClient.Core/LoopbackStatusClient.cs | wc -l)"
if [[ "$status_200_count" -ne 3 ]]; then
    fail 'health, status, and details must each require HTTP 200'
fi
require_text windows-client/src/CodexInfo.WindowsClient.Core/LoopbackStatusClient.cs 'contentLength is not long declaredLength'
require_text windows-client/src/CodexInfo.WindowsClient.Core/LoopbackStatusClient.cs 'bodyStatus.Body.LongLength != declaredLength'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs 'FetchHealthAsync(cancellationToken)'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ClientSettings.cs 'seen.SetEquals(expected)'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsUpdateCoordinator.cs 'FileShare.None'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsUpdateCoordinator.cs 'FileOptions.DeleteOnClose'
require_file windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsPathSafety.cs
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsPathSafety.cs 'EnsureDirectoryTreeWithoutReparse'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsPathSafety.cs 'ContainsReparsePoint'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsUpdateCoordinator.cs 'WindowsPathSafety.ContainsReparsePoint'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ClientSettings.cs 'WindowsPathSafety.EnsureDirectoryTreeWithoutReparse'
require_text docs/WINDOWS_CLIENT.md '%USERPROFILE%\.ssh\config'
require_text docs/PRODUCT_REQUIREMENTS.md '製品バージョンはメイン画面に一度だけ表示し'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'AutomationProperties.Name="{Binding Texts.Copy}"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'Background="Transparent" PointerPressed="OnTitlePointerPressed"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml.cs 'WindowDragBehavior.Begin(this, e)'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'IsEnabled="{Binding CanContinue}"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'Click="OnClose"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml.cs 'PreviewEnvironment.IsChild("settings")'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Background="Transparent"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'PointerPressed="OnTitlePointerPressed"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml.cs 'WindowDragBehavior.Begin('
for window in ThreadsWindow LegalNoticesWindow SettingsWindow; do
    require_text "windows-client/src/CodexInfo.WindowsClient/${window}.axaml" 'Background="Transparent" PointerPressed="OnTitlePointerPressed"'
    require_text "windows-client/src/CodexInfo.WindowsClient/${window}.axaml.cs" 'WindowDragBehavior.Begin('
done
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml.cs 'RoutingStrategies.Tunnel'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml.cs 'IsInteractiveSource(eventArgs.Source)'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml.cs 'WindowDragBehavior.Begin(this, eventArgs)'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs '1 => main.IsAuthenticated'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml.cs 'viewModel.Advance() == SetupAdvanceOutcome.CloseRequested'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs 'if (IsAuthStep && !CanContinue)'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml.cs 'SetupLaunchPolicy.ShouldOpen(App.CurrentSettings)'
require_file windows-client/src/CodexInfo.WindowsClient.Core/WindowsUpdateClient.cs
require_file windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsUpdateCoordinator.cs
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'IsVisible="{Binding IsUpdateNotificationVisible}"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Command="{Binding UpdateCommand}"'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs 'public bool IsUpdateNotificationVisible => !IsAuthRequired'
require_text windows-client/src/CodexInfo.WindowsClient.Core/WindowsUpdateClient.cs 'https://api.github.com/repos/salty919/codex_info_v2/releases?per_page=20'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsUpdateCoordinator.cs 'StartAvailableUpdateAsync'
require_text windows-client/Directory.Build.props '<Version>'
require_file windows-client/src/CodexInfo.WindowsClient.Core/ProductInfo.cs
require_text windows-client/src/CodexInfo.WindowsClient.Core/ProductInfo.cs 'public static string DisplayVersion'
require_text windows-client/src/CodexInfo.WindowsClient.Core/ProductInfo.cs 'Assembly.GetName().Version'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'ProductVersionText'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.ProductVersion"'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ConnectionProcessFactory.cs 'codex_info'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ConnectionProcessFactory.cs '--service'
main_version_marker_count="$(rg -o --fixed-strings 'AutomationProperties.AutomationId="Main.ProductVersion"' windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml | wc -l)"
[[ "$main_version_marker_count" -eq 1 ]] ||
    fail "main version automation marker must appear exactly once: count=$main_version_marker_count"
main_version_binding_count="$(rg -o --fixed-strings 'Text="{Binding ProductVersionText}"' windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml | wc -l)"
[[ "$main_version_binding_count" -eq 1 ]] ||
    fail "main version binding must appear exactly once: count=$main_version_binding_count"
for child_window in SetupWindow SettingsWindow GraphWindow ThreadsWindow LegalNoticesWindow; do
    child_path="windows-client/src/CodexInfo.WindowsClient/${child_window}.axaml"
    if rg -q --fixed-strings 'ProductVersion' "$child_path"; then
        fail "child window must not render a product version: $child_path"
    fi
done
for redundant_version_marker in \
    'Setup.ProductVersion' \
    'Settings.ProductVersion' \
    'Graph.ProductVersion' \
    'Threads.ProductVersion' \
    'Legal.ProductVersion'; do
    if rg -q --fixed-strings -- "$redundant_version_marker" windows-client/src/CodexInfo.WindowsClient; then
        fail "redundant child-window version marker remains: $redundant_version_marker"
    fi
done
native_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
windows_version="$(sed -n 's/.*<Version>\([^<]*\)<\/Version>.*/\1/p' windows-client/Directory.Build.props | head -n 1)"
[[ -n "$native_version" && "$native_version" == "$windows_version" ]] ||
    fail "native and Windows versions differ: native=$native_version windows=$windows_version"
require_text ui/components.slint 'product-version: string'
require_text ui/components.slint 'root.strings.usage-status + " · " + root.strings.product-version'
if rg -q --fixed-strings -- 'root.strings.usage-trend + " · " + root.strings.product-version' ui/components.slint ||
   rg -q --fixed-strings -- 'root.strings.active-threads + " · " + root.strings.product-version' ui/components.slint ||
   rg -q --fixed-strings -- 'root.strings.legal-notices + " · " + root.strings.product-version' ui/components.slint; then
    fail 'redundant native child-window version title remains'
fi
require_text ui/components.slint 'legal-page-names: [string]'
require_text ui/components.slint 'legal-pages: [string]'
require_text ui/components.slint 'legal-protocol: string'
require_text ui/components.slint 'legal-third-party: string'
require_text ui/components.slint 'callback legal-page-back();'
require_text ui/components.slint 'callback legal-page-next();'
require_text ui/components.slint 'root.strings.legal-pages[root.legal-page-index]'
require_text src/main.rs 'const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");'
require_text src/main.rs 'product_version: format!("v{PRODUCT_VERSION}").into()'
require_text src/main.rs 'const LEGAL_PAGE_CHUNK_SCALARS: usize = 620;'
require_text src/main.rs 'include_str!("../LICENSE")'
require_text src/main.rs 'include_str!("../THIRD_PARTY_NOTICES.md")'
require_text src/main.rs 'i18n.text(TextKey::LegalProtocol)'
require_text src/main.rs 'i18n.text(TextKey::LegalThirdParty)'
require_text .github/workflows/windows-client.yml 'needs: [version-policy, core-tests, windows-build, acceptance]'
require_text .github/workflows/windows-client.yml 'Run final acceptance gate before merge'
require_text .github/workflows/windows-client.yml 'windows_window_move_smoke.ps1 -ClientPath $exe -AllowPhysicalInput'
require_text .github/workflows/windows-client.yml "--logger 'trx;LogFilePrefix=windows-client'"
require_text .github/workflows/windows-client.yml 'Expected exactly two Windows TRX reports'
require_text .github/workflows/windows-client.yml 'TRX counters are missing'
require_text .github/workflows/windows-client.yml 'Windows test counts are not release-safe:'
require_text .github/workflows/windows-client.yml 'bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e'
require_text .github/workflows/windows-client.yml '-SourceSha $env:GITHUB_SHA'
require_text .github/workflows/windows-client.yml 'EXPECTED_E2E_SOURCE_SHA: ${{ github.sha }}'
require_file scripts/final_acceptance_gate.sh
require_text scripts/final_acceptance_gate.sh 'expected E2E source SHA is required'
require_text scripts/final_acceptance_gate.sh 'source tree is dirty; release evidence must match a clean committed revision'
require_text scripts/final_acceptance_gate.sh 'source-sha: $expected_sha'
require_text scripts/final_acceptance_gate.sh 'capture: name=$capture_name '
require_text scripts/final_acceptance_gate.sh 'sha256sum "$capture_path"'
require_text scripts/final_acceptance_gate.sh 'window-move-smoke: PASS'
require_text .github/workflows/windows-client.yml 'cancel-in-progress: false'
for native_trigger in 'Cargo.toml' 'Cargo.lock' 'build.rs' 'run.sh' 'src/**' 'protocol/**' 'tests/**' 'ui/**' 'assets/**' 'LICENSE' 'LICENSE.ja.md' 'deny.toml' '.cargo/config.toml' 'scripts/**' 'docs/**'; do
    require_text .github/workflows/windows-client.yml "      - \"$native_trigger\""
done
require_text .github/workflows/windows-client.yml 'dtolnay/rust-toolchain@stable'
final_gate_line="$(rg -n 'bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e' .github/workflows/windows-client.yml | cut -d: -f1 | tail -n 1)"
[[ -n "$final_gate_line" ]] || fail 'final acceptance gate invocation is missing'
if ! sed -n "$((final_gate_line - 4)),${final_gate_line}p" .github/workflows/windows-client.yml |
    rg -q --fixed-strings 'dtolnay/rust-toolchain@stable'; then
    fail 'final acceptance gate must install the pinned Rust toolchain before running'
fi
require_text scripts/regression_guard.sh 'cargo check --locked --all-targets'
require_text scripts/regression_guard.sh 'cargo test --locked --all-targets'
require_text scripts/regression_guard.sh 'cargo build --release --locked'
require_text scripts/regression_guard.sh '--exact --nocapture'
require_text scripts/regression_guard.sh 'Rust all-target test set contains a zero-test target'
require_text scripts/regression_guard.sh 'X11 graph visual gate unverified (DISPLAY unavailable)'
require_text .github/workflows/rust.yml 'xvfb-run --auto-servernum'
require_text .github/workflows/windows-client.yml 'xvfb-run --auto-servernum'
for required_history_test in \
    historical_week_fixture_preserves_each_period_and_graph_samples \
    observed_moving_reset_sequence_keeps_the_spend_in_the_selected_graph \
    long_rolling_reset_sequence_stays_in_one_period_after_a_real_boundary \
    quota_only_reset_fragments_stay_with_the_adjacent_spend_period \
    live_rolling_quota_chain_does_not_expose_an_empty_past_period \
    affected_period_keeps_sol_spend_and_unobserved_quota_distinct \
    shared_graph_fixture_is_the_x_history_oracle \
    model_graph_does_not_invent_spend_during_an_unobserved_gap \
    unused_intervals_mark_long_gap_before_observed_spend \
    graph_controls_use_one_visual_boundary_and_show_short_histories \
    remaining_graph_does_not_infer_quota_loss_from_model_spend \
    affected_timestamp_does_not_mix_a_singleton_reset_period_into_history \
    ambiguous_missing_quota_row_at_a_spend_timestamp_is_not_a_period \
    singleton_reset_snapshot_overlapping_a_spend_period_stays_separate \
    graph_collision_preview_matches_the_historical_singleton_oracle \
    moving_reset_collision_at_30_and_60_seconds_fails_closed \
    record_rejects_alias_quota_collision_before_canonical_merge \
    same_timestamp_reset_drift_above_jitter_fails_closed \
    startup_load_sanitizes_legacy_same_timestamp_quota_collision \
    periodic_quota_refresh_retains_last_good_main_snapshot \
    product_version_is_visible_once_on_native_main_surface; do
    require_text scripts/regression_guard.sh "run_required_rust_test $required_history_test"
done
require_text .github/workflows/windows-client.yml 'Get-GitHubResourceStatus'
require_text .github/workflows/windows-client.yml 'gh api --method POST "repos/$repository/git/refs"'
require_text .github/workflows/windows-client.yml 'gh api --method POST "repos/$repository/releases"'
require_text .github/workflows/windows-client.yml '$draftReleaseEndpoint = "repos/$repository/releases/$($createdRelease.id)"'
require_text .github/workflows/windows-client.yml 'gh release upload $tag $setup $manifest'
require_text .github/workflows/windows-client.yml '-F draft=false'
require_file windows-client/src/CodexInfo.WindowsClient/Settings/ClientSettingsSession.cs
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ClientSettingsSession.cs 'SettingsCorrupt = false'
require_file windows-client/src/CodexInfo.WindowsClient/Graphing/GraphPlotProjection.cs
require_text src/main.rs 'const MODEL_CONTIGUOUS_SAMPLE_MAX_GAP_SECONDS: i64 = 60;'
require_text windows-client/src/CodexInfo.WindowsClient/Graphing/GraphPlotProjection.cs 'private const long ModelContiguousSampleMaxGapSeconds = 60;'
require_text windows-client/src/CodexInfo.WindowsClient/Controls/GraphPlotControl.cs 'GraphPlotProjection.BuildAxes('
require_text windows-client/src/CodexInfo.WindowsClient/Controls/GraphPlotControl.cs 'internal const string IdleBandColorHex = "#3F5D7C";'
require_text windows-client/src/CodexInfo.WindowsClient/Controls/GraphPlotControl.cs 'internal const double IdleBandOpacity = 0.22;'
require_text windows-client/src/CodexInfo.WindowsClient/Controls/GraphPlotControl.cs 'IdleBandColor.WithOpacity(IdleBandOpacity)'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/GraphPlotControlTests.cs 'PlotProjectionDoesNotInventSpendDuringAnUnobservedGap'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/GraphPlotControlTests.cs 'Remaining_accepts_a_delayed_lower_quota_after_unobserved_sol_usage'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/GraphPlotControlTests.cs 'Shared_graph_fixture_matches_the_native_history_oracle'
require_file tests/fixtures/graph_delayed_quota.json
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/CodexInfo.WindowsClient.Presentation.Tests.csproj 'graph_delayed_quota.json'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/GraphPlotControlTests.cs 'IdleBandsUseTheDedicatedVisibleNeutralColor'
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'AutomationProperties.Name="{Binding Texts.Save}"'
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'LanguageOptions'
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'SelectedValueBinding="{Binding Id}"'
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml.cs 'if (DataContext is SettingsViewModel viewModel && viewModel.Save())'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs 'public bool Save()'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs 'public bool SettingsSaveFailed =>'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs 'if (!PersistSettings(setupCompleted: true))'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsEnabled="{Binding HasPeriods}"'
require_text windows-client/src/CodexInfo.WindowsClient/App.axaml 'Noto Sans JP Medium'
require_text windows-client/src/CodexInfo.WindowsClient/App.axaml 'ComboBox:focus'
require_text windows-client/src/CodexInfo.WindowsClient/App.axaml 'CheckBox:focus'
require_text windows-client/src/CodexInfo.WindowsClient/Localization/UiText.cs 'public string FormatElapsed(long? timestamp, string label)'
require_text windows-client/src/CodexInfo.WindowsClient/Localization/UiText.cs 'public static string NormalizeLanguageCode(string? code)'
require_file scripts/windows_window_move_smoke.ps1
require_file windows-client/src/CodexInfo.WindowsClient/WindowDragBehavior.cs
require_text windows-client/src/CodexInfo.WindowsClient/WindowDragBehavior.cs 'window.BeginMoveDrag(eventArgs)'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/ModelUsageViewModel.cs 'public sealed class ModelUsageViewModel : INotifyPropertyChanged, IDisposable'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs 'public ulong ActiveThreadCount => snapshot?.ActiveThreadCount ?? detailsSnapshot?.ActiveThreadCount ?? 0;'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/DetailsWindowViewModels.cs 'Texts.ParentUnavailable'
require_file windows-client/installer/CodexInfo.WindowsClient.iss
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'PrivilegesRequired=lowest'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'SetupArchitecture=x64'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'WizardStyle=modern dynamic'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'SetupIconFile={#ProductIcon}'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'UninstallDisplayIcon={app}\{#ProductExecutable}'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'CloseApplicationsFilter={#ProductExecutable}'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'Source: "{#PayloadDir}\*"'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'Name: "{group}\{#ProductName}"'
require_text windows-client/installer/CodexInfo.WindowsClient.iss 'Type: files; Name: "{app}\CodexInfo.WindowsClient.Uninstaller.exe"'
require_file windows-client/src/CodexInfo.WindowsClient/Assets/CodexInfo.ico
require_file LICENSES/Inno-Setup.txt
require_text windows-client/tools/Build-WindowsInstaller.ps1 'Inno Setup 7\ISCC.exe'
require_text windows-client/tools/Build-WindowsInstaller.ps1 'CodexInfo.WindowsClient.iss'
require_text windows-client/tools/Build-WindowsInstaller.ps1 'Collect-ThirdPartyNotices.ps1'
require_text windows-client/tools/Build-WindowsInstaller.ps1 '--locked-mode'
require_file windows-client/tools/Measure-WindowsGraphLatency.ps1
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 "CODEX_INFO_WINDOWS_PREVIEW = 'graph'"
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 "CODEX_INFO_WINDOWS_PREVIEW_SIZE = '940x640'"
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'CODEX_INFO_WINDOWS_PREVIEW_GRAPH_POINTS'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Graph.Toggle.Remaining'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Graph.Toggle.LUNA'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Graph.Toggle.TERRA'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Graph.Toggle.SOL'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Graph.PeriodSelector'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Graph.MetricSelector'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'CopyFromScreen'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'SendInput'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Assert-E2EQuotaGaugePalette'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Assert-E2EGraphHasModelData'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Assert-E2EGraphHasModelData $plot $graph.Handle $graphPast'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Main.DetailsStatus'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'parts[1] == "/v1/health"'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 '\"service\":\"codex-info\"'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 "main-quota-gauge: seven cells, two X-authority surface colors, and half-period boundary PASS"
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Assert-E2EMainProductVersion'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Assert-E2ENoChildProductVersion'
require_text scripts/final_acceptance_gate.sh 'main-product-version: PASS'
require_text scripts/final_acceptance_gate.sh 'child-product-version: PASS role=Graph count=0'
require_text scripts/final_acceptance_gate.sh 'child-product-version: PASS role=Threads count=0'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'graph-past-model-data: PASS'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Assert-E2EGraphHasIdleBand $plot $graph.Handle $graphPast'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'graph-past-idle-band: PASS'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'ExpectedStartFraction = 0.01'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'ExpectedEndFraction = 0.35'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 '"timestamp":$($pastStart + 3600)'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'AutomationProperties.AutomationId="Graph.Window.Close"'
require_text windows-client/src/CodexInfo.WindowsClient/ThreadsWindow.axaml 'AutomationProperties.AutomationId="Threads.Window.Close"'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'AutomationProperties.AutomationId="Legal.Window.Close"'
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'AutomationProperties.AutomationId="Settings.Window.Close"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'AutomationProperties.AutomationId="Setup.Window.Close"'
series_press_count="$(rg -o --fixed-strings 'Classes="series-toggle" ClickMode="Press"' windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml | wc -l)"
if [[ "$series_press_count" -ne 4 ]]; then
    fail 'all four graph series toggles must acknowledge pointer press immediately'
fi
selector_press_count="$(rg -o --fixed-strings 'ClickMode="Press"' windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml | wc -l)"
if [[ "$selector_press_count" -ne 6 ]]; then
    fail 'both graph selectors and all four series toggles must acknowledge pointer press immediately'
fi
if rg -q --fixed-strings 'mouse_event' windows-client/tools/Measure-WindowsGraphLatency.ps1; then
    fail 'graph latency probe must use checked SendInput rather than deprecated mouse_event'
fi
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Threading.Thread]::Yield'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'graphMenuPaintProbeExtraHeight = 72'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Get-VisibleGraphMenuItemCount $menu'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'ToggleState]::On'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P90Limit 75'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P95Limit 100'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P90Limit 100'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P95Limit 150'
if rg -q --fixed-strings 'Measure-WindowsGraphLatency.ps1' .github/workflows/windows-client.yml; then
    fail 'hosted Windows runner must not be used as an absolute graph performance gate'
fi
if rg -q --fixed-strings 'Start-Sleep' windows-client/tools/Measure-WindowsGraphLatency.ps1; then
    fail 'graph latency probe must use polling, not fixed sleeps'
fi
require_text docs/WINDOWS_CLIENT_REQUIREMENTS.md 'WIN-PAR-13'
require_text docs/WINDOWS_CLIENT_REQUIREMENTS.md 'WIN-INSTALL-01'
require_text docs/REGRESSION_PREVENTION_POLICY.md 'windows_window_move_smoke.ps1 -AllowPhysicalInput'
require_file scripts/x11_graph_visual_gate.sh
require_text scripts/x11_graph_visual_gate.sh 'dedicated idle-band pixels are missing'
require_text scripts/x11_graph_visual_gate.sh 'implausible vertical stroke'
require_text docs/PRODUCT_REQUIREMENTS.md '# Codex Info 製品要件'
require_file windows-client/CodeCoverage.runsettings

if command -v dotnet >/dev/null 2>&1; then
    coverage_results="$(mktemp -d "${TMPDIR:-/tmp}/codex-info-windows-coverage.XXXXXX")"
    trap 'rm -rf -- "$coverage_results"' EXIT
    dotnet restore windows-client/CodexInfo.WindowsClient.sln --locked-mode
    dotnet_test_output="$(dotnet test windows-client/CodexInfo.WindowsClient.sln \
        --no-restore \
        --configuration Release \
        --settings windows-client/CodeCoverage.runsettings \
        --collect 'Code Coverage' \
        --results-directory "$coverage_results" \
        --logger 'trx;LogFilePrefix=windows-client' \
        --logger 'console;verbosity=normal' 2>&1)" || {
        printf '%s\n' "$dotnet_test_output" >&2
        fail 'Windows Core/Presentation tests failed'
    }
    printf '%s\n' "$dotnet_test_output"
    mapfile -t trx_reports < <(find "$coverage_results" -type f -name '*.trx' -print | sort)
    [[ "${#trx_reports[@]}" -eq 2 ]] ||
        fail "expected exactly two Windows test result reports, found ${#trx_reports[@]}"
    test_total=0
    passed_total=0
    failed_total=0
    not_executed_total=0
    for trx_report in "${trx_reports[@]}"; do
        counters_line="$(rg -o '<Counters[^>]+' "$trx_report" | tail -n 1 || true)"
        [[ -n "$counters_line" ]] || fail "TRX counters are missing: $trx_report"
        trx_attr() {
            local name="$1"
            sed -n "s/.* $name=\"\([0-9][0-9]*\)\".*/\1/p" <<<"$counters_line"
        }
        total="$(trx_attr total)"
        executed="$(trx_attr executed)"
        passed="$(trx_attr passed)"
        failed="$(trx_attr failed)"
        not_executed="$(trx_attr notExecuted)"
        [[ "$total" =~ ^[1-9][0-9]*$ && "$executed" == "$total" &&
            "$passed" == "$total" && "$failed" == "0" && "$not_executed" == "0" ]] ||
            fail "Windows TRX result is not release-safe: $trx_report total=${total:-missing} executed=${executed:-missing} passed=${passed:-missing} failed=${failed:-missing} notExecuted=${not_executed:-missing}"
        test_total=$((test_total + total))
        passed_total=$((passed_total + passed))
        failed_total=$((failed_total + failed))
        not_executed_total=$((not_executed_total + not_executed))
    done
    minimum_expected_tests=310
    [[ "$test_total" -ge "$minimum_expected_tests" && "$passed_total" -eq "$test_total" &&
        "$failed_total" -eq 0 && "$not_executed_total" -eq 0 ]] ||
        fail "Windows aggregate test counts are not release-safe: total=$test_total minimum=$minimum_expected_tests passed=$passed_total failed=$failed_total notExecuted=$not_executed_total"
    echo "windows-client-contract-gate: Windows tests executed: $test_total"
    mapfile -t coverage_reports < <(find "$coverage_results" -type f -name '*.cobertura.xml' -print)
    [[ "${#coverage_reports[@]}" -gt 0 ]] || fail 'Cobertura report is missing'
    # The coverage collector may emit per-test-process reports as well as a
    # merged report.  Accept only a fresh report that covers both production
    # assemblies; never use a high-rate partial report as a substitute.
    coverage_rate=''
    coverage_source=''
    for coverage_report in "${coverage_reports[@]}"; do
        if ! rg -q --fixed-strings 'name="CodexInfo.WindowsClient.Core"' "$coverage_report" ||
           ! rg -q --fixed-strings 'name="CodexInfo.WindowsClient"' "$coverage_report"; then
            continue
        fi
        candidate_rate="$(sed -n 's/.*<coverage line-rate="\([^"]*\)".*/\1/p' "$coverage_report" | head -n 1)"
        if [[ -n "$candidate_rate" ]] && awk -v rate="$candidate_rate" 'BEGIN { exit !((rate + 0) >= 0.92) }'; then
            coverage_rate="$candidate_rate"
            coverage_source="$coverage_report"
            break
        fi
    done
    [[ -n "$coverage_rate" ]] ||
        fail 'no merged Cobertura report covers both production assemblies at >=92% line coverage'
    awk -v rate="$coverage_rate" -v source="$coverage_source" 'BEGIN { printf "windows-client-contract-gate: unit coverage %.2f%% (%s)\n", rate * 100, source }'
else
    echo 'windows-client-contract-gate: UNVERIFIED: dotnet unavailable; Windows tests were not executed' >&2
    exit 2
fi

echo 'windows-client-contract-gate: PASS'

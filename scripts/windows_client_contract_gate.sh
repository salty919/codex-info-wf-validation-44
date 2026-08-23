#!/usr/bin/env bash
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

require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenGraph"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenThreads"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenLegal"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenSettings"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.QuotaPeriodGauge"'
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
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'Command="{Binding BackCommand}"'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'Command="{Binding NextCommand}"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshCommand'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshHost'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshConfigAliases'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'OnStartOrStopSsh'
require_file windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsSetupConnectionEnvironment.cs
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsSetupConnectionEnvironment.cs 'FileName = "ssh.exe"'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsSetupConnectionEnvironment.cs 'ArgumentList.Add("-L")'
require_text docs/WINDOWS_CLIENT.md '%USERPROFILE%\.ssh\config'
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
require_text .github/workflows/windows-client.yml 'needs: [version-policy, core-tests, windows-build]'
require_text .github/workflows/windows-client.yml 'cancel-in-progress: false'
require_text .github/workflows/windows-client.yml 'Get-GitHubResourceStatus'
require_text .github/workflows/windows-client.yml 'gh api --method POST "repos/$repository/git/refs"'
require_text .github/workflows/windows-client.yml 'gh release create $tag'
require_text .github/workflows/windows-client.yml 'gh release upload $tag $setup $manifest'
require_text .github/workflows/windows-client.yml '-F draft=false'
require_file windows-client/src/CodexInfo.WindowsClient/Settings/ClientSettingsSession.cs
require_file windows-client/src/CodexInfo.WindowsClient/Graphing/GraphPlotProjection.cs
require_text windows-client/src/CodexInfo.WindowsClient/Controls/GraphPlotControl.cs 'GraphPlotProjection.BuildAxes('
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'AutomationProperties.Name="{Binding Texts.Save}"'
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'LanguageOptions'
require_text windows-client/src/CodexInfo.WindowsClient/SettingsWindow.axaml 'SelectedValueBinding="{Binding Id}"'
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
require_text windows-client/tools/Run-WindowsClientE2E.ps1 "main-quota-gauge: seven cells, two X-authority surface colors, and half-period boundary PASS"
if rg -q --fixed-strings 'mouse_event' windows-client/tools/Measure-WindowsGraphLatency.ps1; then
    fail 'graph latency probe must use checked SendInput rather than deprecated mouse_event'
fi
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'Threading.Thread]::Yield'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P90Limit 75'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P95Limit 100'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P90Limit 100'
require_text windows-client/tools/Measure-WindowsGraphLatency.ps1 'P95Limit 120'
require_text .github/workflows/windows-client.yml 'Run installed Windows graph latency budget'
require_text .github/workflows/windows-client.yml 'Measure-WindowsGraphLatency.ps1'
if rg -q --fixed-strings 'Start-Sleep' windows-client/tools/Measure-WindowsGraphLatency.ps1; then
    fail 'graph latency probe must use polling, not fixed sleeps'
fi
require_text docs/WINDOWS_CLIENT_REQUIREMENTS.md 'WIN-PAR-13'
require_text docs/WINDOWS_CLIENT_REQUIREMENTS.md 'WIN-INSTALL-01'
require_text docs/PRODUCT_REQUIREMENTS.md '# Codex Info 製品要件'
require_file windows-client/CodeCoverage.runsettings

if command -v dotnet >/dev/null 2>&1; then
    coverage_results="$(mktemp -d "${TMPDIR:-/tmp}/codex-info-windows-coverage.XXXXXX")"
    trap 'rm -rf -- "$coverage_results"' EXIT
    dotnet restore windows-client/CodexInfo.WindowsClient.sln --locked-mode
    dotnet test windows-client/CodexInfo.WindowsClient.sln \
        --no-restore \
        --configuration Release \
        --settings windows-client/CodeCoverage.runsettings \
        --collect 'Code Coverage' \
        --results-directory "$coverage_results"
    mapfile -t coverage_reports < <(find "$coverage_results" -type f -name '*.cobertura.xml' -print)
    [[ "${#coverage_reports[@]}" -eq 1 ]] || fail "expected one Cobertura report, found ${#coverage_reports[@]}"
    coverage_rate="$(sed -n 's/.*<coverage line-rate="\([^"]*\)".*/\1/p' "${coverage_reports[0]}" | head -n 1)"
    [[ -n "$coverage_rate" ]] || fail 'Cobertura report has no line-rate'
    awk -v rate="$coverage_rate" 'BEGIN { exit !((rate + 0) >= 0.90) }' ||
        fail "unit-testable product logic line coverage is below 90%: $coverage_rate"
    awk -v rate="$coverage_rate" 'BEGIN { printf "windows-client-contract-gate: unit coverage %.2f%%\n", rate * 100 }'
else
    echo 'windows-client-contract-gate: UNVERIFIED: dotnet unavailable; Windows tests were not executed' >&2
    exit 2
fi

echo 'windows-client-contract-gate: PASS'

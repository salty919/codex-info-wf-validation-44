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
count_live_shell_invocations() {
    local file="$1" command="$2"
    awk -v command="$command" '
        /^[[:space:]]*#/ { next }
        {
            line = $0
            sub(/[[:space:]]*#.*/, "", line)
            if (index(line, command)) count++
        }
        END { print count + 0 }
    ' "$file"
}
require_function_text() {
    local file="$1" function_name="$2" pattern="$3" function_text
    function_text="$(
        awk -v target="$function_name" '
            $0 ~ "^function " target "[[:space:]]*\\{" {
                capturing = 1
                start = NR
            }
            capturing && NR > start && $0 ~ "^function " { exit }
            capturing { print }
        ' "$file"
    )"
    [[ -n "$function_text" ]] || fail "missing function boundary: $file: $function_name"
    rg -q --fixed-strings -- "$pattern" <<<"$function_text" ||
        fail "missing in function boundary: $file: $function_name: $pattern"
}
require_window_text() {
    local file="$1" pattern="$2"
    if ! sed -n '/<Window /,/Background=/p' "$file" | rg -q --fixed-strings -- "$pattern"; then
        fail "missing top-level Window geometry: $file: $pattern"
    fi
}
require_update_property_contract() {
    local file="$1" property="$2" update_member="$3"
    local property_text
    property_text="$(
        awk -v property="$property" '
            !capturing && $0 ~ "^[[:space:]]*public bool " property "[[:space:]]*$" {
                capturing = 1
            }
            capturing {
                print
                line = $0
                opening = gsub(/\{/, "", line)
                closing = gsub(/\}/, "", line)
                depth += opening - closing
                if (opening > 0) saw_open = 1
                if (saw_open && depth == 0) exit
            }
        ' "$file"
    )"
    [[ -n "$property_text" ]] || fail "missing property boundary: $file: $property"

    for required in \
        '!IsAuthRequired' \
        '!hasConnectionFailure' \
        '!initialLoadPending' \
        '!refreshing' \
        "update?.${update_member} == true"; do
        if ! awk -v required="$required" '
            index($0, required) {
                start = index($0, required)
                before = start > 1 ? substr($0, start - 1, 1) : ""
                after = substr($0, start + length(required), 1)
                if (before !~ /[[:alnum:]_]/ && after !~ /[[:alnum:]_]/) {
                    found = 1
                }
            }
            END { exit !found }
        ' <<<"$property_text"; then
            fail "missing in $property property boundary: $required"
        fi
    done
}

require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenGraph"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenThreads"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenLegal"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Click="OnOpenSettings"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.QuotaPeriodGauge"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.StartupLoading"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.DetailsGenerationContract"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'IsVisible="{Binding ShowAuthenticatedContent}"'
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
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'AutomationProperties.AutomationId="Legal.Notice.Text"'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'AutomationProperties.AutomationId="Legal.Page.Position"'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'AutomationProperties.AutomationId="Legal.Page.Back"'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'AutomationProperties.AutomationId="Legal.Page.Next"'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticeCatalog.cs 'ProjectMarkdownToPlainText'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/LegalNoticeCatalogTests.cs 'MarkdownProjectionUsesTheFiniteLegalNoticePlainTextOracle'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/LegalNoticeCatalogTests.cs 'EveryPackagedMarkdownUrlAndCodeBodySurvivesProjection'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/LegalNoticeCatalogTests.cs 'FencedCodePreservesHtmlCommentDelimitersWhileOutsideProjectionRemovesThem'
require_text windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/LegalNoticeCatalogTests.cs 'MalformedInjectedMarkdownUsesTheExistingFailClosedLoadPage'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'legal-plain-text: PASS (all 9 rendered notices, Back, Minimize, and Close are usable)'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Legal Back navigation from page 9 to page 8'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Legal window minimize'
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
require_text docs/PRODUCT_REQUIREMENTS.md '初回起動では、health・status・detailsの最初の完全な世代が揃うまで'
require_text docs/REGRESSION_PREVENTION_POLICY.md 'REG-STARTUP-FRAME'
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
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'IsVisible="{Binding IsUpdateActionVisible}"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'Command="{Binding UpdateCommand}"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.Status.Update"'
main_window_view_model=windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs
require_file "$main_window_view_model"
require_update_property_contract "$main_window_view_model" IsUpdateNotificationVisible IsNotificationVisible
require_update_property_contract "$main_window_view_model" IsUpdateActionVisible IsUpdateActionVisible
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs 'public bool IsStartupLoading => initialLoadPending'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/MainWindowViewModel.cs 'public bool ShowAuthenticatedContent => IsAuthenticated && !IsStartupLoading'
require_text windows-client/src/CodexInfo.WindowsClient.Core/WindowsUpdateClient.cs 'https://api.github.com/repos/salty919/codex_info_v2/releases?per_page=20'
require_text windows-client/src/CodexInfo.WindowsClient/Infrastructure/WindowsUpdateCoordinator.cs 'StartAvailableUpdateAsync'
require_text windows-client/Directory.Build.props '<Version>'
require_file windows-client/src/CodexInfo.WindowsClient.Core/ProductInfo.cs
require_text windows-client/src/CodexInfo.WindowsClient.Core/ProductInfo.cs 'public static string DisplayVersion'
require_text windows-client/src/CodexInfo.WindowsClient.Core/ProductInfo.cs 'Assembly.GetName().Version'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'ProductVersionText'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml 'AutomationProperties.AutomationId="Main.ProductVersion"'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ConnectionProcessFactory.cs 'codex_info'
require_text windows-client/src/CodexInfo.WindowsClient/Settings/ConnectionProcessFactory.cs '--port'
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
require_text ui/app.slint 'startup-loading: false'
require_text ui/app.slint 'text: "◌  " + root.strings.checking;'
require_text src/main.rs '"startup-loading"'
require_text src/main.rs 'native_startup_failure_releases_loading_surface'
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
require_text .github/workflows/windows-client.yml 'uses: ./.github/workflows/rust.yml'
require_text .github/workflows/windows-client.yml 'needs: [version-prepared, native-quality, codeql-analysis, windows-quality, ui-quality]'
require_text .github/workflows/windows-client.yml 'product: ${{ steps.scope.outputs.product }}'
require_text .github/workflows/windows-client.yml 'Classify pull request scope from the trusted base'
require_text .github/workflows/windows-client.yml 'git show "$BASE_SHA:scripts/ci_change_scope.py" > "$classifier"'
require_text .github/workflows/windows-client.yml 'scope="$(python3 "$classifier"'
require_text .github/workflows/windows-client.yml "if: steps.scope.outputs.product == 'true'"
require_text .github/workflows/windows-client.yml 'Run final acceptance gate before merge'
require_text .github/workflows/windows-client.yml 'windows_window_move_smoke.ps1 -ClientPath $exe -AllowPhysicalInput'
require_text .github/workflows/windows-client.yml "if: always() && github.event_name == 'pull_request'"
require_text .github/workflows/windows-client.yml 'VERSION_RESULT: ${{ needs.version-prepared.result }}'
require_text .github/workflows/windows-client.yml 'CODEQL_RESULT: ${{ needs.codeql-analysis.result }}'
require_text .github/workflows/windows-client.yml 'PRODUCT_CHANGED: ${{ needs.version-prepared.outputs.product }}'
require_text .github/workflows/windows-client.yml '[[ "$VERSION_RESULT" == success ]]'
require_text .github/workflows/windows-client.yml '[[ "$VERSION_READY" == true ]]'
require_text .github/workflows/windows-client.yml '[[ -z "$VERSION_READY" ]]'
require_text .github/workflows/windows-client.yml 'for result in "$NATIVE_RESULT" "$CODEQL_RESULT" "$WINDOWS_RESULT" "$UI_RESULT"; do'
require_text .github/workflows/windows-client.yml '[[ "$result" == success ]]'
require_text .github/workflows/windows-client.yml '[[ "$result" == skipped ]]'
product_job_condition="if: github.event_name == 'pull_request' && needs.version-prepared.outputs.product == 'true' && needs.version-prepared.outputs.ready == 'true'"
product_job_count="$(rg -o --fixed-strings -- "$product_job_condition" .github/workflows/windows-client.yml | wc -l)"
[[ "$product_job_count" -eq 4 ]] ||
    fail "native, CodeQL, Windows, and UI quality jobs must be product-only: count=$product_job_count"
acceptance_product_condition="if: needs.version-prepared.outputs.product == 'true'"
acceptance_product_count="$(rg -o --fixed-strings -- "$acceptance_product_condition" .github/workflows/windows-client.yml | wc -l)"
[[ "$acceptance_product_count" -eq 8 ]] ||
    fail "all eight product acceptance steps must be scope-guarded: count=$acceptance_product_count"
ui_quality_scope="$(
    awk '
        $0 == "      - name: Write UI quality evidence" {
            capturing = 1
            next
        }
        capturing && $0 ~ /^      - name:/ { exit }
        capturing { print }
    ' .github/workflows/windows-client.yml
)"
[[ -n "$ui_quality_scope" ]] || fail 'missing Write UI quality evidence step body'
require_ui_quality_text() {
    local pattern="$1"
    rg -q --fixed-strings -- "$pattern" <<<"$ui_quality_scope" ||
        fail "missing in Write UI quality evidence step: $pattern"
}
require_ui_quality_text '$qualityLines'
require_ui_quality_text '$qualityPath'
require_ui_quality_text 'ui-quality.txt'
require_ui_quality_text '$hashLines'
require_ui_quality_text '$manifestPath'
require_ui_quality_text 'SHA256SUMS'
require_ui_quality_text '[System.Text.UTF8Encoding]::new($false)'
require_ui_quality_text '[System.IO.File]::WriteAllText('
require_ui_quality_text '$qualityLines -join "`n"'
require_ui_quality_text '$hashLines -join "`n"'
if rg -q --fixed-strings -- 'Set-Content' <<<"$ui_quality_scope"; then
    fail 'Write UI quality evidence must not use Set-Content'
fi
write_all_text_count="$(rg -o --fixed-strings '[System.IO.File]::WriteAllText(' <<<"$ui_quality_scope" | wc -l)"
[[ "$write_all_text_count" -eq 2 ]] ||
    fail "UI quality marker and SHA256SUMS must each use WriteAllText: count=$write_all_text_count"
utf8_no_bom_count="$(rg -o --fixed-strings '[System.Text.UTF8Encoding]::new($false)' <<<"$ui_quality_scope" | wc -l)"
[[ "$utf8_no_bom_count" -eq 1 ]] ||
    fail "UI quality evidence must declare one UTF-8 no-BOM encoding: count=$utf8_no_bom_count"
quality_lines_line="$(rg -n -m1 --fixed-strings '$qualityLines = @(' <<<"$ui_quality_scope" | cut -d: -f1)"
quality_write_line="$(rg -n -m1 --fixed-strings '[System.IO.File]::WriteAllText(' <<<"$ui_quality_scope" | cut -d: -f1)"
hash_lines_line="$(rg -n -m1 --fixed-strings '$hashLines = @(' <<<"$ui_quality_scope" | cut -d: -f1)"
manifest_write_line="$(rg -n -m2 --fixed-strings '[System.IO.File]::WriteAllText(' <<<"$ui_quality_scope" | tail -n1 | cut -d: -f1)"
[[ -n "$quality_lines_line" && -n "$quality_write_line" && -n "$hash_lines_line" && -n "$manifest_write_line" ]] ||
    fail 'UI quality marker and manifest writer ordering markers are incomplete'
(( quality_lines_line < quality_write_line && quality_write_line < hash_lines_line && hash_lines_line < manifest_write_line )) ||
    fail 'UI quality lines must be written before SHA256SUMS is calculated and written'
workflow_contract_scope="$(
    awk '
        $0 == "  windows-quality:" {
            in_windows_quality = 1
            next
        }
        in_windows_quality && $0 ~ /^  [[:alnum:]_-]+:/ {
            exit
        }
        in_windows_quality &&
            $0 == "      - uses: actions/checkout@v4" {
            checkout_count++
            checkout_line = NR
        }
        in_windows_quality &&
            $0 == "      - name: Audit live applied merge rules" {
            audit_count++
            audit_line = NR
        }
        in_windows_quality &&
            $0 == "      - name: Write Windows quality evidence" {
            write_count++
            write_line = NR
        }
        in_windows_quality &&
            $0 == "      - name: Upload Windows quality evidence" {
            upload_count++
            upload_line = NR
        }
        in_windows_quality && index($0, "apt-get") {
            apt_count++
        }
        in_windows_quality && index($0, "actions/setup-dotnet@") {
            dotnet_setup_count++
        }
        in_windows_quality && index($0, "scripts/windows_client_contract_gate.sh") {
            contract_call_count++
        }
        in_windows_quality && index($0, "WINDOWS_CONTRACT_EVIDENCE_DIR") {
            evidence_env_count++
        }
        END {
            printf "%d %d %d %d %d %d %d %d %d %d %d %d\n",
                checkout_count + 0,
                audit_count + 0,
                write_count + 0,
                upload_count + 0,
                checkout_line + 0,
                audit_line + 0,
                write_line + 0,
                upload_line + 0,
                apt_count + 0,
                dotnet_setup_count + 0,
                contract_call_count + 0,
                evidence_env_count + 0
        }
    ' .github/workflows/windows-client.yml
)"
read -r checkout_count audit_count write_count upload_count \
    checkout_line audit_line write_line upload_line apt_count dotnet_setup_count \
    contract_call_count evidence_env_count <<<"$workflow_contract_scope"
[[ "$checkout_count" -eq 1 && "$audit_count" -eq 1 &&
    "$write_count" -eq 1 && "$upload_count" -eq 1 ]] ||
    fail "Windows PR quality job must have one checkout, live audit, provenance, and upload step"
[[ "$checkout_line" -lt "$audit_line" && "$audit_line" -lt "$write_line" &&
    "$write_line" -lt "$upload_line" ]] ||
    fail "Windows live audit and provenance steps are out of order"
[[ "$apt_count" -eq 0 && "$dotnet_setup_count" -eq 0 &&
    "$contract_call_count" -eq 0 && "$evidence_env_count" -eq 0 ]] ||
    fail "Windows PR quality job must not run local contract setup, tests, or evidence export"
require_text .github/workflows/windows-client.yml 'source-sha: $source_sha'
require_text .github/workflows/windows-client.yml 'tree-sha: $tree_sha'
require_text .github/workflows/windows-client.yml 'quality: merge-policy'
require_text .github/workflows/windows-client.yml 'live-applied-rules: PASS'
require_text .github/workflows/windows-client.yml 'merge-policy: PASS'
if rg -q --fixed-strings 'windows-contract: PASS' .github/workflows/windows-client.yml ||
   rg -q --fixed-strings 'windows-tests: PASS' .github/workflows/windows-client.yml ||
   rg -q --fixed-strings 'windows-quality: PASS' .github/workflows/windows-client.yml; then
    fail 'Windows merge-policy evidence retains obsolete test or Windows-quality markers'
fi
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
require_text scripts/final_acceptance_gate.sh 'grep -Fq --'
require_text scripts/final_acceptance_gate.sh 'grep -F --'
require_file scripts/final_acceptance_gate_test.sh
require_text scripts/final_acceptance_gate_test.sh 'final-acceptance-gate-test: PASS cases=$cases'
require_text scripts/final_acceptance_gate_test.sh 'isolated acceptance PATH unexpectedly contains rg'
require_text scripts/quality_artifact_gate.sh 'release-build: PASS'
require_text scripts/quality_artifact_gate.sh 'cli-contract-e2e: PASS'
require_text scripts/quality_artifact_gate.sh 'quality: merge-policy'
require_text scripts/quality_artifact_gate.sh 'live-applied-rules: PASS'
require_text scripts/quality_artifact_gate.sh 'merge-policy: PASS'
require_text .github/workflows/windows-client.yml 'cancel-in-progress: false'
require_text .github/workflows/windows-client.yml 'pull_request_target:'
require_text .github/workflows/windows-client.yml 'types: [closed]'
require_text .github/workflows/windows-client.yml 'version-prepared:'
require_text .github/workflows/windows-client.yml 'contents: read'
require_text .github/workflows/windows-client.yml 'pull-requests: read'
require_text .github/workflows/windows-client.yml "if: github.event_name == 'pull_request_target' && github.event.pull_request.merged == true"
require_text .github/workflows/windows-client.yml 'Classify merged pull request scope'
require_text .github/workflows/windows-client.yml 'python3 scripts/ci_change_scope.py'
require_text .github/workflows/windows-client.yml 'python3 scripts/release_quality_run_resolver.py'
require_text .github/workflows/windows-client.yml 'runs?event=pull_request&head_sha=$PR_HEAD_SHA&per_page=100'
release_product_count="$(
    awk '
        $0 == "  release:" { in_release = 1; next }
        in_release && $0 ~ /^  [[:alnum:]_-]+:/ { exit }
        in_release { print }
    ' .github/workflows/windows-client.yml |
        rg -o --fixed-strings "if: steps.scope.outputs.product == 'true'" |
        wc -l
)"
[[ "$release_product_count" -eq 5 ]] ||
    fail "all five post-merge product steps must be scope-guarded: count=$release_product_count"
if rg -q --fixed-strings '.pull_requests' .github/workflows/windows-client.yml; then
    fail 'post-merge release must not trust the optional workflow-run pull_requests association'
fi
if rg -q --fixed-strings 'runs?event=pull_request&status=completed&head_sha=' .github/workflows/windows-client.yml; then
    fail 'post-merge release workflow must query all workflow-run statuses'
fi
require_file .github/workflows/version-prepare.yml
require_text .github/workflows/version-prepare.yml 'pull_request_target:'
require_text .github/workflows/version-prepare.yml 'ref: refs/heads/main'
require_text .github/workflows/version-prepare.yml 'persist-credentials: false'
require_text .github/workflows/version-prepare.yml 'Classify pull request scope'
require_text .github/workflows/version-prepare.yml 'python3 scripts/ci_change_scope.py'
require_text .github/workflows/version-prepare.yml "if: steps.scope.outputs.product == 'true'"
require_text .github/workflows/version-prepare.yml 'pull-requests: read'
require_text .github/workflows/version-prepare.yml 'python3 scripts/product_version.py bump --expected "$base_version"'
require_text .github/workflows/version-prepare.yml 'force=false'
if rg -q --fixed-strings 'ref: ${{ github.event.pull_request.head.sha }}' .github/workflows/version-prepare.yml ||
   rg -q --fixed-strings 'checks: write' .github/workflows/version-prepare.yml; then
    fail 'trusted version preparer must not checkout PR code or own a duplicate check result'
fi
if rg -q '^  push:' .github/workflows/windows-client.yml; then
    fail 'main push must not rerun PR quality or release tests'
fi
require_text .github/workflows/rust.yml 'dtolnay/rust-toolchain@stable'
require_text .github/workflows/rust.yml 'x11-apps'
require_text .github/workflows/rust.yml 'cd artifacts/native-quality'
require_text .github/workflows/rust.yml 'sha256sum native-quality.txt > SHA256SUMS'
require_text .github/workflows/rust.yml 'release-build: PASS'
require_text .github/workflows/rust.yml 'cli-contract-e2e: PASS'
require_text .github/workflows/rust.yml 'recorder-daemon: PASS'
for obsolete_native_marker in 'regression-guard: PASS' 'data-protection: PASS'; do
    if rg -q --fixed-strings -- "$obsolete_native_marker" .github/workflows/rust.yml; then
        fail "native PR artifact retains obsolete marker: $obsolete_native_marker"
    fi
done
if rg -q --fixed-strings -- 'sha256sum artifacts/native-quality/native-quality.txt' .github/workflows/rust.yml; then
    fail 'native quality manifest must use bundle-relative paths'
fi
final_gate_line="$(rg -n 'bash scripts/final_acceptance_gate.sh artifacts/windows-ui-e2e' .github/workflows/windows-client.yml | cut -d: -f1 | head -n 1)"
[[ -n "$final_gate_line" ]] || fail 'final acceptance gate invocation is missing'
require_file scripts/pre_pr_gate.sh
pre_pr_regression_calls="$(count_live_shell_invocations scripts/pre_pr_gate.sh 'bash scripts/regression_guard.sh')"
[[ "$pre_pr_regression_calls" -eq 1 ]] ||
    fail "pre_pr_gate must own exactly one regression guard invocation: count=$pre_pr_regression_calls"
pre_pr_contract_calls="$(count_live_shell_invocations scripts/pre_pr_gate.sh 'bash scripts/windows_client_contract_gate.sh')"
[[ "$pre_pr_contract_calls" -eq 1 ]] ||
    fail "pre_pr_gate must own exactly one Windows contract gate invocation: count=$pre_pr_contract_calls"
pre_pr_data_calls="$(count_live_shell_invocations scripts/pre_pr_gate.sh 'bash scripts/data_protection_gate.sh')"
[[ "$pre_pr_data_calls" -eq 1 ]] ||
    fail "pre_pr_gate must own exactly one data protection gate invocation: count=$pre_pr_data_calls"
for workflow in .github/workflows/windows-client.yml .github/workflows/rust.yml; do
    if rg -q --fixed-strings 'scripts/regression_guard.sh' "$workflow" ||
       rg -q --fixed-strings 'scripts/data_protection_gate.sh' "$workflow" ||
       rg -q --fixed-strings 'scripts/windows_client_contract_gate.sh' "$workflow"; then
        fail "local pre-PR gates must not be executable owners in PR workflow: $workflow"
    fi
done
if awk '
    /^[[:space:]]*#/ { next }
    {
        line = $0
        sub(/[[:space:]]*#.*/, "", line)
        if (line ~ /(^|[;&|[:space:]])cargo[[:space:]]+test([[:space:]]|$)/) found = 1
    }
    END { exit !found }
' scripts/data_protection_gate.sh; then
    fail 'data protection gate must not invoke cargo test'
fi
db_fixture_calls="$(count_live_shell_invocations scripts/data_protection_gate.sh 'bash scripts/db_protection_e2e.sh')"
[[ "$db_fixture_calls" -eq 1 ]] ||
    fail "data protection gate must invoke the SQLite fixture exactly once: count=$db_fixture_calls"
if awk '
    /^[[:space:]]*#/ { next }
    {
        line = $0
        sub(/[[:space:]]*#.*/, "", line)
        if (line ~ /(^|[;&|[:space:]])cargo[[:space:]]+test([[:space:]]|$)/) found = 1
    }
    END { exit !found }
' scripts/db_protection_e2e.sh; then
    fail 'DB protection fixture must not invoke cargo test'
fi
for required_db_test in \
    db_protection_runtime_backup_migration_restore \
    backup_generations_are_sqlite_consistent_and_bounded \
    failed_backup_rotation_keeps_existing_generation_untouched \
    verified_migration_switches_only_after_candidate_validation \
    invalid_migration_candidate_leaves_source_untouched \
    migration_that_drops_a_valid_row_is_rejected_before_switch \
    opening_an_old_schema_is_rejected_without_migration \
    corrupt_database_error_preserves_the_original_file; do
    require_text scripts/regression_guard.sh "$required_db_test"
done
require_text scripts/regression_guard.sh 'cargo check --locked --all-targets'
require_text scripts/regression_guard.sh 'cargo test --locked --all-targets'
require_text scripts/regression_guard.sh 'cargo build --release --locked'
all_target_test_invocations="$(count_live_shell_invocations scripts/regression_guard.sh 'cargo test --locked --all-targets')"
[[ "$all_target_test_invocations" -eq 1 ]] ||
    fail "regression guard must execute one all-target test command and inspect its output: count=$all_target_test_invocations"
if rg -q --fixed-strings -- '--exact --nocapture' scripts/regression_guard.sh; then
    fail 'required Rust tests must inspect the one all-target test output instead of rerunning --exact tests'
fi
require_text scripts/regression_guard.sh 'Rust all-target test set contains a zero-test target'
require_text scripts/regression_guard.sh 'X11 graph visual gate unverified (DISPLAY unavailable)'
require_text .github/workflows/rust.yml 'xvfb-run --auto-servernum'
require_text .github/workflows/windows-client.yml 'Get-GitHubResourceStatus'
require_text .github/workflows/windows-client.yml 'gh api --method POST "repos/$repository/git/refs"'
require_text .github/workflows/windows-client.yml 'gh api --method POST "repos/$repository/releases"'
require_text .github/workflows/windows-client.yml 'python3 scripts/release_state_gate.py created --tag $tag'
require_text .github/workflows/windows-client.yml '$createdReleaseId = [long]$createdRelease.id'
require_text .github/workflows/windows-client.yml '$draftReleaseEndpoint = "repos/$repository/releases/$createdReleaseId"'
require_text .github/workflows/windows-client.yml 'python3 scripts/release_state_gate.py draft `'
require_text .github/workflows/windows-client.yml 'python3 scripts/release_state_gate.py tag --sha $env:EXPECTED_MERGE_SHA'
require_text .github/workflows/windows-client.yml 'python3 scripts/release_state_gate.py published `'
require_text .github/workflows/windows-client.yml 'gh release upload $tag $setup $manifest'
require_text .github/workflows/windows-client.yml 'gh api --method PATCH "repos/$repository/releases/$createdReleaseId"'
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
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'public static extern bool PrintWindow'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Capture-E2EWindow 'PrintWindow'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Capture-E2EWindow 'PW_RENDERFULLCONTENT'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Capture-E2EWindow 'changedPixels'
if rg -q --fixed-strings -- 'CopyFromScreen' windows-client/tools/Run-WindowsClientE2E.ps1; then
    fail 'target HWND capture must not fall back to CopyFromScreen'
fi
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2ECaptureSelfTest 'target-occluded'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2ECaptureSelfTest 'invalid-hwnd'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2ECaptureSelfTest 'capture-failure'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Test-E2EGraphCompositedSeriesPixel 'alphas'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Test-E2EGraphCompositedSeriesPixel 'minimumCompositedAlpha'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Get-E2EGraphFlatLineCandidates 'rowCandidateThreshold'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Get-E2EGraphFlatLineCandidates 'FlatCoverageThreshold'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Find-E2EGraphSharedRisingSegment 'sourceColors'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Find-E2EGraphSharedRisingSegment 'minimumSharedVerticalExtent'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Find-E2EGraphSharedRisingSegment 'maxAllowedGap'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Find-E2EGraphSharedRisingSegment 'contributions'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Test-E2EGraphIdleBandPixel 'Get-E2EGraphIdleBackgroundColor'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphIdleBandBitmap 'sampleColumn25'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphIdleBandBitmap 'minimumSampleHits'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphIdleBandBitmap 'minimumCoveredColumns'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphHasIdleBand 'Assert-E2EGraphIdleBandBitmap'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 New-E2EGraphIdleBandSyntheticBitmap 'wrong-sample-columns'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphIdleBandSelfTest 'wrong-interval'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphModelPixels 'flatStartFraction'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphModelPixels 'flatCoverageThreshold'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphModelPixels 'risingCenterFraction'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphModelPixels 'Get-E2EGraphFlatLineCandidates'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphModelPixels 'minimumSeparation'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EGraphHasModelData 'Assert-E2EGraphModelPixels'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'missing-flat'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'valid-offset'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'label-vertical-only'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'wrong-geometry'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'wrong-order'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'sloped'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'wrong-background'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'axis-row-missing-terra'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'axis-row-missing-sol'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'wrong-endpoint'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'short-rise'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'detached-rise'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'missing-series-contribution'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EGraphOracleSelfTest 'non-contiguous'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Main.DetailsStatus'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'main-details-status: PASS (matching status/details generation accepted)'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'main-startup-loading: PASS (first complete generation is visible)'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Main details status is not a complete accepted generation'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'parts[1] == "/v1/health"'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 '\"service\":\"codex-info\"'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'Codex-Info-Published-Pair'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'includePublishedPair'
require_text windows-client/tools/Run-WindowsClientE2E.ps1 'strict thirteen-field contract'
require_file windows-client/tools/Test-WindowsClientFixtureContract.ps1
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixtureRawRequest 'X-Codex-Info-E2E-Phase'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureWireContract 'Assert-E2E ($Health.StatusCode -eq 200)'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureWireContract 'Assert-E2E ($statusPair -cmatch'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureWireContract 'Assert-E2EFixtureJsonKeys -Json $statusJson'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureWireContract 'Assert-E2EFixtureJsonKeys -Json $detailsJson'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureWireContract 'Assert-E2EFixtureHistorySamples'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureHistorySamples 'expectedSampleKeys'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureHistorySamples 'timestamp % 60'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureHistorySamples 'periodRecords'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixtureHistorySamples '$reset -eq $previousReset'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixturePreflight 'Invoke-E2EFixtureRawRequest'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixturePreflight 'Assert-E2EFixturePreflightResponses'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Assert-E2EFixturePreflightResponses 'Assert-E2EFixtureWireContract'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 New-E2EFixtureDocuments 'orderedSampleObjects'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixtureContractTests 'pair-missing'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixtureContractTests 'pair-mismatch'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixtureContractTests 'history-gaps-missing'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixtureContractTests 'history-sample-order'
require_function_text windows-client/tools/Run-WindowsClientE2E.ps1 Invoke-E2EFixtureContractTests 'history-sample-minute-bucket'
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
require_text docs/REGRESSION_PREVENTION_POLICY.md '物理window move証拠'
require_text docs/REGRESSION_PREVENTION_POLICY.md 'CI受入時の`-AllowPhysicalInput`実行ログ'
require_text .github/workflows/windows-client.yml '$moveSmokeOutput = @(& ./scripts/windows_window_move_smoke.ps1'
require_text .github/workflows/windows-client.yml "[string]\$moveSmokeOutput[-1] -ne 'window-move-smoke: PASS'"
if rg -q --fixed-strings 'if ($LASTEXITCODE -ne 0) { throw '\''Physical window move smoke failed.'\'' }' .github/workflows/windows-client.yml; then
    fail 'physical move smoke must not inherit a stale native LASTEXITCODE from its caller'
fi
require_file scripts/x11_graph_visual_gate.sh
require_file scripts/x11_startup_visual_gate.sh
require_file scripts/workflow_quality_gate.py
require_file scripts/ci_trust_fixture.py
require_file scripts/ci_change_scope.py
require_file scripts/test_ci_change_scope.py
require_file scripts/test_codeql_workflow.py
require_file .github/workflows/codeql.yml
require_file scripts/release_candidate_gate.sh
require_file scripts/release_candidate_gate_test.sh
require_file scripts/release_quality_run_resolver.py
require_file scripts/test_release_quality_run_resolver.py
require_file scripts/release_state_gate.py
require_file docs/REQUIREMENTS_LEDGER.md
for required_ledger_id in X-START-01 X-START-02 X-START-03 X-GRAPH-01 X-THREAD-01 WIN-START-01 WIN-GRAPH-01 WIN-VERSION-01 PROC-LEDGER-01 WF-NONPRODUCT-01; do
    require_text docs/REQUIREMENTS_LEDGER.md "| $required_ledger_id |"
done
require_text scripts/regression_guard.sh 'bash scripts/requirements_ledger_gate.sh --final'
require_text scripts/requirements_ledger_gate.sh 'final gate requires verified status'
require_text scripts/x11_graph_visual_gate.sh 'dedicated idle-band pixels are missing'
require_text scripts/x11_graph_visual_gate.sh 'implausible vertical stroke'
require_text docs/PRODUCT_REQUIREMENTS.md '# Codex Info 製品要件'
require_file windows-client/CodeCoverage.runsettings

# These finite fixtures are owned by this contract gate.  They exercise the
# workflow/release trust boundaries without repeating a product build, unit
# test suite, UI run, or acceptance job.
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_ci_change_scope.py
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_codeql_workflow.py
PYTHONDONTWRITEBYTECODE=1 python3 scripts/workflow_quality_gate.py --self-test
PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci_trust_fixture.py --self-test
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_release_quality_run_resolver.py
bash scripts/final_acceptance_gate_test.sh
bash scripts/release_candidate_gate_test.sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/release_state_gate.py --self-test

if command -v dotnet >/dev/null 2>&1; then
    contract_evidence_dir="${WINDOWS_CONTRACT_EVIDENCE_DIR:-}"
    if [[ -n "$contract_evidence_dir" ]]; then
        [[ ! -e "$contract_evidence_dir" ]] ||
            fail "Windows contract evidence directory already exists: $contract_evidence_dir"
        mkdir -p "$contract_evidence_dir"
    fi
    coverage_results="$(mktemp -d "${TMPDIR:-/tmp}/codex-info-windows-coverage.XXXXXX")"
    trap '
        if [[ -n "${contract_evidence_dir:-}" ]]; then
            cp -R "$coverage_results"/. "$contract_evidence_dir"/
        fi
        rm -rf -- "$coverage_results"
    ' EXIT
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

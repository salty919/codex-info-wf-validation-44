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
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowRemaining}"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowSol}"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowTerra}"'
require_text windows-client/src/CodexInfo.WindowsClient/GraphWindow.axaml 'IsChecked="{Binding ShowLuna}"'
require_text windows-client/src/CodexInfo.WindowsClient/ThreadsWindow.axaml 'ThreadTreeControl'
require_text windows-client/src/CodexInfo.WindowsClient/LegalNoticesWindow.axaml 'ScrollViewer'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshCommand'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshHost'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'SshConfigAliases'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'OnStartOrStopSsh'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs 'FileName = "ssh.exe"'
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs 'ArgumentList.Add("-L")'
require_text docs/WINDOWS_CLIENT.md '%USERPROFILE%\.ssh\config'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'AutomationProperties.Name="{Binding Texts.Copy}"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'Background="Transparent" PointerPressed="OnTitlePointerPressed"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml.cs 'WindowDragBehavior.Begin(this, e)'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'IsEnabled="{Binding CanContinue}"'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml 'Click="OnClose"'
require_text windows-client/src/CodexInfo.WindowsClient/MainWindow.axaml.cs 'PreviewEnvironment.IsChild("settings")'
for window in MainWindow GraphWindow ThreadsWindow LegalNoticesWindow SettingsWindow; do
    require_text "windows-client/src/CodexInfo.WindowsClient/${window}.axaml" 'Background="Transparent" PointerPressed="OnTitlePointerPressed"'
    require_text "windows-client/src/CodexInfo.WindowsClient/${window}.axaml.cs" 'WindowDragBehavior.Begin('
done
require_text windows-client/src/CodexInfo.WindowsClient/ViewModels/SettingsViewModel.cs '1 => main.IsAuthenticated'
require_text windows-client/src/CodexInfo.WindowsClient/SetupWindow.axaml.cs 'if (vm.IsAuthStep && !vm.CanContinue)'
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
require_text windows-client/tools/Install-WindowsClient.ps1 'GetFolderPath('\''Programs'\'')'
require_text windows-client/tools/Uninstall-WindowsClient.ps1 'Codex Info\Codex Info Monitor.lnk'
require_text windows-client/installer/CodexInfo.WindowsClient.Installer.csproj 'PayloadZip'
require_text windows-client/installer/Program.cs 'RegisterUninstall'
require_text windows-client/installer/Program.cs 'DeleteSubKeyTree'
require_text windows-client/installer/Program.cs 'ExtractEmbeddedPayload'
require_text windows-client/tools/Build-WindowsInstaller.ps1 'PublishSingleFile=true'
require_text windows-client/tools/Build-WindowsInstaller.ps1 'Collect-ThirdPartyNotices.ps1'
require_text windows-client/tools/Build-WindowsInstaller.ps1 '--locked-mode'
require_text docs/WINDOWS_CLIENT_REQUIREMENTS.md 'WIN-PAR-13'
require_text docs/WINDOWS_CLIENT_REQUIREMENTS.md 'WIN-INSTALL-01'
require_text docs/PRODUCT_REQUIREMENTS.md 'CodexInfo 製品要件'

if command -v dotnet >/dev/null 2>&1; then
    dotnet restore windows-client/CodexInfo.WindowsClient.sln --locked-mode
    dotnet test windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
else
    echo 'windows-client-contract-gate: dotnet unavailable; static contract checks PASS'
fi

echo 'windows-client-contract-gate: PASS'

# Codex Info Monitor Windows client installer helper.
# Creates a Start Menu shortcut without storing credentials or endpoint settings.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path $_ -PathType Container })]
    [string]$PublishDirectory,
    [switch]$DesktopShortcut
)

$resolvedPublish = (Resolve-Path $PublishDirectory).Path
$exe = Join-Path $resolvedPublish 'CodexInfo.WindowsClient.exe'
if (-not (Test-Path $exe -PathType Leaf)) {
    throw "CodexInfo.WindowsClient.exe was not found in $resolvedPublish"
}

$startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) 'Codex Info'
New-Item -ItemType Directory -Path $startMenu -Force | Out-Null
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $startMenu 'Codex Info Monitor.lnk'))
$shortcut.TargetPath = $exe
$shortcut.WorkingDirectory = $resolvedPublish
$shortcut.Description = 'Codex Info Windows monitoring client'
$shortcut.Save()

if ($DesktopShortcut) {
    $desktop = [Environment]::GetFolderPath('Desktop')
    $desktopShortcut = $shell.CreateShortcut((Join-Path $desktop 'Codex Info Monitor.lnk'))
    $desktopShortcut.TargetPath = $exe
    $desktopShortcut.WorkingDirectory = $resolvedPublish
    $desktopShortcut.Description = 'Codex Info Windows monitoring client'
    $desktopShortcut.Save()
}

Write-Host "Start Menu shortcut created: $startMenu\Codex Info Monitor.lnk"

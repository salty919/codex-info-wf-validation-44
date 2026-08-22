[CmdletBinding()]
param()

$startMenuShortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Codex Info\Codex Info Monitor.lnk'
$desktopShortcut = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Codex Info Monitor.lnk'
foreach ($path in @($startMenuShortcut, $desktopShortcut)) {
    if (Test-Path $path -PathType Leaf) {
        Remove-Item -LiteralPath $path -Force
    }
}
$startMenu = Split-Path $startMenuShortcut -Parent
if ((Test-Path $startMenu -PathType Container) -and -not (Get-ChildItem -LiteralPath $startMenu -Force)) {
    Remove-Item -LiteralPath $startMenu -Force
}
Write-Host 'Codex Info Monitor shortcuts removed.'

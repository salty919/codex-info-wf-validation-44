# Captures one fresh installed Windows client window for acceptance evidence.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [string]$Preview = 'setup',
    [string]$PreviewSize = '760x680'
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class CodexInfoCaptureWin32 {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int command);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr extra);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr extra);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
'@
[CodexInfoCaptureWin32]::SetProcessDPIAware() | Out-Null

$env:CODEX_INFO_WINDOWS_PREVIEW = $Preview
$env:CODEX_INFO_WINDOWS_PREVIEW_SIZE = $PreviewSize
$expectedTitle = switch ($Preview) {
    { $_ -in @('normal', 'auth', 'error', 'warning', 'danger', 'zero', 'full') } { 'Codex Info Monitor' }
    'graph' { 'Codex Info Graph' }
    'threads' { 'Codex Info Threads' }
    'legal' { 'Codex Info Legal' }
    'settings' { 'Codex Info Settings' }
    default { 'Codex Info Setup' }
}
$script:codexInfoCaptureTitle = $expectedTitle
$exe = Join-Path $env:LOCALAPPDATA 'Programs\Codex Info Monitor\CodexInfo.WindowsClient.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Installed client not found: $exe" }
$process = Start-Process -FilePath $exe -PassThru
try {
    $window = [IntPtr]::Zero
    for ($i = 0; $i -lt 80 -and $window -eq [IntPtr]::Zero; $i++) {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        $callback = [CodexInfoCaptureWin32+EnumWindowsProc] {
            param([IntPtr]$handle, [IntPtr]$extra)
            [uint32]$owner = 0
            [CodexInfoCaptureWin32]::GetWindowThreadProcessId($handle, [ref]$owner) | Out-Null
            if ($owner -ne [uint32]$process.Id -or -not [CodexInfoCaptureWin32]::IsWindowVisible($handle)) { return $true }
            $title = New-Object System.Text.StringBuilder 256
            [CodexInfoCaptureWin32]::GetWindowText($handle, $title, $title.Capacity) | Out-Null
            if ($title.ToString() -like ("*{0}*" -f $script:codexInfoCaptureTitle)) { $script:codexInfoCaptureWindow = $handle; return $false }
            return $true
        }
        $script:codexInfoCaptureWindow = [IntPtr]::Zero
        [CodexInfoCaptureWin32]::EnumWindows($callback, [IntPtr]::Zero) | Out-Null
        $window = $script:codexInfoCaptureWindow
    }
    if ($window -eq [IntPtr]::Zero) { throw "Fresh $Preview window ($expectedTitle) did not open" }
    [CodexInfoCaptureWin32]::ShowWindow($window, 9) | Out-Null
    [CodexInfoCaptureWin32]::BringWindowToTop($window) | Out-Null
    [CodexInfoCaptureWin32]::SetForegroundWindow($window) | Out-Null
    Start-Sleep -Milliseconds 500
    $rect = New-Object CodexInfoCaptureWin32+RECT
    [CodexInfoCaptureWin32]::GetWindowRect($window, [ref]$rect) | Out-Null
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) { throw "Invalid window bounds: ${width}x${height}" }
    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
    $bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    Write-Output "capture: PASS pid=$($process.Id) size=${width}x${height} path=$OutputPath"
}
finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}

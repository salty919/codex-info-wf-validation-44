# Non-invasive borderless-window move smoke.  This uses only targeted
# WM_LBUTTON* messages and never calls SetCursorPos/mouse_event, so the host
# cursor is not moved or captured by the acceptance harness.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ClientPath
)

$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class CodexInfoMessageMoveWin32 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr extra);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr extra);
}
'@

if (-not (Test-Path -LiteralPath $ClientPath -PathType Leaf)) { throw "Client not found: $ClientPath" }
$env:CODEX_INFO_WINDOWS_PREVIEW = 'normal'
$env:CODEX_INFO_WINDOWS_PREVIEW_SIZE = '760x680'
$process = Start-Process -FilePath $ClientPath -PassThru
try {
    $window = [IntPtr]::Zero
    for ($i = 0; $i -lt 80 -and $window -eq [IntPtr]::Zero; $i++) {
        Start-Sleep -Milliseconds 250
        $callback = [CodexInfoMessageMoveWin32+EnumWindowsProc] {
            param([IntPtr]$handle, [IntPtr]$extra)
            [uint32]$owner = 0
            [CodexInfoMessageMoveWin32]::GetWindowThreadProcessId($handle, [ref]$owner) | Out-Null
            if ($owner -ne [uint32]$process.Id -or -not [CodexInfoMessageMoveWin32]::IsWindowVisible($handle)) { return $true }
            $title = New-Object System.Text.StringBuilder 256
            [CodexInfoMessageMoveWin32]::GetWindowText($handle, $title, $title.Capacity) | Out-Null
            if ($title.ToString() -like '*Codex Info Monitor*') { $script:codexInfoMessageWindow = $handle; return $false }
            return $true
        }
        $script:codexInfoMessageWindow = [IntPtr]::Zero
        [CodexInfoMessageMoveWin32]::EnumWindows($callback, [IntPtr]::Zero) | Out-Null
        $window = $script:codexInfoMessageWindow
    }
    if ($window -eq [IntPtr]::Zero) { throw 'Fresh main window not found' }
    [CodexInfoMessageMoveWin32]::SetForegroundWindow($window) | Out-Null
    Start-Sleep -Milliseconds 300
    $before = New-Object CodexInfoMessageMoveWin32+RECT
    [CodexInfoMessageMoveWin32]::GetWindowRect($window, [ref]$before) | Out-Null
    # Client-area title point (80,36) -> targeted left-button press, move,
    # release.  No global cursor API is called.
    $pack = { param([int]$x, [int]$y) [IntPtr]([int64](($y -shl 16) -bor ($x -band 0xffff))) }
    [CodexInfoMessageMoveWin32]::PostMessage($window, 0x0201, [IntPtr]1, (&$pack 80 36)) | Out-Null
    Start-Sleep -Milliseconds 150
    [CodexInfoMessageMoveWin32]::PostMessage($window, 0x0200, [IntPtr]1, (&$pack 230 116)) | Out-Null
    Start-Sleep -Milliseconds 150
    [CodexInfoMessageMoveWin32]::PostMessage($window, 0x0202, [IntPtr]::Zero, (&$pack 230 116)) | Out-Null
    Start-Sleep -Milliseconds 350
    $after = New-Object CodexInfoMessageMoveWin32+RECT
    [CodexInfoMessageMoveWin32]::GetWindowRect($window, [ref]$after) | Out-Null
    if ($after.Left -eq $before.Left -and $after.Top -eq $before.Top) {
        throw "Targeted message drag did not move the window: before=$($before.Left),$($before.Top) after=$($after.Left),$($after.Top)"
    }
    Write-Output "window-message-move: PASS before=$($before.Left),$($before.Top) after=$($after.Left),$($after.Top) cursor-untouched=true"
}
finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}

# Runs the finite Windows UI Automation acceptance path against the installed
# client.  The normal mode uses the configured loopback service.  CI may pass
# -Fixture to provide a bounded, local /v1/status + /v1/details pair; this still
# drives the installed EXE and the real rendered windows, but does not require
# an account or an SSH tunnel.
[CmdletBinding()]
param(
    [string]$ClientPath = '',
    [string]$OutputDirectory = '',
    [switch]$Fixture
)

$ErrorActionPreference = 'Stop'

function Resolve-E2EOutputDirectory {
    param([string]$Requested)

    if (-not [string]::IsNullOrWhiteSpace($Requested)) {
        return [IO.Path]::GetFullPath($Requested)
    }

    # Keep raw logs and screenshots outside the repository.  /mnt/d is the
    # usual WSL hand-off location; the other locations cover hosted Windows
    # runners and local Windows machines without a D: mount.
    if (Test-Path -LiteralPath '/mnt/d' -PathType Container) {
        return [IO.Path]::GetFullPath('/mnt/d/temp/codex-info-windows-e2e')
    }
    if (Test-Path -LiteralPath '/home/salty' -PathType Container) {
        return [IO.Path]::GetFullPath('/home/salty/.cache/codex-info-windows-e2e')
    }
    if (-not [string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
        return [IO.Path]::GetFullPath((Join-Path $env:RUNNER_TEMP 'codex-info-windows-e2e'))
    }
    return [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) 'codex-info-windows-e2e'))
}

$script:e2eOutput = Resolve-E2EOutputDirectory $OutputDirectory
$script:e2eRepositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')).TrimEnd([char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar))
if ($script:e2eOutput.Equals($script:e2eRepositoryRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $script:e2eOutput.StartsWith($script:e2eRepositoryRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'E2E artifacts must be written outside the repository.'
}
New-Item -ItemType Directory -Path $script:e2eOutput -Force | Out-Null
$script:e2eLogPath = Join-Path $script:e2eOutput 'windows-client-e2e.log'
$script:e2eWindowRecords = [System.Collections.Generic.List[object]]::new()
$script:e2eProcess = $null
$script:e2eFixtureRunning = $false
$script:e2eSettingsPath = Join-Path $env:LOCALAPPDATA 'CodexInfo\settings.json'
$script:e2eSettingsBackup = Join-Path ([IO.Path]::GetTempPath()) ("codex-info-e2e-settings-" + [Guid]::NewGuid().ToString('N') + '.json')
$script:e2eSettingsWasPresent = $false

function Write-E2E {
    param([Parameter(Mandatory = $true)][string]$Message)

    $line = "{0} {1}" -f ([DateTimeOffset]::Now.ToString('o')), $Message
    Add-Content -LiteralPath $script:e2eLogPath -Value $line -Encoding utf8
    # Host output keeps helper return values (UIA elements, screenshots, and
    # records) out of the PowerShell pipeline while still exposing the raw
    # line in an interactive/CI log.
    Write-Host $line
}

function Assert-E2E {
    param(
        [Parameter(Mandatory = $true)][bool]$Condition,
        [Parameter(Mandatory = $true)][string]$Message
    )

    if (-not $Condition) {
        throw "ASSERT: $Message"
    }
}

function Wait-E2E {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$Probe,
        [Parameter(Mandatory = $true)][string]$Description,
        [int]$TimeoutSeconds = 30
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        try {
            $value = & $Probe
            if ($null -ne $value -and [bool]$value) {
                return $value
            }
        }
        catch {
            # UIA elements can be replaced while Avalonia paints a new state.
            # Re-querying the same finite target is safe; a timeout is still a
            # hard failure and is never reported as a skip/pass.
        }
        Start-Sleep -Milliseconds 200
    }
    throw "TIMEOUT: $Description (${TimeoutSeconds}s)"
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class CodexInfoWindowsE2EWin32 {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int command);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr extra);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr extra);
    [StructLayout(LayoutKind.Sequential)] public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }
}
'@

if ($Fixture) {
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

public static class CodexInfoWindowsE2EFixtureServer {
    private static TcpListener listener;
    private static Thread worker;
    private static volatile bool running;
    private static string statusBody;
    private static string detailsBody;

    public static bool Start(string status, string details, int port) {
        if (running) return false;
        try {
            statusBody = status;
            detailsBody = details;
            listener = new TcpListener(IPAddress.Loopback, port);
            listener.Start();
            running = true;
            worker = new Thread(Loop) { IsBackground = true, Name = "CodexInfoWindowsE2EFixture" };
            worker.Start();
            return true;
        }
        catch (SocketException) {
            running = false;
            try { if (listener != null) listener.Stop(); } catch { }
            listener = null;
            return false;
        }
    }

    public static bool IsRunning() { return running; }

    public static void Stop() {
        running = false;
        try { if (listener != null) listener.Stop(); } catch { }
        try { if (worker != null) worker.Join(1000); } catch { }
        listener = null;
        worker = null;
    }

    private static void Loop() {
        while (running) {
            TcpClient client = null;
            try {
                client = listener.AcceptTcpClient();
                Handle(client);
            }
            catch (SocketException) {
                if (running) { }
            }
            catch (ObjectDisposedException) {
                if (running) { }
            }
            catch { }
            finally {
                try { if (client != null) client.Close(); } catch { }
            }
        }
    }

    private static void Handle(TcpClient client) {
        using (var stream = client.GetStream()) {
            var buffer = new byte[8192];
            var used = 0;
            while (used < buffer.Length) {
                var read = stream.Read(buffer, used, buffer.Length - used);
                if (read <= 0) break;
                used += read;
                if (used >= 4 &&
                    buffer[used - 4] == 13 && buffer[used - 3] == 10 &&
                    buffer[used - 2] == 13 && buffer[used - 1] == 10) break;
            }
            var request = Encoding.ASCII.GetString(buffer, 0, used);
            var firstLine = request.Split(new[] { "\r\n" }, StringSplitOptions.None)[0];
            var parts = firstLine.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
            var code = 404;
            var reason = "Not Found";
            var body = "{\"api_version\":\"v1\",\"error\":\"not_found\"}";
            if (parts.Length >= 2 && parts[0] == "GET") {
                if (parts[1] == "/v1/status") {
                    code = 200;
                    reason = "OK";
                    body = statusBody;
                }
                else if (parts[1] == "/v1/details") {
                    code = 200;
                    reason = "OK";
                    body = detailsBody;
                }
            }
            else if (parts.Length >= 2) {
                code = 405;
                reason = "Method Not Allowed";
                body = "{\"api_version\":\"v1\",\"error\":\"method_not_allowed\"}";
            }
            var payload = Encoding.UTF8.GetBytes(body);
            var header = String.Format(
                "HTTP/1.1 {0} {1}\r\nContent-Type: application/json; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {2}\r\nConnection: close\r\n\r\n",
                code, reason, payload.Length);
            var headerBytes = Encoding.ASCII.GetBytes(header);
            stream.Write(headerBytes, 0, headerBytes.Length);
            stream.Write(payload, 0, payload.Length);
            stream.Flush();
        }
    }
}
'@
}

[CodexInfoWindowsE2EWin32]::SetProcessDPIAware() | Out-Null

function Find-E2EWindow {
    param(
        [Parameter(Mandatory = $true)][int]$ProcessId,
        [Parameter(Mandatory = $true)][string]$TitleFragment
    )

    $script:e2eSearchProcessId = [uint32]$ProcessId
    $script:e2eSearchTitle = $TitleFragment
    $script:e2eSearchWindow = [IntPtr]::Zero
    $callback = [CodexInfoWindowsE2EWin32+EnumWindowsProc] {
        param([IntPtr]$Handle, [IntPtr]$Extra)
        [uint32]$owner = 0
        [CodexInfoWindowsE2EWin32]::GetWindowThreadProcessId($Handle, [ref]$owner) | Out-Null
        if ($owner -ne $script:e2eSearchProcessId -or
            -not [CodexInfoWindowsE2EWin32]::IsWindowVisible($Handle)) {
            return $true
        }
        $title = New-Object System.Text.StringBuilder 256
        [CodexInfoWindowsE2EWin32]::GetWindowText($Handle, $title, $title.Capacity) | Out-Null
        if ($title.ToString().IndexOf($script:e2eSearchTitle, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            $script:e2eSearchWindow = $Handle
            return $false
        }
        return $true
    }
    [CodexInfoWindowsE2EWin32]::EnumWindows($callback, [IntPtr]::Zero) | Out-Null
    return $script:e2eSearchWindow
}

function Get-E2EWindowBounds {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    $rect = New-Object CodexInfoWindowsE2EWin32+RECT
    Assert-E2E ([CodexInfoWindowsE2EWin32]::GetWindowRect($Handle, [ref]$rect)) 'GetWindowRect failed.'
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    Assert-E2E ($width -gt 0 -and $height -gt 0) "Invalid window bounds ${width}x${height}."
    return [pscustomobject]@{
        Left = $rect.Left
        Top = $rect.Top
        Width = $width
        Height = $height
    }
}

function Record-E2EWindow {
    param(
        [Parameter(Mandatory = $true)][string]$Role,
        [Parameter(Mandatory = $true)][int]$ExpectedProcessId,
        [Parameter(Mandatory = $true)][IntPtr]$Handle
    )

    [uint32]$owner = 0
    [CodexInfoWindowsE2EWin32]::GetWindowThreadProcessId($Handle, [ref]$owner) | Out-Null
    Assert-E2E ($owner -eq [uint32]$ExpectedProcessId) "$Role HWND is owned by PID $owner, expected $ExpectedProcessId."
    $bounds = Get-E2EWindowBounds $Handle
    $record = [pscustomobject]@{
        role = $Role
        pid = [int]$owner
        hwnd = ('0x{0:X}' -f $Handle.ToInt64())
        left = $bounds.Left
        top = $bounds.Top
        width = $bounds.Width
        height = $bounds.Height
        recorded_at = [DateTimeOffset]::Now.ToString('o')
    }
    $script:e2eWindowRecords.Add($record)
    Write-E2E ("window: role={0} pid={1} hwnd={2} bounds={3}x{4}+{5}+{6}" -f
        $Role, $owner, $record.hwnd, $bounds.Width, $bounds.Height, $bounds.Left, $bounds.Top)
    return $record
}

function Bring-E2EWindowToFront {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    [CodexInfoWindowsE2EWin32]::ShowWindow($Handle, 9) | Out-Null
    [CodexInfoWindowsE2EWin32]::BringWindowToTop($Handle) | Out-Null
    [CodexInfoWindowsE2EWin32]::SetForegroundWindow($Handle) | Out-Null
    Start-Sleep -Milliseconds 250
}

function Get-E2EUiaRoot {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    return [System.Windows.Automation.AutomationElement]::FromHandle($Handle)
}

function Get-E2EAllDescendants {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root)

    $all = [System.Collections.Generic.List[object]]::new()
    $all.Add($Root)
    $descendants = $Root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($element in $descendants) { $all.Add($element) }
    return $all
}

function Find-E2EElementByAutomationId {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][string]$AutomationId
    )

    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty,
        $AutomationId)
    return $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Find-E2EButtonByName {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][string]$Name
    )

    foreach ($element in Get-E2EAllDescendants $Root) {
        try {
            if ($element.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and
                [string]$element.Current.Name -eq $Name) {
                return $element
            }
        }
        catch { }
    }
    return $null
}

function Get-E2EControlElements {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][System.Windows.Automation.ControlType]$ControlType
    )

    $result = [System.Collections.Generic.List[object]]::new()
    foreach ($element in Get-E2EAllDescendants $Root) {
        try {
            if ($element.Current.ControlType -eq $ControlType) { $result.Add($element) }
        }
        catch { }
    }
    return $result
}

function Get-E2EVisibleControlElements {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][System.Windows.Automation.ControlType]$ControlType
    )

    return @(Get-E2EControlElements $Root $ControlType | Where-Object {
        try {
            $rectangle = $_.Current.BoundingRectangle
            -not $_.Current.IsOffscreen -and $_.Current.IsEnabled -and
                $rectangle.Width -gt 0 -and $rectangle.Height -gt 0
        }
        catch { $false }
    })
}

function Get-E2ETextValues {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root)

    $values = [System.Collections.Generic.List[string]]::new()
    foreach ($element in Get-E2EAllDescendants $Root) {
        try {
            $name = [string]$element.Current.Name
            if (-not [string]::IsNullOrWhiteSpace($name)) { $values.Add($name.Trim()) }
        }
        catch { }
        try {
            $valuePattern = $null
            if ($element.TryGetCurrentPattern(
                    [System.Windows.Automation.ValuePattern]::Pattern,
                    [ref]$valuePattern)) {
                $value = [string]$valuePattern.Current.Value
                if (-not [string]::IsNullOrWhiteSpace($value)) { $values.Add($value.Trim()) }
            }
        }
        catch { }
    }
    return @($values | Select-Object -Unique)
}

function Invoke-E2EElement {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Element)

    $pattern = $null
    Assert-E2E ($Element.TryGetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) 'Element has no InvokePattern.'
    $pattern.Invoke()
}

function Get-E2EToggleState {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Element)

    $pattern = $null
    Assert-E2E ($Element.TryGetCurrentPattern(
        [System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) 'Element has no TogglePattern.'
    return $pattern.Current.ToggleState
}

function Toggle-E2EElement {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Element)

    $pattern = $null
    Assert-E2E ($Element.TryGetCurrentPattern(
        [System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) 'Element has no TogglePattern.'
    $pattern.Toggle()
}

function Select-E2EListItem {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $items = Get-E2EVisibleControlElements $Root ([System.Windows.Automation.ControlType]::ListItem)
    $item = $items | Where-Object { [string]$_.Current.Name -eq $Label } | Select-Object -First 1
    Assert-E2E ($null -ne $item) "List item '$Label' is missing."
    $selection = $null
    if ($item.TryGetCurrentPattern(
            [System.Windows.Automation.SelectionItemPattern]::Pattern,
            [ref]$selection)) {
        $selection.Select()
        return
    }
    $invoke = $null
    Assert-E2E ($item.TryGetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern, [ref]$invoke)) "List item '$Label' is not selectable."
    $invoke.Invoke()
}

function Get-E2ESelectedListItemLabel {
    param([Parameter(Mandatory = $true)][object[]]$Items)

    foreach ($item in $Items) {
        try {
            $selection = $null
            if ($item.TryGetCurrentPattern(
                    [System.Windows.Automation.SelectionItemPattern]::Pattern,
                    [ref]$selection) -and $selection.Current.IsSelected) {
                $label = [string]$item.Current.Name
                if (-not [string]::IsNullOrWhiteSpace($label)) { return $label }
            }
        }
        catch { }
    }
    return ''
}

function Get-E2ESelectorLabel {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Selector)

    $values = @(Get-E2ETextValues $Selector)
    $filtered = $values | Where-Object {
        $_ -notin @(
            'Reset time', 'Model usage', 'Remaining quota',
            'Reset period', 'Dollars', 'Tokens')
    }
    if ($filtered.Count -gt 0) { return [string]$filtered[-1] }
    if ($values.Count -gt 0) { return [string]$values[-1] }
    return ''
}

function Wait-E2ESelectorLabel {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][string]$AutomationId,
        [Parameter(Mandatory = $true)][string]$Expected
    )

    Wait-E2E -Description "selector $AutomationId displays '$Expected'" -Probe {
        $selector = Find-E2EElementByAutomationId $Root $AutomationId
        if ($null -eq $selector) { return $false }
        $label = Get-E2ESelectorLabel $selector
        return $label -eq $Expected -or $label.Contains($Expected)
    } | Out-Null
}

function Capture-E2EWindow {
    param(
        [Parameter(Mandatory = $true)][IntPtr]$Handle,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $safeName = ($Name -replace '[^A-Za-z0-9_.-]', '_')
    $path = Join-Path $script:e2eOutput "$safeName.png"
    # UIA state is published before the compositor necessarily presents the
    # corresponding frame.  Allow one bounded paint interval before taking
    # the independent screen observation.
    Start-Sleep -Milliseconds 250
    $bounds = Get-E2EWindowBounds $Handle
    $bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-E2E "capture: name=$safeName path=$path sha256=$hash size=$($bounds.Width)x$($bounds.Height)"
    return [pscustomobject]@{ Path = $path; Hash = $hash }
}

function Assert-E2EImageChanged {
    param(
        [Parameter(Mandatory = $true)][psobject]$Before,
        [Parameter(Mandatory = $true)][psobject]$After,
        [Parameter(Mandatory = $true)][string]$Description
    )
    Assert-E2E ($Before.Hash -ne $After.Hash) "$Description did not change the rendered window."
}

function New-E2EFixtureDocuments {
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $currentStart = $now - 7200
    $currentReset = $now + 7200
    $pastStart = $now - 25200
    $pastReset = $now - 14400
    $status = [ordered]@{
        api_version = 'v1'
        state = 'ready'
        observed_at = $now
        authenticated = $true
        plan_label = 'Pro'
        quota = [ordered]@{ remaining_percent = 72.0; reset_at = $currentReset; window_seconds = 14400; monthly = $false }
        models = @(
            [ordered]@{ name = 'SOL'; input_tokens = 1200; cached_input_tokens = 200; output_tokens = 400 },
            [ordered]@{ name = 'TERRA'; input_tokens = 2400; cached_input_tokens = 500; output_tokens = 800 },
            [ordered]@{ name = 'LUNA'; input_tokens = 3600; cached_input_tokens = 700; output_tokens = 1100 }
        )
        active_thread_count = 3
    }
    $details = [ordered]@{
        api_version = 'v1'
        state = 'ready'
        observed_at = $now
        authenticated = $true
        plan_label = 'Pro'
        quota = $status.quota
        models = @(
            [ordered]@{ name = 'SOL'; input_tokens = 1200; cached_input_tokens = 200; output_tokens = 400; input_dollars = 1.20; cached_input_dollars = 0.20; output_dollars = 0.40 },
            [ordered]@{ name = 'TERRA'; input_tokens = 2400; cached_input_tokens = 500; output_tokens = 800; input_dollars = 2.40; cached_input_dollars = 0.50; output_dollars = 0.80 },
            [ordered]@{ name = 'LUNA'; input_tokens = 3600; cached_input_tokens = 700; output_tokens = 1100; input_dollars = 3.60; cached_input_dollars = 0.70; output_dollars = 1.10 }
        )
        active_thread_count = 3
        history_periods = @(
            [ordered]@{ id = 'e2e-current'; start_at = $currentStart; end_at = $now; reset_at = $currentReset; label = 'Current period'; current = $true },
            [ordered]@{ id = 'e2e-past'; start_at = $pastStart; end_at = $pastReset; reset_at = $pastReset; label = 'Past period'; current = $false }
        )
        history_samples = @(
            [ordered]@{ timestamp = $currentStart + 60; reset_at = $currentReset; remaining_percent = 92.0; sol_dollars = 0.25; terra_dollars = 0.50; luna_dollars = 0.75; sol_tokens = 100; terra_tokens = 200; luna_tokens = 300 },
            [ordered]@{ timestamp = $now - 60; reset_at = $currentReset; remaining_percent = 72.0; sol_dollars = 1.20; terra_dollars = 2.40; luna_dollars = 3.60; sol_tokens = 1200; terra_tokens = 2400; luna_tokens = 3600 },
            [ordered]@{ timestamp = $pastStart + 60; reset_at = $pastReset; remaining_percent = 98.0; sol_dollars = 0.10; terra_dollars = 0.20; luna_dollars = 0.30; sol_tokens = 50; terra_tokens = 100; luna_tokens = 150 },
            [ordered]@{ timestamp = $pastReset - 60; reset_at = $pastReset; remaining_percent = 84.0; sol_dollars = 0.60; terra_dollars = 1.20; luna_dollars = 1.80; sol_tokens = 600; terra_tokens = 1200; luna_tokens = 1800 }
        )
        threads = @(
            [ordered]@{ id = 'e2e-root'; title = 'E2E root task'; parent_thread_id = $null; model = 'TERRA'; model_label = 'TERRA'; total_tokens = 2400; context_usage_tokens = 800; context_window_tokens = 16000; created_at = $now - 3600; last_user_message_at = $now - 300; is_subagent = $false; depth = 0 },
            [ordered]@{ id = 'e2e-child'; title = 'E2E child task'; parent_thread_id = 'e2e-root'; model = 'LUNA'; model_label = 'LUNA'; total_tokens = 1200; context_usage_tokens = 400; context_window_tokens = 16000; created_at = $now - 2400; last_user_message_at = $now - 600; is_subagent = $true; depth = 1 },
            [ordered]@{ id = 'e2e-orphan'; title = 'E2E orphan task'; parent_thread_id = 'missing-parent'; model = 'SOL'; model_label = 'SOL'; total_tokens = 600; context_usage_tokens = $null; context_window_tokens = $null; created_at = $now - 1200; last_user_message_at = $null; is_subagent = $true; depth = $null }
        )
        estimated_cost_label = '$12.34'
    }
    return [pscustomobject]@{
        Status = ($status | ConvertTo-Json -Compress -Depth 12)
        Details = ($details | ConvertTo-Json -Compress -Depth 12)
        Now = $now
    }
}

function Enter-E2EFixture {
    $documents = New-E2EFixtureDocuments
    if (Test-Path -LiteralPath $script:e2eSettingsPath -PathType Leaf) {
        $script:e2eSettingsWasPresent = $true
        Copy-Item -LiteralPath $script:e2eSettingsPath -Destination $script:e2eSettingsBackup -Force
    }
    $settingsDirectory = Split-Path -Parent $script:e2eSettingsPath
    New-Item -ItemType Directory -Path $settingsDirectory -Force | Out-Null
    $settingsJson = '{"language":"en","setupCompleted":true,"connectionConfigured":true,"timeZoneId":"UTC","connectionProfile":"none","connectionSelector":"none"}'
    [IO.File]::WriteAllText($script:e2eSettingsPath, $settingsJson, [Text.UTF8Encoding]::new($false))
    Assert-E2E ([CodexInfoWindowsE2EFixtureServer]::Start($documents.Status, $documents.Details, 8787)) 'Could not bind the fixture to loopback port 8787.'
    $script:e2eFixtureRunning = $true
    Write-E2E "fixture: PASS periods=2 threads=3 endpoint=http://127.0.0.1:8787"
}

function Exit-E2EFixture {
    if ($script:e2eFixtureRunning) {
        [CodexInfoWindowsE2EFixtureServer]::Stop()
        $script:e2eFixtureRunning = $false
    }
    if ($script:e2eSettingsWasPresent) {
        Copy-Item -LiteralPath $script:e2eSettingsBackup -Destination $script:e2eSettingsPath -Force
    }
    elseif (Test-Path -LiteralPath $script:e2eSettingsPath -PathType Leaf) {
        Remove-Item -LiteralPath $script:e2eSettingsPath -Force
    }
    if (Test-Path -LiteralPath $script:e2eSettingsBackup -PathType Leaf) {
        Remove-Item -LiteralPath $script:e2eSettingsBackup -Force
    }
}

function Open-E2EChildWindow {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$MainRoot,
        [Parameter(Mandatory = $true)][string]$ButtonName,
        [string]$ButtonAutomationId = '',
        [Parameter(Mandatory = $true)][string]$Title,
        [Parameter(Mandatory = $true)][string]$Role,
        [Parameter(Mandatory = $true)][int]$ProcessId
    )

    $button = Wait-E2E -Description "main button '$ButtonName'" -Probe {
        $candidate = if ([string]::IsNullOrWhiteSpace($ButtonAutomationId)) {
            Find-E2EButtonByName $MainRoot $ButtonName
        }
        else {
            Find-E2EElementByAutomationId $MainRoot $ButtonAutomationId
        }
        if ($null -ne $candidate -and
            $candidate.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and
            $candidate.Current.IsEnabled) { return $candidate }
        # Keep a localized real-data run usable when an older installed build
        # predates the navigation AutomationId attributes.
        if (-not [string]::IsNullOrWhiteSpace($ButtonAutomationId)) {
            $localized = Find-E2EButtonByName $MainRoot $ButtonName
            if ($null -ne $localized -and $localized.Current.IsEnabled) { return $localized }
        }
        return $false
    }
    Invoke-E2EElement $button
    $handle = Wait-E2E -Description "$Title window" -Probe {
        $candidate = Find-E2EWindow $ProcessId $Title
        if ($candidate -eq [IntPtr]::Zero) { return $false }
        return $candidate
    }
    Bring-E2EWindowToFront $handle
    $record = Record-E2EWindow $Role $ProcessId $handle
    return [pscustomobject]@{
        Handle = $handle
        Root = Get-E2EUiaRoot $handle
        Record = $record
    }
}

function Find-E2ECloseButton {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root)

    $named = @('Close')
    $buttons = @(Get-E2EControlElements $Root ([System.Windows.Automation.ControlType]::Button))
    foreach ($button in $buttons) {
        try {
            if ($named -contains [string]$button.Current.Name) { return $button }
        }
        catch { }
    }
    # The product's borderless windows put close at the right edge of the
    # title row. This bounded geometry fallback is locale-independent.
    $windowBounds = Get-E2EWindowBounds ([IntPtr]$Root.Current.NativeWindowHandle)
    $candidates = @($buttons | Where-Object {
        $rect = $_.Current.BoundingRectangle
        $rect.Right -ge ($windowBounds.Left + $windowBounds.Width - 80) -and
            $rect.Top -le ($windowBounds.Top + 90)
    })
    return $candidates | Sort-Object { $_.Current.BoundingRectangle.Right } -Descending | Select-Object -First 1
}

try {
    $resolvedClientPath = if ([string]::IsNullOrWhiteSpace($ClientPath)) {
        Join-Path $env:LOCALAPPDATA 'Programs\Codex Info Monitor\CodexInfo.WindowsClient.exe'
    }
    else { [IO.Path]::GetFullPath($ClientPath) }
    Assert-E2E (Test-Path -LiteralPath $resolvedClientPath -PathType Leaf) "Installed client not found: $resolvedClientPath"
    Write-E2E "start: client=$resolvedClientPath fixture=$Fixture output=$script:e2eOutput"

    if ($Fixture) { Enter-E2EFixture }
    $script:e2eProcess = Start-Process -FilePath $resolvedClientPath -PassThru
    $clientPid = $script:e2eProcess.Id
    Write-E2E "process: pid=$clientPid"

    $mainHandle = Wait-E2E -Description 'Main window' -Probe {
        $candidate = Find-E2EWindow $clientPid 'Codex Info Monitor'
        if ($candidate -eq [IntPtr]::Zero) { return $false }
        return $candidate
    }
    Bring-E2EWindowToFront $mainHandle
    $mainRecord = Record-E2EWindow 'Main' $clientPid $mainHandle
    $mainRoot = Get-E2EUiaRoot $mainHandle
    $mainCapture = Capture-E2EWindow $mainHandle '01-main-ready'
    Assert-E2E ($mainCapture.Hash.Length -eq 64) 'Main screenshot hash is missing.'

    # Finite path: one Graph window, one period round-trip, two metrics, then
    # one OFF/ON cycle for each of four independent series.  No combinations
    # of these controls are generated.
    Write-E2E 'case-1: open Graph'
    $graph = Open-E2EChildWindow -MainRoot $mainRoot -ButtonName 'Graph' -ButtonAutomationId 'Main.OpenGraph' -Title 'Codex Info Graph' -Role 'Graph' -ProcessId $clientPid
    $graphRoot = $graph.Root
    $plot = Wait-E2E -Description 'Graph plot' -Probe {
        $candidate = Find-E2EElementByAutomationId $graphRoot 'Graph.Plot'
        if ($null -eq $candidate) { return $false }
        $rect = $candidate.Current.BoundingRectangle
        if ($candidate.Current.IsOffscreen -or $rect.Width -le 0 -or $rect.Height -le 0) { return $false }
        return $candidate
    }
    Write-E2E ("graph: plot bounds={0}x{1}" -f $plot.Current.BoundingRectangle.Width, $plot.Current.BoundingRectangle.Height)
    $periodSelector = Find-E2EElementByAutomationId $graphRoot 'Graph.PeriodSelector'
    $metricSelector = Find-E2EElementByAutomationId $graphRoot 'Graph.MetricSelector'
    Assert-E2E ($null -ne $periodSelector -and $null -ne $metricSelector) 'Graph selectors are missing.'
    $currentLabel = Get-E2ESelectorLabel $periodSelector
    $graphCurrent = Capture-E2EWindow $graph.Handle '02-graph-current'

    Write-E2E 'case-2: period current -> past -> current and display-value assertions'
    Toggle-E2EElement $periodSelector
    $graphBounds = Get-E2EWindowBounds $graph.Handle
    $periodMenuRightBoundary = $graphBounds.Left + $graphBounds.Width - 200
    $periodItems = Wait-E2E -Description 'two Graph period options' -Probe {
        $items = @(Get-E2EVisibleControlElements $graphRoot ([System.Windows.Automation.ControlType]::ListItem) | Where-Object {
            $_.Current.BoundingRectangle.Left -lt $periodMenuRightBoundary
        })
        if ($items.Count -ge 2) { return $items }
        return $false
    }
    $periodLabels = @($periodItems | ForEach-Object { [string]$_.Current.Name } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique)
    Assert-E2E ($periodLabels.Count -ge 2) "Graph period menu exposes fewer than two values: $($periodLabels -join ', ')."
    $selectedPeriodLabel = Get-E2ESelectedListItemLabel $periodItems
    if (-not [string]::IsNullOrWhiteSpace($selectedPeriodLabel)) {
        $currentLabel = $selectedPeriodLabel
    }
    Assert-E2E ($periodLabels -contains $currentLabel) 'Current period display value is not represented by a selected menu item.'
    $pastLabel = [string]($periodLabels | Where-Object { $_ -ne $currentLabel } | Select-Object -First 1)
    Assert-E2E (-not [string]::IsNullOrWhiteSpace($pastLabel)) 'Past period option is missing.'
    Select-E2EListItem $graphRoot $pastLabel
    Wait-E2ESelectorLabel $graphRoot 'Graph.PeriodSelector' $pastLabel
    $graphPast = Capture-E2EWindow $graph.Handle '03-graph-past'
    Assert-E2EImageChanged $graphCurrent $graphPast 'Current-to-past period selection'

    $periodSelector = Find-E2EElementByAutomationId $graphRoot 'Graph.PeriodSelector'
    Toggle-E2EElement $periodSelector
    Select-E2EListItem $graphRoot $currentLabel
    Wait-E2ESelectorLabel $graphRoot 'Graph.PeriodSelector' $currentLabel
    $graphCurrentAgain = Capture-E2EWindow $graph.Handle '04-graph-current-again'
    Assert-E2EImageChanged $graphPast $graphCurrentAgain 'Past-to-current period selection'

    Write-E2E 'case-3: select both metric values'
    $metricSelector = Find-E2EElementByAutomationId $graphRoot 'Graph.MetricSelector'
    $initialMetric = Get-E2ESelectorLabel $metricSelector
    Toggle-E2EElement $metricSelector
    $metricMenuLeftBoundary = $graphBounds.Left + $graphBounds.Width - 200
    $metricItems = Wait-E2E -Description 'two Graph metric options' -Probe {
        $items = @(Get-E2EVisibleControlElements $graphRoot ([System.Windows.Automation.ControlType]::ListItem) | Where-Object {
            $_.Current.BoundingRectangle.Left -ge $metricMenuLeftBoundary
        })
        if ($items.Count -ge 2) { return $items }
        return $false
    }
    $metricLabels = @($metricItems | ForEach-Object { [string]$_.Current.Name } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique)
    Assert-E2E ($metricLabels.Count -eq 2) "Metric menu must expose exactly two values: $($metricLabels -join ', ')."
    $selectedMetricLabel = Get-E2ESelectedListItemLabel $metricItems
    if (-not [string]::IsNullOrWhiteSpace($selectedMetricLabel)) {
        $initialMetric = $selectedMetricLabel
    }
    Assert-E2E ($metricLabels -contains $initialMetric) 'Initial metric display value is not represented by a selected menu item.'
    $otherMetric = [string]($metricLabels | Where-Object { $_ -ne $initialMetric } | Select-Object -First 1)
    Assert-E2E (-not [string]::IsNullOrWhiteSpace($otherMetric)) 'Second metric option is missing.'
    Select-E2EListItem $graphRoot $otherMetric
    Wait-E2ESelectorLabel $graphRoot 'Graph.MetricSelector' $otherMetric
    Wait-E2E -Description "axis for metric '$otherMetric'" -Probe {
        $texts = Get-E2ETextValues $graphRoot
        if ($texts -contains $otherMetric -or ($texts -join ' ') -like "*$otherMetric*") { return $true }
        return $false
    } | Out-Null
    $graphOtherMetric = Capture-E2EWindow $graph.Handle '05-graph-other-metric'
    Assert-E2EImageChanged $graphCurrentAgain $graphOtherMetric "Metric selection '$otherMetric'"

    $metricSelector = Find-E2EElementByAutomationId $graphRoot 'Graph.MetricSelector'
    Toggle-E2EElement $metricSelector
    Select-E2EListItem $graphRoot $initialMetric
    Wait-E2ESelectorLabel $graphRoot 'Graph.MetricSelector' $initialMetric
    $graphInitialMetricAgain = Capture-E2EWindow $graph.Handle '06-graph-initial-metric'
    Assert-E2EImageChanged $graphOtherMetric $graphInitialMetricAgain "Metric selection '$initialMetric'"

    Write-E2E 'case-4: each series toggle OFF then ON exactly once'
    $toggleCases = @(
        @{ Id = 'Graph.Toggle.Remaining'; Name = 'Remaining' },
        @{ Id = 'Graph.Toggle.LUNA'; Name = 'LUNA' },
        @{ Id = 'Graph.Toggle.TERRA'; Name = 'TERRA' },
        @{ Id = 'Graph.Toggle.SOL'; Name = 'SOL' }
    )
    $toggleIndex = 0
    foreach ($toggleCase in $toggleCases) {
        $toggleIndex++
        $toggle = Find-E2EElementByAutomationId $graphRoot $toggleCase.Id
        Assert-E2E ($null -ne $toggle) "Graph toggle is missing: $($toggleCase.Id)"
        $initialState = Get-E2EToggleState $toggle
        Assert-E2E ($initialState -eq [System.Windows.Automation.ToggleState]::On) "$($toggleCase.Name) is not initially ON."
        $beforeOff = Capture-E2EWindow $graph.Handle ("07-toggle-{0}-before" -f $toggleCase.Name)
        Toggle-E2EElement $toggle
        Wait-E2E -Description "$($toggleCase.Name) OFF" -Probe {
            $candidate = Find-E2EElementByAutomationId $graphRoot $toggleCase.Id
            if ($null -ne $candidate -and (Get-E2EToggleState $candidate) -eq [System.Windows.Automation.ToggleState]::Off) { return $true }
            return $false
        } | Out-Null
        $off = Capture-E2EWindow $graph.Handle ("08-toggle-{0}-off" -f $toggleCase.Name)
        Assert-E2EImageChanged $beforeOff $off "$($toggleCase.Name) OFF render"
        Toggle-E2EElement (Find-E2EElementByAutomationId $graphRoot $toggleCase.Id)
        Wait-E2E -Description "$($toggleCase.Name) ON" -Probe {
            $candidate = Find-E2EElementByAutomationId $graphRoot $toggleCase.Id
            if ($null -ne $candidate -and (Get-E2EToggleState $candidate) -eq [System.Windows.Automation.ToggleState]::On) { return $true }
            return $false
        } | Out-Null
        $on = Capture-E2EWindow $graph.Handle ("09-toggle-{0}-on" -f $toggleCase.Name)
        Assert-E2EImageChanged $off $on "$($toggleCase.Name) ON render"
        Write-E2E "toggle: name=$($toggleCase.Name) off=Off on=On cycle=$toggleIndex"
    }

    $closeGraph = Find-E2ECloseButton $graphRoot
    Assert-E2E ($null -ne $closeGraph) 'Graph Close button is missing.'
    Invoke-E2EElement $closeGraph
    Wait-E2E -Description 'Graph window close' -Probe {
        return (Find-E2EWindow $clientPid 'Codex Info Graph') -eq [IntPtr]::Zero
    } | Out-Null

    Write-E2E 'case-5: open Threads and assert root/child/orphan rows and columns'
    $threads = Open-E2EChildWindow -MainRoot $mainRoot -ButtonName 'Threads' -ButtonAutomationId 'Main.OpenThreads' -Title 'Codex Info Threads' -Role 'Threads' -ProcessId $clientPid
    $threadsRoot = $threads.Root
    $threadTexts = Wait-E2E -Description 'Threads rows' -Probe {
        $values = @(Get-E2ETextValues $threadsRoot)
        if ($values.Count -ge 8) { return $values }
        return $false
    }
    $fixtureRows = @(
        @{ Title = 'E2E root task'; Id = 'e2e-root'; Model = 'TERRA'; Column = 'Depth 0' },
        @{ Title = 'E2E child task'; Id = 'e2e-child'; Model = 'LUNA'; Column = 'Depth 1' },
        @{ Title = 'E2E orphan task'; Id = 'e2e-orphan'; Model = 'SOL'; Column = 'missing-parent' }
    )
    if ($Fixture) {
        foreach ($row in $fixtureRows) {
            Assert-E2E (@($threadTexts | Where-Object { $_ -eq $row.Title }).Count -gt 0) "Threads row title missing: $($row.Title)"
            Assert-E2E (@($threadTexts | Where-Object { $_ -eq $row.Id }).Count -gt 0) "Threads ID column missing: $($row.Id)"
            Assert-E2E (@($threadTexts | Where-Object { $_ -eq $row.Model }).Count -gt 0) "Threads model column missing: $($row.Model)"
            Assert-E2E (@($threadTexts | Where-Object { $_ -like "*$($row.Column)*" }).Count -gt 0) "Threads metadata column missing: $($row.Column)"
        }
        Assert-E2E (@($threadTexts | Where-Object { $_ -like '*Parent: e2e-root*' }).Count -gt 0) 'Child parent column is missing.'
        Assert-E2E (@($threadTexts | Where-Object { $_ -like '*missing-parent*' }).Count -gt 0) 'Orphan parent column is missing.'
    }
    else {
        # Real data mode accepts the server's row identities, but still
        # requires a row container with several visible cells (title, id,
        # model and metadata). An empty or status-only window cannot pass.
        $threadRows = @(Get-E2EControlElements $threadsRoot ([System.Windows.Automation.ControlType]::ListItem))
        if ($threadRows.Count -gt 0) {
            $richRows = @($threadRows | Where-Object { @(Get-E2ETextValues $_).Count -ge 4 })
            Assert-E2E ($richRows.Count -gt 0) 'Threads rows did not expose representative columns.'
        }
        else {
            # Avalonia versions differ in whether ItemsControl containers are
            # surfaced as ListItem. Keep the observable fallback bounded while
            # still requiring multiple non-empty cell values.
            $nonEmpty = @($threadTexts | Where-Object { $_.Length -ge 2 })
            Assert-E2E ($nonEmpty.Count -ge 4) 'Threads did not expose a real data row and columns.'
        }
    }
    $threadCapture = Capture-E2EWindow $threads.Handle '10-threads-rows'
    Assert-E2E ($threadCapture.Hash.Length -eq 64) 'Threads screenshot hash is missing.'

    Write-E2E 'case-6: same PID and HWND records'
    $allPids = @($script:e2eWindowRecords | ForEach-Object { $_.pid } | Select-Object -Unique)
    Assert-E2E ($allPids.Count -eq 1 -and $allPids[0] -eq $clientPid) "Window PID set is not singleton: $($allPids -join ',')."
    $allHwnds = @($script:e2eWindowRecords | ForEach-Object { $_.hwnd } | Select-Object -Unique)
    Assert-E2E ($allHwnds.Count -eq $script:e2eWindowRecords.Count) 'Window HWND records are not unique.'
    $windowRecordPath = Join-Path $script:e2eOutput 'window-records.json'
    $script:e2eWindowRecords | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $windowRecordPath -Encoding utf8
    Write-E2E "windows: PASS records=$($script:e2eWindowRecords.Count) pid=$clientPid records_path=$windowRecordPath"

    Write-E2E 'windows-client-e2e: PASS (Graph open, period current/past/current, 2 metrics, 4 toggle OFF/ON cycles, Threads rows/columns, PID/HWND records)'
    $script:e2eSuccess = $true
}
catch {
    Write-E2E "windows-client-e2e: FAIL $($_.Exception.Message)"
    throw
}
finally {
    if ($null -ne $script:e2eProcess) {
        try {
            if (-not $script:e2eProcess.HasExited) {
                Stop-Process -Id $script:e2eProcess.Id -Force -ErrorAction SilentlyContinue
            }
        }
        catch { }
    }
    if ($Fixture) {
        try { Exit-E2EFixture } catch { Write-E2E "fixture-cleanup: FAIL $($_.Exception.Message)" }
    }
}

# A successful script invocation returns naturally.  Failures are thrown from
# the acceptance assertions above, so callers cannot mistake a SKIP for PASS.

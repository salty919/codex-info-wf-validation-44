# Runs the finite Windows UI Automation acceptance path against the installed
# client.  The normal mode uses the configured loopback service.  CI may pass
# -Fixture to provide a bounded, local /v1/status + /v1/details pair; this still
# drives the installed EXE and the real rendered windows, but does not require
# an account or an SSH tunnel.
[CmdletBinding()]
param(
    [string]$ClientPath = '',
    [string]$OutputDirectory = '',
    [switch]$Fixture,
    [string]$SourceSha = ''
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
if (Test-Path -LiteralPath $script:e2eOutput -PathType Container) {
    # Evidence must belong to this invocation.  Never append to or reuse a
    # prior run's log/screenshots, even when a hosted runner reuses its temp
    # directory after a retry.
    Remove-Item -LiteralPath $script:e2eOutput -Recurse -Force
}
New-Item -ItemType Directory -Path $script:e2eOutput -Force | Out-Null
$script:e2eLogPath = Join-Path $script:e2eOutput 'windows-client-e2e.log'
$script:e2eSourceSha = if (-not [string]::IsNullOrWhiteSpace($SourceSha)) { $SourceSha } elseif (-not [string]::IsNullOrWhiteSpace($env:GITHUB_SHA)) { $env:GITHUB_SHA } else { 'unknown' }
$script:e2eWindowRecords = [System.Collections.Generic.List[object]]::new()
$script:e2eProcess = $null
$script:e2eFixtureRunning = $false
$script:e2ePreviewEnabled = -not [string]::IsNullOrWhiteSpace($env:CODEX_INFO_WINDOWS_PREVIEW)
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
    private static int healthRequests;
    private static int statusRequests;
    private static int detailsRequests;

    public static bool Start(string status, string details, int port) {
        if (running) return false;
        try {
            statusBody = status;
            detailsBody = details;
            healthRequests = 0;
            statusRequests = 0;
            detailsRequests = 0;
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

    public static string RequestSummary() {
        return String.Format(
            "health={0} status={1} details={2}", healthRequests, statusRequests, detailsRequests);
    }

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
                    Interlocked.Increment(ref statusRequests);
                    code = 200;
                    reason = "OK";
                    body = statusBody;
                }
                else if (parts[1] == "/v1/health") {
                    Interlocked.Increment(ref healthRequests);
                    code = 200;
                    reason = "OK";
                    body = "{\"api_version\":\"v1\",\"service\":\"codex-info\"}";
                }
                else if (parts[1] == "/v1/details") {
                    Interlocked.Increment(ref detailsRequests);
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

function Get-E2EElementsByAutomationId {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][string]$AutomationId
    )

    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty,
        $AutomationId)
    return @($Root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition))
}

function Assert-E2EMainProductVersion {
    param([Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root)

    $versions = @(Get-E2EElementsByAutomationId $Root 'Main.ProductVersion')
    Assert-E2E ($versions.Count -eq 1) "Main product version must have exactly one UIA element, found $($versions.Count)."
    $value = [string]$versions[0].Current.Name
    Assert-E2E ($value -match '^v[0-9]+\.[0-9]+\.[0-9]+$') "Main product version is malformed: '$value'."
    Write-E2E "main-product-version: PASS value=$value count=$($versions.Count)"
}

function Assert-E2ENoChildProductVersion {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][string]$Role
    )

    $versions = @(Get-E2EElementsByAutomationId $Root 'Main.ProductVersion')
    Assert-E2E ($versions.Count -eq 0) "$Role child window must not expose Main.ProductVersion, found $($versions.Count)."
    Write-E2E "child-product-version: PASS role=$Role count=$($versions.Count)"
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

    $helpText = [string]$Selector.Current.HelpText
    if (-not [string]::IsNullOrWhiteSpace($helpText)) {
        return $helpText.Trim()
    }

    $values = @(Get-E2ETextValues $Selector)
    # The selector's own accessible name is a localized field label
    # (for example "Reset time"), while its child text is the selected
    # value. Exclude the root name dynamically instead of maintaining an
    # English-only list that makes a Japanese installed run time out.
    $rootName = [string]$Selector.Current.Name
    $filtered = $values | Where-Object {
        $_ -ne $rootName -and $_ -notin @(
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

    $script:e2eLastSelectorLabel = ''
    try {
        Wait-E2E -Description "selector $AutomationId displays '$Expected'" -Probe {
            $selector = Find-E2EElementByAutomationId $Root $AutomationId
            if ($null -eq $selector) { return $false }
            $script:e2eLastSelectorLabel = Get-E2ESelectorLabel $selector
            return $script:e2eLastSelectorLabel -eq $Expected -or $script:e2eLastSelectorLabel.Contains($Expected)
        } | Out-Null
    }
    catch {
        Write-E2E "selector: FAIL id=$AutomationId expected='$Expected' observed='$script:e2eLastSelectorLabel'"
        throw
    }
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

function Assert-E2EGraphHasModelData {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Plot,
        [Parameter(Mandatory = $true)][IntPtr]$Handle,
        [Parameter(Mandatory = $true)][psobject]$Capture
    )

    $window = Get-E2EWindowBounds $Handle
    $plotBounds = $Plot.Current.BoundingRectangle
    $bitmap = [System.Drawing.Bitmap]::FromFile($Capture.Path)
    $series = @(
        @{ Name = 'LUNA'; Color = [System.Drawing.ColorTranslator]::FromHtml('#E6A23C'); FlatColor = [System.Drawing.ColorTranslator]::FromHtml('#7B5E31') },
        @{ Name = 'TERRA'; Color = [System.Drawing.ColorTranslator]::FromHtml('#5DC98A'); FlatColor = [System.Drawing.ColorTranslator]::FromHtml('#377158') },
        @{ Name = 'SOL'; Color = [System.Drawing.ColorTranslator]::FromHtml('#A88CF5'); FlatColor = [System.Drawing.ColorTranslator]::FromHtml('#5C538D') }
    )
    $hits = @{}
    $columns = @{}
    $spans = @{}
    foreach ($item in $series) {
        $hits[$item.Name] = 0
        $columns[$item.Name] = @{}
    }
    try {
        [int]$left = [Math]::Max(0, [int]($plotBounds.Left - $window.Left))
        [int]$top = [Math]::Max(0, [int]($plotBounds.Top - $window.Top))
        [int]$right = [Math]::Min($bitmap.Width, [int]($plotBounds.Right - $window.Left))
        [int]$bottom = [Math]::Min($bitmap.Height, [int]($plotBounds.Bottom - $window.Top))
        Assert-E2E ($right -gt $left -and $bottom -gt $top) 'Graph plot bounds are outside the captured window.'
        for ($x = $left; $x -lt $right; $x++) {
            for ($y = $top; $y -lt $bottom; $y++) {
                $pixel = $bitmap.GetPixel($x, $y)
                foreach ($item in $series) {
                    $dr = $pixel.R - $item.Color.R
                    $dg = $pixel.G - $item.Color.G
                    $db = $pixel.B - $item.Color.B
                    $flatDr = $pixel.R - $item.FlatColor.R
                    $flatDg = $pixel.G - $item.FlatColor.G
                    $flatDb = $pixel.B - $item.FlatColor.B
                    if ((($dr * $dr) + ($dg * $dg) + ($db * $db) -le 24 * 24) -or
                        (($flatDr * $flatDr) + ($flatDg * $flatDg) + ($flatDb * $flatDb) -le 24 * 24)) {
                        $hits[$item.Name] = $hits[$item.Name] + 1
                        if (-not $columns[$item.Name].ContainsKey($x)) {
                            $columns[$item.Name][$x] = 0
                        }
                        $columns[$item.Name][$x]++
                    }
                }
            }
        }
    }
    finally {
        $bitmap.Dispose()
    }
    foreach ($item in $series) {
        $seriesColumns = @($columns[$item.Name].Keys | ForEach-Object { [int]$_ } | Sort-Object)
        Assert-E2E ($hits[$item.Name] -gt 0) "Past graph has no rendered $($item.Name) model series pixels."
        # The fixture's three cumulative observations span the period.  A
        # single endpoint label or an accidental vertical stroke must not be
        # sufficient evidence that the historical model line rendered.
        $minimumColumns = [Math]::Max(20, [int][Math]::Ceiling(($right - $left) * 0.15))
        Assert-E2E ($seriesColumns.Count -ge $minimumColumns) `
            "Past graph $($item.Name) model series has too little horizontal span: columns=$($seriesColumns.Count), expected>=$minimumColumns."
        $span = $seriesColumns[-1] - $seriesColumns[0]
        $spans[$item.Name] = $span
        Assert-E2E ($span -ge [int][Math]::Ceiling(($right - $left) * 0.15)) `
            "Past graph $($item.Name) model series is concentrated at one x position: span=$span."
        # A legitimate observation after an unobserved gap is rendered as a
        # vertical step at that observation timestamp.  The forbidden case is
        # the synthetic reset jump at the plot's left edge; check that boundary
        # specifically instead of rejecting valid late observations.
        $leftBoundaryLimit = $left + [int][Math]::Ceiling(($right - $left) * 0.05)
        $leftBoundaryPixels = @($columns[$item.Name].GetEnumerator() |
            Where-Object { [int]$_.Key -le $leftBoundaryLimit } |
            ForEach-Object { [int]$_.Value } | Measure-Object -Maximum).Maximum
        if ($null -eq $leftBoundaryPixels) { $leftBoundaryPixels = 0 }
        Assert-E2E ($leftBoundaryPixels -le [int][Math]::Ceiling(($bottom - $top) * 0.20)) `
            "Past graph $($item.Name) model series contains a synthetic reset stroke at the left boundary: pixels=$leftBoundaryPixels."
    }
    Write-E2E ("graph-past-model-data: PASS LUNA={0}({1}px span) TERRA={2}({3}px span) SOL={4}({5}px span)" -f `
        $hits['LUNA'], $spans['LUNA'], $hits['TERRA'], $spans['TERRA'], $hits['SOL'], $spans['SOL'])
}

function Assert-E2EGraphHasIdleBand {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Plot,
        [Parameter(Mandatory = $true)][IntPtr]$Handle,
        [Parameter(Mandatory = $true)][psobject]$Capture,
        [double]$ExpectedStartFraction = 0.01,
        [double]$ExpectedEndFraction = 0.35
    )

    $window = Get-E2EWindowBounds $Handle
    $plotBounds = $Plot.Current.BoundingRectangle
    $bitmap = [System.Drawing.Bitmap]::FromFile($Capture.Path)
    $hits = 0
    $columnHits = @{}
    try {
        [int]$left = [Math]::Max(0, [int]($plotBounds.Left - $window.Left))
        [int]$top = [Math]::Max(0, [int]($plotBounds.Top - $window.Top))
        [int]$right = [Math]::Min($bitmap.Width, [int]($plotBounds.Right - $window.Left))
        [int]$bottom = [Math]::Min($bitmap.Height, [int]($plotBounds.Bottom - $window.Top))
        Assert-E2E ($right -gt $left -and $bottom -gt $top) 'Graph plot bounds are outside the captured window.'
        Assert-E2E ($ExpectedStartFraction -ge 0 -and $ExpectedEndFraction -le 1 -and
            $ExpectedEndFraction -gt $ExpectedStartFraction) 'Idle-band expected range is invalid.'
        [int]$expectedLeft = $left + [int](($right - $left) * $ExpectedStartFraction)
        [int]$expectedRight = $left + [int](($right - $left) * $ExpectedEndFraction)
        Assert-E2E ($expectedRight -gt $expectedLeft) 'Idle-band expected range is sub-pixel.'
        # #3F5D7C at opacity .22 over #101925 composites near #1A2838.
        # The bounded tolerance accepts compositor rounding while excluding
        # the plot surface (#101925) and grid (#263548).
        for ($x = $expectedLeft; $x -lt $expectedRight; $x++) {
            $columnHits[$x] = 0
            for ($y = $top + 20; $y -lt ($bottom - 20); $y++) {
                $pixel = $bitmap.GetPixel($x, $y)
                if ($pixel.R -ge 20 -and $pixel.R -le 34 -and
                    $pixel.G -ge 32 -and $pixel.G -le 48 -and
                    $pixel.B -ge 48 -and $pixel.B -le 64) {
                    $hits = $hits + 1
                    $columnHits[$x] = $columnHits[$x] + 1
                }
            }
        }
    }
    finally {
        $bitmap.Dispose()
    }
    $sampleColumns = @(
        $expectedLeft + [int](($expectedRight - $expectedLeft) * 0.25),
        $expectedLeft + [int](($expectedRight - $expectedLeft) * 0.50),
        $expectedLeft + [int](($expectedRight - $expectedLeft) * 0.75)
    ) | Select-Object -Unique
    foreach ($column in $sampleColumns) {
        $columnHitCount = [int]($columnHits[$column])
        Assert-E2E ($columnHitCount -ge 20) "Past graph idle-band color is missing at expected x=$column (hits=$columnHitCount)."
    }
    Assert-E2E ($hits -ge 100) "Past graph has no visible idle-band color pixels in the expected interval (hits=$hits)."
    Write-E2E "graph-past-idle-band: PASS pixels=$hits range=$ExpectedStartFraction-$ExpectedEndFraction color=#3F5D7C opacity=0.22"
}

function Assert-E2EQuotaGaugePalette {
    param(
        [Parameter(Mandatory = $true)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory = $true)][IntPtr]$Handle,
        [Parameter(Mandatory = $true)][psobject]$Capture
    )

    $gauge = Find-E2EElementByAutomationId $Root 'Main.QuotaPeriodGauge'
    Assert-E2E ($null -ne $gauge) 'Main quota period gauge is missing.'
    $rectangle = $gauge.Current.BoundingRectangle
    Assert-E2E (-not $gauge.Current.IsOffscreen -and $rectangle.Width -gt 70 -and $rectangle.Height -ge 8) `
        'Main quota period gauge has invalid rendered bounds.'
    $window = Get-E2EWindowBounds $Handle
    $cellWidth = $rectangle.Width / 7.0
    $sampleY = [int][Math]::Floor($rectangle.Top - $window.Top + $rectangle.Height / 2.0)
    # The fixture has one half of its reset window remaining.  These samples
    # cover full, fractional, and empty cells away from the one-pixel boundary.
    $samples = @(
        @{ name = 'full-1'; cell = 0; fraction = 0.50; expected = '#56B2F5' },
        @{ name = 'full-3'; cell = 2; fraction = 0.50; expected = '#56B2F5' },
        @{ name = 'partial-filled'; cell = 3; fraction = 0.25; expected = '#56B2F5' },
        @{ name = 'partial-unfilled'; cell = 3; fraction = 0.75; expected = '#326799' },
        @{ name = 'empty-5'; cell = 4; fraction = 0.50; expected = '#326799' },
        @{ name = 'empty-7'; cell = 6; fraction = 0.50; expected = '#326799' }
    )
    $bitmap = [System.Drawing.Bitmap]::FromFile($Capture.Path)
    try {
        foreach ($sample in $samples) {
            $sampleX = [int][Math]::Floor(
                $rectangle.Left - $window.Left + ($sample.cell + $sample.fraction) * $cellWidth)
            Assert-E2E ($sampleX -ge 0 -and $sampleX -lt $bitmap.Width -and
                $sampleY -ge 0 -and $sampleY -lt $bitmap.Height) `
                "Quota palette sample '$($sample.name)' is outside the captured window."
            $pixel = $bitmap.GetPixel($sampleX, $sampleY)
            $actual = '#{0:X2}{1:X2}{2:X2}' -f $pixel.R, $pixel.G, $pixel.B
            Assert-E2E ($actual -eq $sample.expected) `
                "Quota palette sample '$($sample.name)' is $actual, expected $($sample.expected)."
        }
    }
    finally {
        $bitmap.Dispose()
    }
    Write-E2E 'main-quota-gauge: seven cells, two X-authority surface colors, and half-period boundary PASS'
}

function New-E2EFixtureDocuments {
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $currentStart = $now - 7200
    $currentReset = $now + 7200
    $pastStart = $now - 25200
    $pastReset = $now - 14400
    $status = @"
{"api_version":"v1","state":"ready","observed_at":$now,"authenticated":true,"plan_label":"Pro","quota":{"remaining_percent":72.0,"reset_at":$currentReset,"window_seconds":14400,"monthly":false},"models":[{"name":"SOL","input_tokens":1200,"cached_input_tokens":200,"output_tokens":400},{"name":"TERRA","input_tokens":2400,"cached_input_tokens":500,"output_tokens":800},{"name":"LUNA","input_tokens":3600,"cached_input_tokens":700,"output_tokens":1100}],"active_thread_count":3}
"@
    # Keep this wire fixture as explicit JSON.  The details endpoint is a
    # strict twelve-field contract; serializing nested PowerShell dictionaries
    # can silently change null/number kinds between Windows PowerShell builds.
    $details = @"
{"api_version":"v1","state":"ready","observed_at":$now,"authenticated":true,"plan_label":"Pro","quota":{"remaining_percent":72.0,"reset_at":$currentReset,"window_seconds":14400,"monthly":false},"models":[{"name":"SOL","input_tokens":1200,"cached_input_tokens":200,"output_tokens":400,"input_dollars":1.20,"cached_input_dollars":0.20,"output_dollars":0.40},{"name":"TERRA","input_tokens":2400,"cached_input_tokens":500,"output_tokens":800,"input_dollars":2.40,"cached_input_dollars":0.50,"output_dollars":0.80},{"name":"LUNA","input_tokens":3600,"cached_input_tokens":700,"output_tokens":1100,"input_dollars":3.60,"cached_input_dollars":0.70,"output_dollars":1.10}],"active_thread_count":3,"history_periods":[{"id":"e2e-current","start_at":$currentStart,"end_at":$now,"reset_at":$currentReset,"label":"Current period","current":true},{"id":"e2e-past","start_at":$pastStart,"end_at":$pastReset,"reset_at":$pastReset,"label":"Past period","current":false}],"history_samples":[{"timestamp":$($currentStart + 60),"reset_at":$currentReset,"remaining_percent":92.0,"sol_dollars":0.25,"terra_dollars":0.50,"luna_dollars":0.75,"sol_tokens":100,"terra_tokens":200,"luna_tokens":300},{"timestamp":$($now - 60),"reset_at":$currentReset,"remaining_percent":72.0,"sol_dollars":1.20,"terra_dollars":2.40,"luna_dollars":3.60,"sol_tokens":1200,"terra_tokens":2400,"luna_tokens":3600},{"timestamp":$($pastStart + 60),"reset_at":$pastReset,"remaining_percent":98.0,"sol_dollars":0.10,"terra_dollars":0.20,"luna_dollars":0.30,"sol_tokens":50,"terra_tokens":100,"luna_tokens":150},{"timestamp":$($pastStart + 3600),"reset_at":$pastReset,"remaining_percent":98.0,"sol_dollars":0.10,"terra_dollars":0.20,"luna_dollars":0.30,"sol_tokens":50,"terra_tokens":100,"luna_tokens":150},{"timestamp":$($pastReset - 60),"reset_at":$pastReset,"remaining_percent":84.0,"sol_dollars":0.60,"terra_dollars":1.20,"luna_dollars":1.80,"sol_tokens":600,"terra_tokens":1200,"luna_tokens":1800}],"threads":[{"id":"e2e-root","title":"E2E root task","parent_thread_id":null,"model":"TERRA","model_label":"TERRA","total_tokens":2400,"context_usage_tokens":800,"context_window_tokens":16000,"created_at":$($now - 3600),"last_user_message_at":$($now - 300),"is_subagent":false,"depth":0},{"id":"e2e-child","title":"E2E child task","parent_thread_id":"e2e-root","model":"LUNA","model_label":"LUNA","total_tokens":1200,"context_usage_tokens":400,"context_window_tokens":16000,"created_at":$($now - 2400),"last_user_message_at":$($now - 600),"is_subagent":true,"depth":1},{"id":"e2e-orphan","title":"E2E orphan task","parent_thread_id":"missing-parent","model":"SOL","model_label":"SOL","total_tokens":600,"context_usage_tokens":null,"context_window_tokens":null,"created_at":$($now - 1200),"last_user_message_at":null,"is_subagent":true,"depth":null}],"estimated_cost_label":"USD 12.34"}
"@
    return [pscustomobject]@{
        Status = $status.Trim()
        Details = $details.Trim()
        Now = $now
    }
}

function Enter-E2EFixture {
    $documents = New-E2EFixtureDocuments
    [IO.File]::WriteAllText(
        (Join-Path $script:e2eOutput 'fixture-status.json'),
        $documents.Status,
        [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText(
        (Join-Path $script:e2eOutput 'fixture-details.json'),
        $documents.Details,
        [Text.UTF8Encoding]::new($false))
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

    # Stable AutomationIds are locale- and layout-independent.  Prefer them
    # over the bounded geometry fallback, which can otherwise confuse a
    # right-aligned selector with the title-bar close command.
    foreach ($automationId in @(
            'Main.Window.Close',
            'Graph.Window.Close',
            'Threads.Window.Close',
            'Legal.Window.Close',
            'Settings.Window.Close',
            'Setup.Window.Close')) {
        $candidate = Find-E2EElementByAutomationId $Root $automationId
        if ($null -ne $candidate -and
            $candidate.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and
            $candidate.Current.IsEnabled) {
            return $candidate
        }
    }

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
    Write-E2E "source-sha: $script:e2eSourceSha"

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
    $mainGauge = Wait-E2E -Description 'Main quota period gauge' -Probe {
        $candidate = Find-E2EElementByAutomationId $mainRoot 'Main.QuotaPeriodGauge'
        if ($null -ne $candidate) {
            $rect = $candidate.Current.BoundingRectangle
            if (-not $candidate.Current.IsOffscreen -and $rect.Width -gt 0 -and $rect.Height -gt 0) {
                return $candidate
            }
        }
        return $false
    }
    $mainCapture = Capture-E2EWindow $mainHandle '01-main-ready'
    Assert-E2E ($mainCapture.Hash.Length -eq 64) 'Main screenshot hash is missing.'
    Assert-E2EMainProductVersion $mainRoot
    if ($Fixture -or $script:e2ePreviewEnabled) {
        Assert-E2EQuotaGaugePalette -Root $mainRoot -Handle $mainHandle -Capture $mainCapture
    }
    else {
        $gauge = $mainGauge
        $gaugeRect = $gauge.Current.BoundingRectangle
        Assert-E2E (-not $gauge.Current.IsOffscreen -and $gaugeRect.Width -gt 0 -and $gaugeRect.Height -gt 0) 'Main quota period gauge is not visible.'
        Write-E2E ("main-quota-gauge: observed bounds={0}x{1}" -f $gaugeRect.Width, $gaugeRect.Height)
    }
    if ($Fixture) {
        Write-E2E ("fixture: requests={0}" -f [CodexInfoWindowsE2EFixtureServer]::RequestSummary())
    }

    # Give the bounded initial refresh one UI turn before opening a child
    # window.  The child-window assertions below inspect the rendered graph,
    # period options, metrics, and rows directly; a mutable summary TextBlock
    # peer is deliberately not used as a proxy for those surfaces.
    Start-Sleep -Seconds 5
    $startupLoading = Find-E2EElementByAutomationId $mainRoot 'Main.StartupLoading'
    Assert-E2E ($null -eq $startupLoading -or $startupLoading.Current.IsOffscreen -or -not $startupLoading.Current.IsEnabled) `
        'Startup loading surface is still visible after the first refresh window.'
    Write-E2E 'main-startup-loading: PASS (first complete generation is visible)'
    $detailsStatus = Find-E2EElementByAutomationId $mainRoot 'Main.DetailsStatus'
    Assert-E2E ($null -ne $detailsStatus) 'Main details status is missing.'
    $detailsStatusText = [string]$detailsStatus.Current.Name
    Write-E2E ("main: details status='{0}' observed" -f $detailsStatusText)
    # A screenshot or a successful status request is not sufficient evidence:
    # the main surface must have accepted the matching details generation.
    # Consume the locale-independent AutomationProperties.Name contract rather
    # than attempting to decode localized rendered text.
    $detailsContract = Find-E2EElementByAutomationId $mainRoot 'Main.DetailsGenerationContract'
    Assert-E2E ($null -ne $detailsContract) 'Main details generation contract is missing.'
    $detailsContractText = [string]$detailsContract.Current.Name
    $detailsIsLatest = $detailsContractText -eq 'ready'
    $detailsHasFailure = $detailsContractText -eq 'error'
    Write-E2E ("main: details contract value='{0}'" -f $detailsContractText)
    Write-E2E ("main: details contract latest={0} failure={1} length={2}" -f $detailsIsLatest, $detailsHasFailure, $detailsStatusText.Length)
    Assert-E2E ($detailsIsLatest -and -not $detailsHasFailure) `
        "Main details status is not a complete accepted generation: '$detailsStatusText'"
    Write-E2E 'main-details-status: PASS (matching status/details generation accepted)'

    # Finite path: one Graph window, one period round-trip, two metrics, then
    # one OFF/ON cycle for each of four independent series.  No combinations
    # of these controls are generated.
    Write-E2E 'case-1: open Graph'
    $graph = Open-E2EChildWindow -MainRoot $mainRoot -ButtonName 'Graph' -ButtonAutomationId 'Main.OpenGraph' -Title 'Codex Info Graph' -Role 'Graph' -ProcessId $clientPid
    $graphRoot = $graph.Root
    Assert-E2ENoChildProductVersion $graphRoot 'Graph'
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
    $periodItems = Wait-E2E -Description 'two Graph period options' -Probe {
        # Only the open in-window menu is enabled; the other pre-measured menu
        # remains disabled.  Filtering by enabled UIA state is DPI-independent.
        $items = @(Get-E2EVisibleControlElements $graphRoot ([System.Windows.Automation.ControlType]::ListItem))
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
    if ($Fixture -or $script:e2ePreviewEnabled) {
        Assert-E2EGraphHasModelData $plot $graph.Handle $graphPast
    }
    if ($Fixture) {
        Assert-E2EGraphHasIdleBand $plot $graph.Handle $graphPast
    }

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
    $metricItems = Wait-E2E -Description 'two Graph metric options' -Probe {
        $items = @(Get-E2EVisibleControlElements $graphRoot ([System.Windows.Automation.ControlType]::ListItem))
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
    Assert-E2ENoChildProductVersion $threadsRoot 'Threads'
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
            $rowElement = Wait-E2E -Description "Threads row identity '$($row.Id)'" -Probe {
                $candidate = Find-E2EElementByAutomationId $threadsRoot $row.Id
                if ($null -ne $candidate) { return $candidate }
                return $false
            }
            Assert-E2E ([string]$rowElement.Current.Name -eq $row.Title) "Threads row identity/title mismatch: $($row.Id)"
            Assert-E2E (@($threadTexts | Where-Object { $_ -eq $row.Model }).Count -gt 0) "Threads model column missing: $($row.Model)"
            Assert-E2E (@($threadTexts | Where-Object { $_ -like "*$($row.Column)*" }).Count -gt 0) "Threads metadata column missing: $($row.Column)"
        }
        Assert-E2E (@($threadTexts | Where-Object { $_ -like '*Parent: e2e-root*' }).Count -gt 0) 'Child parent column is missing.'
        Assert-E2E (@($threadTexts | Where-Object { $_ -like '*Parent: missing-parent*' }).Count -gt 0) 'Orphan parent column is missing.'
    }
    else {
        # Real data mode accepts the server's row identities, but still
        # requires a row container with several visible cells (title, model,
        # and metadata). An empty or status-only window cannot pass.
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

    $graphEvidence = if ($Fixture) { 'past-period model and idle-band pixels' } else { 'past-period model pixels' }
    Write-E2E ("windows-client-e2e: PASS (Graph open, {0}, period current/past/current, 2 metrics, 4 toggle OFF/ON cycles, Threads rows/columns, PID/HWND records)" -f $graphEvidence)
    $script:e2eSuccess = $true
}
catch {
    if ($null -ne $script:e2eProcess -and $null -ne $mainRoot) {
        try {
            $failureStatus = Find-E2EElementByAutomationId $mainRoot 'Main.DetailsStatus'
            if ($null -ne $failureStatus) {
                Write-E2E ("main: failure details status='{0}'" -f $failureStatus.Current.Name)
            }
            $null = Capture-E2EWindow $mainHandle 'failure-main'
        }
        catch { }
    }
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

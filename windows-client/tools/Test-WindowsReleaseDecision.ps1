# Copyright (C) 2026 salty919
# SPDX-License-Identifier: GPL-3.0-only

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$decisionScript = Join-Path $PSScriptRoot 'Get-WindowsReleaseDecision.ps1'
$work = Join-Path ([IO.Path]::GetTempPath()) ("codex-info-release-decision-" + [Guid]::NewGuid().ToString('N'))

function Write-Props {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string]$Version,
        [switch]$IncludeVersion
    )

    $versionElement = if ($IncludeVersion) { "    <Version>$Version</Version>" } else { '' }
    $xml = @"
<Project>
  <PropertyGroup>
$versionElement
    <Deterministic>true</Deterministic>
  </PropertyGroup>
</Project>
"@
    [IO.File]::WriteAllText($Path, $xml, [System.Text.UTF8Encoding]::new($false))
}

function Invoke-DecisionCase {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$CurrentVersion,
        [string]$BaseVersion,
        [switch]$BaseHasVersion,
        [switch]$OmitBaseProps,
        [Parameter(Mandatory = $true)][ValidateSet('push', 'pull_request')][string]$EventName,
        [hashtable]$Expected,
        [switch]$ExpectFailure
    )

    $currentPath = Join-Path $work "$Name.current.props"
    $basePath = Join-Path $work "$Name.base.props"
    $outputPath = Join-Path $work "$Name.outputs.txt"
    Write-Props -Path $currentPath -Version $CurrentVersion -IncludeVersion
    if (-not $OmitBaseProps) {
        Write-Props -Path $basePath -Version $BaseVersion -IncludeVersion:$BaseHasVersion
    }

    $failed = $false
    try {
        $arguments = @{
            CurrentPropsPath = $currentPath
            EventName = $EventName
            OutputPath = $outputPath
        }
        if (-not $OmitBaseProps) {
            $arguments.BasePropsPath = $basePath
        }
        & $decisionScript @arguments | Out-Null
    }
    catch {
        $failed = $true
    }

    if ($ExpectFailure) {
        if (-not $failed) {
            throw "Expected release decision case to fail: $Name"
        }
        Write-Host "PASS $Name (rejected)"
        return
    }
    if ($failed) {
        throw "Release decision case failed unexpectedly: $Name"
    }

    $values = @{}
    foreach ($line in Get-Content -LiteralPath $outputPath) {
        $parts = $line.Split('=', 2)
        if ($parts.Count -eq 2) {
            $values[$parts[0]] = $parts[1]
        }
    }
    foreach ($key in $Expected.Keys) {
        if ($values[$key] -ne $Expected[$key]) {
            throw "Unexpected $key for ${Name}: expected $($Expected[$key]), got $($values[$key])"
        }
    }
    Write-Host "PASS $Name"
}

try {
    New-Item -ItemType Directory -Path $work -Force | Out-Null
    Invoke-DecisionCase -Name 'first-version' -CurrentVersion '1.0.0' -OmitBaseProps -EventName 'push' `
        -Expected @{ version = '1.0.0'; changed = 'true'; upward = 'true'; release = 'true' }
    Invoke-DecisionCase -Name 'first-version-empty-base' -CurrentVersion '1.0.0' -BaseVersion '' -EventName 'push' `
        -Expected @{ version = '1.0.0'; changed = 'true'; upward = 'true'; release = 'true' }
    Invoke-DecisionCase -Name 'unchanged' -CurrentVersion '1.0.0' -BaseVersion '1.0.0' -BaseHasVersion `
        -EventName 'push' -Expected @{ version = '1.0.0'; changed = 'false'; upward = 'false'; release = 'false' }
    Invoke-DecisionCase -Name 'upward' -CurrentVersion '1.1.0' -BaseVersion '1.0.0' -BaseHasVersion `
        -EventName 'push' -Expected @{ version = '1.1.0'; changed = 'true'; upward = 'true'; release = 'true' }
    Invoke-DecisionCase -Name 'upward-pr' -CurrentVersion '1.1.0' -BaseVersion '1.0.0' -BaseHasVersion `
        -EventName 'pull_request' -Expected @{ version = '1.1.0'; changed = 'true'; upward = 'true'; release = 'false' }
    Invoke-DecisionCase -Name 'downward' -CurrentVersion '0.9.0' -BaseVersion '1.0.0' -BaseHasVersion `
        -EventName 'push' -ExpectFailure
    Invoke-DecisionCase -Name 'invalid' -CurrentVersion '1.0' -BaseVersion '1.0.0' -BaseHasVersion `
        -EventName 'push' -ExpectFailure
}
finally {
    if (Test-Path -LiteralPath $work) {
        Remove-Item -LiteralPath $work -Recurse -Force
    }
}

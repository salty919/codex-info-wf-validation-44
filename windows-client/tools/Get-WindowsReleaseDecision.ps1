# Copyright (C) 2026 salty919
# SPDX-License-Identifier: GPL-3.0-only

<#
.SYNOPSIS
    Validates the stable Windows client version and computes the release decision.

.DESCRIPTION
    CurrentPropsPath is the checked-out Directory.Build.props. BasePropsPath is
    the corresponding file from the comparison commit. A base file without a
    Version element, or an omitted base path, represents the first introduction
    of the release authority; a supplied but missing or malformed base file is
    an error.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CurrentPropsPath,
    [string]$BasePropsPath,
    [Parameter(Mandatory = $true)][ValidateSet('push', 'pull_request')][string]$EventName,
    [string]$OutputPath = $env:GITHUB_OUTPUT
)

$ErrorActionPreference = 'Stop'

function Get-VersionFromProps {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [switch]$AllowMissingVersion
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Version properties file was not found: $Path"
    }

    try {
        $document = [xml](Get-Content -LiteralPath $Path -Raw)
    }
    catch {
        throw "Version properties file is not valid XML: $Path"
    }
    if ($null -eq $document) {
        throw "Version properties file is not valid XML: $Path"
    }

    $versionNodes = $document.SelectNodes("/*[local-name()='Project']/*[local-name()='PropertyGroup']/*[local-name()='Version']")
    if ($AllowMissingVersion -and $versionNodes.Count -eq 0) {
        return $null
    }
    if ($versionNodes.Count -ne 1) {
        throw "Version properties file must contain exactly one Project/PropertyGroup/Version element: $Path"
    }

    $version = $versionNodes[0].InnerText.Trim()
    if ($version -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
        throw "Version must be a stable X.Y.Z value: $version"
    }

    return $version
}

$currentVersion = Get-VersionFromProps -Path $CurrentPropsPath
$previousVersion = if ([string]::IsNullOrWhiteSpace($BasePropsPath)) {
    $null
}
else {
    Get-VersionFromProps -Path $BasePropsPath -AllowMissingVersion
}

$changed = $null -eq $previousVersion -or $previousVersion -ne $currentVersion
$upward = $false
if ($changed -and $null -eq $previousVersion) {
    # Introducing the first stable property is an upward release from no authority.
    $upward = $true
}
elseif ($changed) {
    $upward = ([version]$currentVersion) -gt ([version]$previousVersion)
}

if ($changed -and -not $upward) {
    throw "Version change must be strictly upward: $previousVersion -> $currentVersion"
}

$release = $EventName -eq 'push' -and $changed -and $upward
$outputs = @(
    "version=$currentVersion"
    "changed=$($changed.ToString().ToLowerInvariant())"
    "upward=$($upward.ToString().ToLowerInvariant())"
    "release=$($release.ToString().ToLowerInvariant())"
)
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $outputDirectory = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($outputDirectory)) {
        New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
    }
    # Use the framework encoder instead of PowerShell 7's `utf8NoBOM`
    # spelling so the exact release-decision fixtures also run under the
    # Windows PowerShell 5.1 available on supported local Windows hosts.
    [IO.File]::AppendAllLines(
        [IO.Path]::GetFullPath($OutputPath),
        [string[]]$outputs,
        [System.Text.UTF8Encoding]::new($false))
}

Write-Output ($outputs -join [Environment]::NewLine)

# Copyright (C) 2026 salty919
# SPDX-License-Identifier: GPL-3.0-only

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$Destination
)

$ErrorActionPreference = 'Stop'

function Get-NuGetGlobalPackagesPath {
    if (-not [string]::IsNullOrWhiteSpace($env:NUGET_PACKAGES)) {
        return [System.IO.Path]::GetFullPath($env:NUGET_PACKAGES)
    }

    $output = (& dotnet nuget locals global-packages --list | Out-String)
    $match = [regex]::Match($output, '(?m)^global-packages:\s*(.+?)\s*$')
    if (-not $match.Success) {
        throw 'NuGet global-packages directory could not be determined.'
    }

    return $match.Groups[1].Value.Trim()
}

function Copy-RequiredPackageFile {
    param(
        [Parameter(Mandatory = $true)][string]$PackageName,
        [Parameter(Mandatory = $true)][string]$Version,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string]$NuGetRoot,
        [Parameter(Mandatory = $true)][string]$LicenseDirectory
    )

    $packageDirectory = Join-Path (Join-Path $NuGetRoot $PackageName) $Version
    $source = Join-Path $packageDirectory $FileName
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required notice is missing: $PackageName $Version $FileName"
    }

    $safeName = "$PackageName-$Version-$FileName" -replace '[\\/]', '-'
    Copy-Item -LiteralPath $source -Destination (Join-Path $LicenseDirectory $safeName) -Force
}

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$destinationDirectory = [System.IO.Path]::GetFullPath($Destination)
$licenseDirectory = Join-Path $destinationDirectory 'THIRD-PARTY-LICENSES'

New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $licenseDirectory -Force | Out-Null

Copy-Item -LiteralPath (Join-Path $repositoryRoot 'THIRD_PARTY_NOTICES.md') -Destination $destinationDirectory -Force
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSE') -Destination $destinationDirectory -Force
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'COPYRIGHT') -Destination $destinationDirectory -Force
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSES') -Destination $destinationDirectory -Recurse -Force
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'assets/NOTICE.txt') -Destination $licenseDirectory -Force

$nugetRoot = Get-NuGetGlobalPackagesPath
if (-not (Test-Path -LiteralPath $nugetRoot -PathType Container)) {
    throw "NuGet global-packages directory does not exist: $nugetRoot"
}

# These are the Windows native assets of the exact runtime graph locked in
# windows-client/**/packages.lock.json. MIT notices for managed packages live
# in LICENSES/MIT.txt; these packages additionally carry upstream notices.
$requiredFiles = @(
    @{ Name = 'avalonia.angle.windows.natives'; Version = '2.1.27548.20260419'; File = 'LICENSE' },
    @{ Name = 'harfbuzzsharp.nativeassets.win32'; Version = '8.3.1.3'; File = 'LICENSE.txt' },
    @{ Name = 'harfbuzzsharp.nativeassets.win32'; Version = '8.3.1.3'; File = 'THIRD-PARTY-NOTICES.txt' },
    @{ Name = 'skiasharp.nativeassets.win32'; Version = '3.119.4'; File = 'LICENSE.txt' },
    @{ Name = 'skiasharp.nativeassets.win32'; Version = '3.119.4'; File = 'THIRD-PARTY-NOTICES.txt' },
    @{ Name = 'scottplot'; Version = '5.1.59'; File = 'Notices/LICENSE.txt' },
    @{ Name = 'scottplot'; Version = '5.1.59'; File = 'Notices/NOTICES.txt' }
)

foreach ($requiredFile in $requiredFiles) {
    Copy-RequiredPackageFile -PackageName $requiredFile.Name -Version $requiredFile.Version -FileName $requiredFile.File -NuGetRoot $nugetRoot -LicenseDirectory $licenseDirectory
}

Write-Host "Third-party notices copied to $destinationDirectory"

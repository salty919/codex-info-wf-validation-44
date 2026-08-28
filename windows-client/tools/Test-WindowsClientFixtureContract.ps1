[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$runner = Join-Path $PSScriptRoot 'Run-WindowsClientE2E.ps1'
& $runner -FixtureContractTest

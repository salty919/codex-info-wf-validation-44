# Builds a single-file Windows setup executable containing the published client.
[CmdletBinding()]
param(
    [string]$Configuration = 'Release',
    [string]$Runtime = 'win-x64',
    [string]$OutputDirectory = 'artifacts/windows-installer'
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$clientProject = Join-Path $root 'windows-client\src\CodexInfo.WindowsClient\CodexInfo.WindowsClient.csproj'
$installerProject = Join-Path $root 'windows-client\installer\CodexInfo.WindowsClient.Installer.csproj'
$output = Join-Path $root $OutputDirectory
$work = Join-Path ([IO.Path]::GetTempPath()) ("codex-info-installer-" + [Guid]::NewGuid().ToString('N'))
$payload = Join-Path $work 'payload'
$zip = Join-Path $work 'payload.zip'

try {
    New-Item -ItemType Directory -Path $payload -Force | Out-Null
    dotnet restore $clientProject --runtime $Runtime --locked-mode
    dotnet publish $clientProject --configuration $Configuration --runtime $Runtime --self-contained true --output $payload --no-restore
    & (Join-Path $PSScriptRoot 'Collect-ThirdPartyNotices.ps1') -Destination $payload
    Compress-Archive -Path (Join-Path $payload '*') -DestinationPath $zip -CompressionLevel Optimal
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    dotnet restore $installerProject --runtime $Runtime --locked-mode
    dotnet publish $installerProject --configuration $Configuration --runtime $Runtime --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true "-p:PayloadZip=$zip" --output $output --no-restore
    $setup = Join-Path $output 'CodexInfo.WindowsClient.Setup.exe'
    if (-not (Test-Path $setup -PathType Leaf)) {
        throw "Installer publish did not create $setup"
    }
    Get-ChildItem -LiteralPath $output -Filter '*.pdb' -File -ErrorAction SilentlyContinue |
        Remove-Item -Force
    Write-Host "Created $setup"
}
finally {
    if (Test-Path $work) { Remove-Item -LiteralPath $work -Recurse -Force }
}

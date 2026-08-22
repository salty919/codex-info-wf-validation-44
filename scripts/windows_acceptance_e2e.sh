#!/usr/bin/env bash
set -euo pipefail

# Evidence gate for the Windows client.  It is intentionally read-only with
# respect to the repository and only starts/stops the installed per-user
# client when a Windows interop shell is available.
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() { echo "windows-acceptance-e2e: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing evidence file: $1"; }
require_text() { rg -q --fixed-strings -- "$2" "$1" || fail "missing contract: $1: $2"; }

DOTNET="$(command -v dotnet || true)"
if [[ -z "$DOTNET" && -x /home/salty/.codex_info_dotnet_sdk/dotnet ]]; then
    DOTNET=/home/salty/.codex_info_dotnet_sdk/dotnet
fi
[[ -n "$DOTNET" ]] || fail 'dotnet SDK is required for the Windows contract evidence'

"$DOTNET" test windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
bash scripts/windows_client_contract_gate.sh
perl -MXML::Parser -e 'XML::Parser->new->parsefile($_) for @ARGV' windows-client/src/CodexInfo.WindowsClient/*.axaml

runtime_dir="docs/evidence/visual-2026-08-22-current-v3-full"
declare -A expected=(
  [windows-main-normal-v3.png]=6c6108bc6f60c3cb6e7c9c38b416b950e338bd066aeeffc83e1ae0d82b1c9a05
  [windows-main-warning-v3.png]=76e6efe1667c3276f5dde11bf9286b5cf20979b35d4068ace8a7ba7ace19e6fb
  [windows-main-danger-v3.png]=59ddd0b53cf346811cfc59154be03970cb0575456ec2721d33812a97ae153945
  [windows-main-zero-v3.png]=57623eb33fb9baad4f93cfbc7d90d2600fb3ccb1bfaec6d05034c29060c48a1a
  [windows-main-full-v3.png]=534944b488b7be8216ef7ce25be53a761ff9a56d038f941a8ed15b944b53da5e
  [windows-main-error-v3.png]=4e1dfcf84254571629bb627eb3815ab13e10d7006d642c6da4ed0b81a2d17f0b
  [windows-main-auth-v3.png]=cbb524e26731332a41513e902813a23ed42accc57d14900184d27c088ab58ce6
  [windows-graph-current-v3.png]=75b9b65f6b7c7699ddcf2c1f8f8a4abf9656a48033192d4d41e083ee364c75c6
  [windows-threads-v3.png]=6cd0bf03f94e57bbf589ee421c23da8ffe6802fc36195e84b5e5cb7cd1c00698
  [windows-legal-v3.png]=7bb7f6132f14af27bb9c3a6ed9b45c7eff180512fc3e59f64dc8d5b8be4151a3
  [windows-settings-v3.png]=d93d5846e3c61c3b93b27e846d13cd585848c840146af51fa729f9e5005ea766
  [windows-setup-v3.png]=c5cd27d4201a3663af497282a40f3e88af9becddf4d1c6c553d5778f7d6552a5
)
for name in "${!expected[@]}"; do
    path="$runtime_dir/$name"
    require_file "$path"
    actual="$(sha256sum "$path" | awk '{print $1}')"
    [[ "$actual" == "${expected[$name]}" ]] || fail "evidence hash mismatch: $name"
done

installer_hash=8bccb053d6883b2079760c3a4a08d908dd401fd5300a7d6250f6484bf8ff59f4
installer="artifacts/windows-installer-20260822-5/CodexInfo.WindowsClient.Setup.exe"
require_file "$installer"
[[ "$(sha256sum "$installer" | awk '{print $1}')" == "$installer_hash" ]] || fail "installer hash mismatch: $installer"

require_text docs/evidence/WINDOWS_CLIENT_CURRENT_2026-08-22.md "$installer_hash"
require_text docs/evidence/WINDOWS_CLIENT_CURRENT_2026-08-22.md 'uninstallExit=0'
require_text docs/evidence/WINDOWS_CLIENT_CURRENT_2026-08-22.md '84baa617f82bce3e981a2173d4041581939f930b94c23b62c0d8089afcf3f430'
require_text docs/WINDOWS_CLIENT_REQUIREMENTS.md 'WIN-INSTALL-04'
require_text docs/COMPLETION_PROTOCOL.md 'GOV-SUBAGENT-01'
require_text docs/RELEASE_MANIFEST_2026-08-22.md 'RELEASE HOLD / 提出不可'

if command -v powershell.exe >/dev/null 2>&1; then
    move_script_win="$(wslpath -w scripts/windows_window_move_smoke.ps1 2>/dev/null || true)"
    [[ -n "$move_script_win" ]] || fail 'could not translate move smoke path for Windows PowerShell'
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command '
      $shortcut = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Codex Info\Codex Info Monitor.lnk"
      if (-not (Test-Path -LiteralPath $shortcut)) { throw "Start-menu shortcut missing" }
      $shell = New-Object -ComObject WScript.Shell
      $link = $shell.CreateShortcut($shortcut)
      if (-not (Test-Path -LiteralPath $link.TargetPath)) { throw "Start-menu target missing" }
      $p = Start-Process -FilePath $link.TargetPath -WorkingDirectory $link.WorkingDirectory -PassThru
      Start-Sleep -Seconds 3
      if ($p.HasExited) { throw "Start-menu target exited during smoke" }
      Stop-Process -Id $p.Id -Force
      Write-Output "windows-start-menu-smoke: PASS"
      $env:CODEX_INFO_WINDOWS_PREVIEW = "normal"
      $preview = Start-Process -FilePath $link.TargetPath -WorkingDirectory $link.WorkingDirectory -PassThru
      Start-Sleep -Seconds 3
      $wshell = New-Object -ComObject WScript.Shell
      if (-not $wshell.AppActivate($preview.Id)) { throw "keyboard smoke could not activate client" }
      1..8 | ForEach-Object { $wshell.SendKeys("{TAB}") }
      $wshell.SendKeys("{ESC}")
      Start-Sleep -Milliseconds 500
      if ($preview.HasExited) { throw "client exited during keyboard traversal" }
      Stop-Process -Id $preview.Id -Force
      Write-Output "windows-keyboard-focus-smoke: PASS"
    '
    client_path="$(powershell.exe -NoProfile -Command '$shell=New-Object -ComObject WScript.Shell; $lnk="$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Codex Info\Codex Info Monitor.lnk"; $shell.CreateShortcut($lnk).TargetPath' | tr -d '\r')"
    [[ -n "$client_path" ]] || fail 'could not resolve installed client path for move smoke'
    if [[ "${CODEX_INFO_ALLOW_PHYSICAL_INPUT:-0}" == "1" ]]; then
        powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$move_script_win" -ClientPath "$client_path" -AllowPhysicalInput
    else
        echo 'windows-physical-move-smoke: SKIP (set CODEX_INFO_ALLOW_PHYSICAL_INPUT=1 for explicit cursor input)'
    fi
else
    echo 'windows-start-menu-smoke: evidence already captured; powershell.exe unavailable in this runner'
fi

echo 'windows-acceptance-e2e: PASS (contract, runtime image hashes, installer identity, Start-menu, keyboard; physical move smoke is explicit opt-in)'

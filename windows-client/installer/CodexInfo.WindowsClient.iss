; Copyright (C) 2026 salty919
; SPDX-License-Identifier: GPL-3.0-only

#ifndef PayloadDir
  #error PayloadDir must identify the self-contained Windows client payload.
#endif
#ifndef OutputDir
  #error OutputDir must identify the installer artifact directory.
#endif
#ifndef ProductIcon
  #error ProductIcon must identify CodexInfo.ico.
#endif
#ifndef ProductVersion
  #define ProductVersion "1.0.0"
#endif

#define ProductName "Codex Info Monitor"
#define ProductExecutable "CodexInfo.WindowsClient.exe"

[Setup]
AppId=CodexInfo.WindowsClient
AppName={#ProductName}
AppVersion={#ProductVersion}
AppVerName={#ProductName} {#ProductVersion}
AppPublisher=salty919
AppCopyright=Copyright (C) 2026 salty919
VersionInfoVersion={#ProductVersion}
VersionInfoCompany=salty919
VersionInfoDescription={#ProductName} Setup
VersionInfoProductName={#ProductName}
DefaultDirName={localappdata}\Programs\{#ProductName}
DefaultGroupName=Codex Info
PrivilegesRequired=lowest
SetupArchitecture=x64
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=CodexInfo.WindowsClient.Setup
OutputManifestFile=CodexInfo.WindowsClient.Setup.manifest.txt
SetupIconFile={#ProductIcon}
UninstallDisplayIcon={app}\{#ProductExecutable}
UninstallDisplayName={#ProductName}
Compression=lzma2/ultra64
SolidCompression=yes
LZMANumBlockThreads=8
WizardStyle=modern dynamic
DisableWelcomePage=no
DisableReadyPage=no
DisableProgramGroupPage=yes
AllowNoIcons=no
CloseApplications=yes
CloseApplicationsFilter={#ProductExecutable}
RestartApplications=no
RestartIfNeededByRun=no
UsePreviousAppDir=yes
UsePreviousTasks=yes
ChangesEnvironment=no
ChangesAssociations=no

[Languages]
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[InstallDelete]
; Remove the superseded hand-written bootstrapper when updating an existing
; installation. User settings and Linux-side history live outside {app}.
Type: files; Name: "{app}\CodexInfo.WindowsClient.Uninstaller.exe"

[UninstallDelete]
; `createallsubdirs` preserves these empty notice directories by default.
; Remove only known-empty product directories; never recurse over {app}, so
; an unexpected user file is not silently deleted.
Type: dirifempty; Name: "{app}\THIRD-PARTY-LICENSES"
Type: dirifempty; Name: "{app}\LICENSES"
Type: dirifempty; Name: "{app}"

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#ProductName}"; Filename: "{app}\{#ProductExecutable}"; WorkingDir: "{app}"; IconFilename: "{app}\{#ProductExecutable}"; IconIndex: 0
Name: "{autodesktop}\{#ProductName}"; Filename: "{app}\{#ProductExecutable}"; WorkingDir: "{app}"; IconFilename: "{app}\{#ProductExecutable}"; IconIndex: 0; Tasks: desktopicon

[Registry]
; One-time migration from the former custom bootstrapper's uninstall key.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\CodexInfo.WindowsClient"; Flags: deletekey

[Run]
Filename: "{app}\{#ProductExecutable}"; Description: "{cm:LaunchProgram,{#ProductName}}"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent

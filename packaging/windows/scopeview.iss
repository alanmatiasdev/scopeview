; Inno Setup script. Build with:  iscc /DAppVersion=1.0.0 packaging\windows\scopeview.iss
; Expects target\release\scopeview.exe and is run from the repository root.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6C0B2A55-4A8B-4E0B-9E52-5C2C7F8D1E11}
AppName=ScopeView
AppVersion={#AppVersion}
AppPublisher=Alan Matias
AppPublisherURL=https://github.com/alanmatiasdev/scopeview
AppSupportURL=https://github.com/alanmatiasdev/scopeview/issues
DefaultDirName={autopf}\ScopeView
DefaultGroupName=ScopeView
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
LicenseFile=..\..\LICENSE
SetupIconFile=..\..\assets\icon\scopeview.ico
UninstallDisplayIcon={app}\scopeview.exe
OutputDir=..\..\dist
OutputBaseFilename=scopeview-{#AppVersion}-setup
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\release\scopeview.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\ScopeView"; Filename: "{app}\scopeview.exe"
Name: "{autodesktop}\ScopeView"; Filename: "{app}\scopeview.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\scopeview.exe"; Description: "{cm:LaunchProgram,ScopeView}"; Flags: nowait postinstall skipifsilent

#define MyAppName "SundayDeck"
#define MyAppPublisher "SundayDeck Contributors"
#define MyAppExeName "SundayDeck.exe"

[Setup]
AppId={{7A9B4C21-5F2B-4E1A-A06B-2937F0C42E10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir={#MyOutputDir}
OutputBaseFilename=SundayDeck-{#MyAppVersion}-windows-x86_64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#MySourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Registry]
Root: HKA; Subkey: "Software\Classes\.sdeck"; ValueType: string; ValueName: ""; ValueData: "SundayDeck.Bundle"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\SundayDeck.Bundle"; ValueType: string; ValueName: ""; ValueData: "SundayDeck Service Bundle"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\SundayDeck.Bundle\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

#define AppName "Sushi Calendar"
#define AppVersion "2.5.95"
[Setup]
AppId={{A0A59356-833B-4F8A-9A39-E83F8418657A}
AppName={#AppName}
AppVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\Sushi Calendar
DefaultGroupName=Sushi Calendar
OutputDir=Release
OutputBaseFilename=Sushi_Calendar_v2.5.95_Setup
SetupIconFile=assets\SushiCalendar.ico
UninstallDisplayIcon={app}\SushiCalendar_v2.5.95.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
[Files]
Source: "target\release\mr_calendar.exe"; DestDir: "{app}"; DestName: "SushiCalendar_v2.5.95.exe"; Flags: ignoreversion
[Icons]
Name: "{group}\Sushi Calendar"; Filename: "{app}\SushiCalendar_v2.5.95.exe"; IconFilename: "{app}\SushiCalendar_v2.5.95.exe"
Name: "{autodesktop}\Sushi Calendar"; Filename: "{app}\SushiCalendar_v2.5.95.exe"; IconFilename: "{app}\SushiCalendar_v2.5.95.exe"; Tasks: desktopicon
[Tasks]
Name: "desktopicon"; Description: "Create Desktop shortcut"
Name: "autostart"; Description: "Start Sushi Calendar with Windows (hidden in Tray)"; Flags: checkedonce
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "MRCalendar"; ValueType: none; Flags: deletevalue
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "SushiCalendar"; ValueData: """{app}\SushiCalendar_v2.5.95.exe"" --startup"; Flags: uninsdeletevalue; Tasks: autostart
[Run]
Filename: "{app}\SushiCalendar_v2.5.95.exe"; Description: "Run Sushi Calendar"; Flags: nowait postinstall skipifsilent

[InstallDelete]
Type: files; Name: "{app}\Sushi_Calendar.exe"

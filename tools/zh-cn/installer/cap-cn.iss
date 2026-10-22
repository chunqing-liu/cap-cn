#ifndef PayloadDir
  #error PayloadDir is required
#endif
#ifndef OutputDir
  #error OutputDir is required
#endif

[Setup]
AppId={#AppId}
AppName=Cap 中文试用版
AppVersion={#AppVersion}
AppVerName=Cap 中文试用版 {#AppVersion}（非官方）
AppPublisher=本地非官方汉化构建
VersionInfoVersion={#NumericVersion}
VersionInfoDescription=Cap 中文试用版安装程序（非官方）
DefaultDirName={localappdata}\Programs\CapCN
DefaultGroupName=Cap 中文试用版
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19041
WizardStyle=modern
DisableWelcomePage=no
DisableProgramGroupPage=yes
LicenseFile={#PayloadDir}\app\LICENSE-AGPL.txt
InfoBeforeFile={#PayloadDir}\安装与迁移说明.txt
UninstallDisplayName=Cap 中文试用版（非官方）
UninstallDisplayIcon={app}\Cap.exe
OutputDir={#OutputDir}
OutputBaseFilename=Cap-CN-{#AppVersion}-windows-x64-setup
Compression=lzma2/fast
SolidCompression=yes
CloseApplications=yes
CloseApplicationsFilter=Cap.exe
RestartApplications=no
SetupLogging=yes

[Languages]
Name: "chinesesimp"; MessagesFile: "ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "快捷方式："

[Dirs]
Name: "{app}\profile"; Flags: uninsneveruninstall
Name: "{app}\profile\recordings"; Flags: uninsneveruninstall

[Files]
Source: "{#PayloadDir}\app\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#PayloadDir}\seed-store.json"; DestDir: "{app}\profile"; DestName: "store"; Flags: onlyifdoesntexist uninsneveruninstall

[Icons]
Name: "{group}\Cap 中文试用版"; Filename: "{app}\Cap.exe"; WorkingDir: "{app}"
Name: "{group}\维护与迁移"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -STA -WindowStyle Hidden -ExecutionPolicy Bypass -File ""{app}\tools\maintenance.ps1"""; WorkingDir: "{app}"; IconFilename: "{app}\Cap.exe"
Name: "{group}\打开数据目录"; Filename: "{app}\profile"
Name: "{group}\使用说明"; Filename: "{app}\安装与迁移说明.txt"
Name: "{autodesktop}\Cap 中文试用版"; Filename: "{app}\Cap.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\Cap.exe"; Description: "运行 Cap 中文试用版"; Flags: nowait postinstall skipifsilent

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ExePath, ReleasePath: String;
  ReleaseInfo: AnsiString;
begin
  Result := '';
  ExePath := ExpandConstant('{app}\Cap.exe');
  if FileExists(ExePath) then begin
    ReleasePath := ExpandConstant('{app}\release.json');
    ReleaseInfo := '';
    LoadStringFromFile(ReleasePath, ReleaseInfo);
    if Pos('{#AppId}', String(ReleaseInfo)) = 0 then
      Result := '此目录包含官方版或其他程序，不能覆盖。请为中文版选择一个独立目录。';
  end;
end;

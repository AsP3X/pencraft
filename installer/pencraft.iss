; Setup wizard for Pencraft. Built with Inno Setup 6.3 or newer:
;
;   ISCC /DAppVersion=0.2.0 /DExePath=..\app\target\release\pencraft.exe installer\pencraft.iss
;
; installer\build.ps1 does this with the version from app\Cargo.toml.
;
; The wizard asks whether to install for this user only (no administrator
; rights needed) or for everyone. It adds a Start menu entry, optionally a
; desktop icon, optionally opens .pen files with Pencraft, and optionally
; puts the folder on PATH so AI clients can run `pencraft mcp`.
; The AppId must never change: it is how an update finds the old install.

#define AppName "Pencraft"
#define AppExe "pencraft.exe"
#define Publisher "Pencraft"
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef ExePath
  #define ExePath "..\app\target\release\pencraft.exe"
#endif

[Setup]
AppId={{CB9CA395-8DE1-4D17-BAC5-AFF90EBE0DC9}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#Publisher}
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\app\target\installer
OutputBaseFilename=pencraft-{#AppVersion}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; The icon of the setup program itself. pencraft.exe carries the same one
; (app\build.rs), which is what the shortcuts and .pen files show.
SetupIconFile=pencraft.ico
ChangesEnvironment=yes
ChangesAssociations=yes
; A running Pencraft is closed before its file is replaced, and started again.
CloseApplications=yes
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppExe}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "associate"; Description: "Open .pen and .pencraft files with {#AppName}"
Name: "addtopath"; Description: "Add to PATH, so AI clients can run pencraft mcp"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#ExePath}"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; Comment: "Canvas design editor for .pen files"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; HKA is the machine's registry for an install for everyone, the user's own otherwise.
Root: HKA; Subkey: "Software\Classes\.pen\OpenWithProgids"; ValueType: string; ValueName: "Pencraft.Design"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\.pencraft\OpenWithProgids"; ValueType: string; ValueName: "Pencraft.Design"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Pencraft.Design"; ValueType: string; ValueName: ""; ValueData: "Pencraft design"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Pencraft.Design\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Pencraft.Design\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: associate

[Run]
Filename: "{app}\{#AppExe}"; Description: "Start {#AppName}"; Flags: nowait postinstall skipifsilent
; An update Pencraft started itself (app\src\update\install.rs) runs without
; pages. /WAITPID=<process> has it wait for that Pencraft to quit first, and
; /RELAUNCH asks for the app to come back, as the user who started the setup
; and not as the administrator an install for everyone runs as.
Filename: "{app}\{#AppExe}"; Flags: nowait runasoriginaluser; Check: RelaunchAsked

[Code]
// Whether the command line has /RELAUNCH.
function RelaunchAsked: Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/RELAUNCH') = 0 then
      Result := True;
end;

function OpenProcess(Access: DWORD; Inherit: BOOL; Process: DWORD): THandle;
  external 'OpenProcess@kernel32.dll stdcall';
function WaitForSingleObject(Handle: THandle; Milliseconds: DWORD): DWORD;
  external 'WaitForSingleObject@kernel32.dll stdcall';
function CloseHandle(Handle: THandle): BOOL;
  external 'CloseHandle@kernel32.dll stdcall';

// The process number given with /WAITPID=, or 0.
function WaitFor: Integer;
var
  I: Integer;
  P: String;
begin
  Result := 0;
  for I := 1 to ParamCount do
  begin
    P := ParamStr(I);
    if CompareText(Copy(P, 1, 9), '/WAITPID=') = 0 then
      Result := StrToIntDef(Copy(P, 10, Length(P)), 0);
  end;
end;

// The Pencraft that started an update is quitting while Setup starts. Its
// file cannot be replaced before it is gone, and a silent Setup that finds
// it running gives up, so wait for it: half a minute at most.
function InitializeSetup: Boolean;
var
  Process: Integer;
  Handle: THandle;
begin
  Result := True;
  Process := WaitFor;
  if Process <= 0 then
    exit;
  // SYNCHRONIZE: the right to wait for the process, nothing more.
  Handle := OpenProcess($00100000, False, Process);
  if Handle = 0 then
    exit;
  WaitForSingleObject(Handle, 30000);
  CloseHandle(Handle);
end;

// PATH lives in the machine environment for an install for everyone, and
// in the user's own environment otherwise.

function PathRoot: Integer;
begin
  if IsAdminInstallMode then
    Result := HKEY_LOCAL_MACHINE
  else
    Result := HKEY_CURRENT_USER;
end;

function PathKey: String;
begin
#ifdef TestPathKey
  // Test builds edit a value under this key instead of the real PATH.
  Result := '{#TestPathKey}';
#else
  if IsAdminInstallMode then
    Result := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
  else
    Result := 'Environment';
#endif
end;

// Position of Dir as a whole entry of the semicolon-separated Paths, or 0.
function PathPos(Paths, Dir: String): Integer;
begin
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';');
end;

procedure AddToPath(Dir: String);
var
  Paths: String;
begin
  if not RegQueryStringValue(PathRoot, PathKey, 'Path', Paths) then
  begin
    // A PATH that exists but cannot be read is left alone: writing now
    // would replace it with this one folder.
    if RegValueExists(PathRoot, PathKey, 'Path') then
      exit;
    Paths := '';
  end;
  if PathPos(Paths, Dir) > 0 then
    exit;
  if (Paths <> '') and (Paths[Length(Paths)] <> ';') then
    Paths := Paths + ';';
  RegWriteExpandStringValue(PathRoot, PathKey, 'Path', Paths + Dir);
end;

procedure RemoveFromPath(Dir: String);
var
  Paths: String;
  P: Integer;
begin
  if not RegQueryStringValue(PathRoot, PathKey, 'Path', Paths) then
    exit;
  P := PathPos(Paths, Dir);
  if P = 0 then
    exit;
  // P counts in the string with a ';' put in front, so it is where the
  // entry starts in Paths itself. Take the entry and one separator out.
  Delete(Paths, P, Length(Dir) + 1);
  if (Paths <> '') and (Paths[Length(Paths)] = ';') then
    Delete(Paths, Length(Paths), 1);
  RegWriteExpandStringValue(PathRoot, PathKey, 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    AddToPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;

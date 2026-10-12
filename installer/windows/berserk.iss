#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#ifndef BinaryDir
  #define BinaryDir "..\..\dist\windows"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif
#define MarkerKey "Software\Berserk\Installer"

[Setup]
AppId={{76A81232-C98B-46D2-8A59-9E97988C40CB}
AppName=Berserk CLI
AppVersion={#AppVersion}
AppPublisher=Justin Angelo Perez
AppPublisherURL=https://github.com/justinangeloperez327/berserk
DefaultDirName={localappdata}\Programs\Berserk
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableProgramGroupPage=yes
DisableDirPage=no
ChangesEnvironment=yes
LicenseFile=..\..\LICENSE
OutputDir={#OutputDir}
OutputBaseFilename=BerserkSetup-{#AppVersion}-windows-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName=Berserk CLI {#AppVersion}
UninstallDisplayIcon={app}\bin\berserk.exe

[Files]
Source: "{#BinaryDir}\berserk.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "{#BinaryDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinaryDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinaryDir}\BUILD-MANIFEST.json"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
Root: HKCU; Subkey: "{#MarkerKey}"; Flags: uninsdeletekey

[Messages]
FinishedLabel=Berserk CLI is installed. Open a new terminal and run "berserk new app".%n%nRust 1.88+ and the MSVC C++ build tools are required to build and serve applications.

[Code]
function NormalizePart(Part: String): String;
begin
  Part := Trim(Part);
  if (Length(Part) >= 2) and (Part[1] = '"') and (Part[Length(Part)] = '"') then begin
    Delete(Part, Length(Part), 1);
    Delete(Part, 1, 1);
  end;
  while (Length(Part) > 3) and (Part[Length(Part)] = '\') do
    Delete(Part, Length(Part), 1);
  Result := Lowercase(Part);
end;

function HasPart(Value, Entry: String): Boolean;
var Rest, Part: String; Separator: Integer;
begin
  Result := False;
  Rest := Value;
  while True do begin
    Separator := Pos(';', Rest);
    if Separator = 0 then Part := Rest
    else Part := Copy(Rest, 1, Separator - 1);
    if NormalizePart(Part) = NormalizePart(Entry) then begin
      Result := True;
      Exit;
    end;
    if Separator = 0 then Exit;
    Delete(Rest, 1, Separator);
  end;
end;

function WithoutPart(Value, Entry: String): String;
var Rest, Part: String; Separator: Integer; Kept: Boolean;
begin
  Result := '';
  Kept := False;
  Rest := Value;
  while True do begin
    Separator := Pos(';', Rest);
    if Separator = 0 then Part := Rest
    else Part := Copy(Rest, 1, Separator - 1);
    if NormalizePart(Part) <> NormalizePart(Entry) then begin
      if Kept then Result := Result + ';';
      Result := Result + Part;
      Kept := True;
    end;
    if Separator = 0 then Exit;
    Delete(Rest, 1, Separator);
  end;
end;

procedure CurStepChanged(Step: TSetupStep);
var UserPath, BinPath, PreviousPath, UpdatedPath: String;
begin
  if Step <> ssPostInstall then Exit;
  BinPath := ExpandConstant('{app}\bin');
  RegQueryStringValue(HKCU, 'Environment', 'Path', UserPath);
  if RegQueryStringValue(HKCU, '{#MarkerKey}', 'AddedPath', PreviousPath) then
    if NormalizePart(PreviousPath) <> NormalizePart(BinPath) then begin
      UserPath := WithoutPart(UserPath, PreviousPath);
      if not RegWriteExpandStringValue(HKCU, 'Environment', 'Path', UserPath) then
        RaiseException('Could not update the previous Berserk PATH entry.');
      RegDeleteValue(HKCU, '{#MarkerKey}', 'AddedPath');
    end;
  if not HasPart(UserPath, BinPath) then begin
    UpdatedPath := UserPath;
    if (UpdatedPath <> '') and (UpdatedPath[Length(UpdatedPath)] <> ';') then
      UpdatedPath := UpdatedPath + ';';
    UpdatedPath := UpdatedPath + BinPath;
    if Length(UpdatedPath) > 32760 then
      RaiseException('Your user PATH is too long to add Berserk safely.');
    if not RegWriteExpandStringValue(HKCU, 'Environment', 'Path', UpdatedPath) then
      RaiseException('Could not add Berserk to your user PATH.');
    if not RegWriteStringValue(HKCU, '{#MarkerKey}', 'AddedPath', BinPath) then
      RaiseException('Could not record the Berserk PATH entry for uninstall.');
  end;
end;

procedure CurUninstallStepChanged(Step: TUninstallStep);
var UserPath, AddedPath: String;
begin
  if Step <> usUninstall then Exit;
  if RegQueryStringValue(HKCU, '{#MarkerKey}', 'AddedPath', AddedPath) then
    if RegQueryStringValue(HKCU, 'Environment', 'Path', UserPath) then
      RegWriteExpandStringValue(HKCU, 'Environment', 'Path', WithoutPart(UserPath, AddedPath));
end;

param(
    [Parameter(Mandatory = $true)][string]$Installer,
    [Parameter(Mandatory = $true)][string]$FrameworkCommit,
    [Parameter(Mandatory = $true)][string]$EvidenceDir
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
function Install-Cli([string]$Destination, [string]$Log) {
    $process = Start-Process -FilePath $Installer -ArgumentList @(
        '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-',
        "/DIR=`"$Destination`"", "/LOG=`"$Log`""
    ) -Wait -PassThru
    Assert-True ($process.ExitCode -eq 0) "Installer failed: $($process.ExitCode)"
}
function Uninstall-Cli([string]$Destination, [string]$Log) {
    $process = Start-Process -FilePath (Join-Path $Destination 'unins000.exe') -ArgumentList @(
        '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$Log`""
    ) -Wait -PassThru
    Assert-True ($process.ExitCode -eq 0) "Uninstaller failed: $($process.ExitCode)"
}
function Read-UserPath {
    return [string]$script:Registry.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
}

$Installer = (Resolve-Path $Installer).Path
$Cargo = (Get-Command cargo -ErrorAction Stop).Source
$Registry = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
$OriginalPath = $Registry.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
$OriginalKind = if ($null -ne $OriginalPath) { $Registry.GetValueKind('Path') } else { $null }
$OriginalProcessPath = $env:PATH
$MachinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
$OriginalAddress = $env:BERSERK_ADDRESS
$OriginalRustFlags = $env:RUSTFLAGS
$Work = Join-Path $env:RUNNER_TEMP 'Berserk Installer Test'
$InstallDir = Join-Path $Work 'CLI With Spaces'
$Bin = Join-Path $InstallDir 'bin'
$ProjectRoot = Join-Path $Work 'User Projects'
$Server = $null
New-Item -ItemType Directory -Force $EvidenceDir, $ProjectRoot | Out-Null
$Baseline = [string]$OriginalPath
if ($Baseline -and -not $Baseline.EndsWith(';')) { $Baseline += ';' }
$Baseline += 'C:\Berserk Unrelated Sentinel'
$Expected = "$Baseline;$Bin"
$InstalledExe = Join-Path $Bin 'berserk.exe'

try {
    $Registry.SetValue('Path', $Baseline, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Install-Cli $InstallDir (Join-Path $EvidenceDir 'install.log')
    Assert-True ((Read-UserPath) -eq $Expected) 'Install did not preserve and append the user PATH'
    Assert-True ([Environment]::GetEnvironmentVariable('Path', 'Machine') -eq $MachinePath) 'Installer changed machine PATH'
    Assert-True (Test-Path $InstalledExe) 'Installed executable is missing'
    $env:PATH = "$MachinePath;$(Read-UserPath);$OriginalProcessPath"
    Assert-True ((Get-Command berserk -ErrorAction Stop).Source -eq $InstalledExe) 'berserk does not resolve through the installed PATH'
    $version = (& berserk --version | Out-String).Trim()
    Assert-True ($LASTEXITCODE -eq 0 -and $version -eq 'berserk 1.0.0') 'Installed version command failed'

    Install-Cli $InstallDir (Join-Path $EvidenceDir 'upgrade.log')
    Assert-True ((Read-UserPath) -eq $Expected) 'Repeated installation duplicated or changed PATH'

    Push-Location $ProjectRoot
    try {
        & berserk new app
        Assert-True ($LASTEXITCODE -eq 0) 'Installed CLI failed to create app'
    } finally { Pop-Location }
    $App = Join-Path $ProjectRoot 'app'
    # The job's static-CRT flag is for the distributed CLI only. Consumers use
    # their normal toolchain settings, including host procedural macros.
    $env:RUSTFLAGS = $null
    $Manifest = Get-Content (Join-Path $App 'Cargo.toml') -Raw
    Assert-True ($Manifest.Contains("rev = `"$FrameworkCommit`"")) 'Generated app is not pinned to the tested framework'
    Assert-True ($Manifest.Contains('git = "https://github.com/justinangeloperez327/berserk"')) 'Generated app still requires unpublished framework registry packages'
    & $Cargo generate-lockfile --manifest-path (Join-Path $App 'Cargo.toml')
    Assert-True ($LASTEXITCODE -eq 0) 'Generated Git-backed dependency graph failed'
    & $Cargo build --locked --manifest-path (Join-Path $App 'Cargo.toml')
    Assert-True ($LASTEXITCODE -eq 0) 'Generated app build failed'

    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = $listener.LocalEndpoint.Port
    $listener.Stop()
    $env:BERSERK_ADDRESS = "127.0.0.1:$port"
    $Server = Start-Process -FilePath $InstalledExe -ArgumentList 'serve' -WorkingDirectory $App -PassThru `
        -RedirectStandardOutput (Join-Path $EvidenceDir 'serve.stdout.log') `
        -RedirectStandardError (Join-Path $EvidenceDir 'serve.stderr.log')
    $healthy = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        if ($Server.HasExited) { throw 'berserk serve exited before the app became healthy' }
        try {
            $response = Invoke-WebRequest "http://127.0.0.1:$port/health" -TimeoutSec 2
            if ($response.StatusCode -eq 200) { $healthy = $true; break }
        } catch { Start-Sleep -Milliseconds 500 }
    }
    Assert-True $healthy 'Installed berserk serve did not expose the generated app health route'
    $welcome = Invoke-WebRequest "http://127.0.0.1:$port/" -TimeoutSec 5
    Assert-True ($welcome.StatusCode -eq 200) 'Generated app welcome route failed'
    & taskkill.exe /PID $Server.Id /T /F | Out-Null
    $Server.WaitForExit()
    $Server = $null

    Uninstall-Cli $InstallDir (Join-Path $EvidenceDir 'uninstall.log')
    Assert-True ((Read-UserPath) -eq $Baseline) 'Uninstall did not preserve unrelated user PATH entries'
    Assert-True (-not (Test-Path $InstalledExe)) 'Uninstall left the installed executable'
    Assert-True (Test-Path (Join-Path $App 'Cargo.toml')) 'Uninstall removed the user project'

    $PreexistingPath = "$Baseline;$($Bin.ToUpperInvariant())\"
    $Registry.SetValue('Path', $PreexistingPath, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Install-Cli $InstallDir (Join-Path $EvidenceDir 'preexisting-install.log')
    Assert-True ((Read-UserPath) -eq $PreexistingPath) 'Installer duplicated a pre-existing equivalent entry'
    Uninstall-Cli $InstallDir (Join-Path $EvidenceDir 'preexisting-uninstall.log')
    Assert-True ((Read-UserPath) -eq $PreexistingPath) 'Uninstaller removed a user-owned pre-existing entry'
    Assert-True ([Environment]::GetEnvironmentVariable('Path', 'Machine') -eq $MachinePath) 'Machine PATH changed'

    @{
        version = $version
        framework_commit = $FrameworkCommit
        install_and_path = 'passed'
        upgrade_without_duplicates = 'passed'
        git_app_build = 'passed'
        serve_health_and_welcome = 'passed'
        uninstall_preserves_projects_and_path = 'passed'
        preexisting_path_preserved = 'passed'
        machine_path_unchanged = 'passed'
    } | ConvertTo-Json | Set-Content (Join-Path $EvidenceDir 'installation-results.json') -Encoding utf8
} finally {
    if ($null -ne $Server -and -not $Server.HasExited) {
        & taskkill.exe /PID $Server.Id /T /F | Out-Null
    }
    if (Test-Path (Join-Path $InstallDir 'unins000.exe')) {
        try { Uninstall-Cli $InstallDir (Join-Path $EvidenceDir 'cleanup.log') } catch { Write-Warning $_ }
    }
    if ($null -eq $OriginalPath) { $Registry.DeleteValue('Path', $false) }
    else { $Registry.SetValue('Path', $OriginalPath, $OriginalKind) }
    $Registry.Dispose()
    $env:PATH = $OriginalProcessPath
    $env:BERSERK_ADDRESS = $OriginalAddress
    $env:RUSTFLAGS = $OriginalRustFlags
}

<#
.SYNOPSIS
  Builds the Windows release executable and both installers.

.DESCRIPTION
  Writes to app\target\installer:
    pencraft-<version>-setup.exe   setup wizard (Inno Setup); the app's update card downloads this
    pencraft-<version>-x64.msi     Windows Installer package (WiX)
    pencraft-<version>-x64.exe     the plain program

  The version comes from app\Cargo.toml. An installer whose tool is missing is
  skipped with a note on how to get the tool; the script then exits with 1.
  The release workflow writes SHA256SUMS.txt over the files of every system.

  Tools:
    WiX Toolset 4 or 5    dotnet tool install --global wix --version 5.0.2
    Inno Setup 6.3+       winget install JRSoftware.InnoSetup

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File installer\build.ps1
#>
[CmdletBinding()]
param(
    # Use the executable that is already in app\target\release.
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$app = Join-Path $root 'app'

$manifest = Get-Content (Join-Path $app 'Cargo.toml') -Raw
if ($manifest -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw 'app\Cargo.toml has no version.'
}
$version = $Matches[1]
$exe = Join-Path $app 'target\release\pencraft.exe'
$out = Join-Path $app 'target\installer'

if (-not $SkipBuild) {
    Write-Host "Building pencraft $version (release)"
    Push-Location $app
    try {
        cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'cargo build failed.' }
    } finally {
        Pop-Location
    }
}
if (-not (Test-Path $exe)) { throw "$exe is missing. Run without -SkipBuild." }
New-Item -ItemType Directory -Force $out | Out-Null
# Files of other versions would end up in a release. Start clean.
Get-ChildItem $out -File | Remove-Item -Force

$missing = @()

# ---- MSI
$wix = Get-Command wix -ErrorAction SilentlyContinue
if ($wix) {
    $msi = Join-Path $out "pencraft-$version-x64.msi"
    Write-Host "Building $msi"
    & $wix.Source build (Join-Path $PSScriptRoot 'pencraft.wxs') `
        -arch x64 -pdbtype none -d "Version=$version" -d "ExePath=$exe" -o $msi
    if ($LASTEXITCODE -ne 0) { throw 'wix build failed.' }
} else {
    $missing += 'MSI skipped: the WiX Toolset is not installed. Get it with: dotnet tool install --global wix --version 5.0.2'
}

# ---- setup.exe
$iscc = Get-Command ISCC -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
if (-not $iscc) {
    $iscc = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe')
    ) | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if ($iscc) {
    Write-Host "Building the setup wizard with $iscc"
    # The name must end in -setup.exe: app\src\update.rs looks for that.
    & $iscc /Qp "/DAppVersion=$version" "/DExePath=$exe" "/O$out" (Join-Path $PSScriptRoot 'pencraft.iss')
    if ($LASTEXITCODE -ne 0) { throw 'ISCC failed.' }
} else {
    $missing += 'setup.exe skipped: Inno Setup is not installed. Get it with: winget install JRSoftware.InnoSetup'
}

# ---- the plain program, for people who want no installer
Copy-Item $exe (Join-Path $out "pencraft-$version-x64.exe") -Force

Write-Host ''
Get-ChildItem $out -File | ForEach-Object {
    Write-Host ('{0,-48} {1,8:N0} KB' -f $_.Name, ($_.Length / 1KB))
}
if ($missing.Count -gt 0) {
    Write-Host ''
    $missing | ForEach-Object { Write-Warning $_ }
    exit 1
}

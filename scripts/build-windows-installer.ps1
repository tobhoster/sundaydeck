# Build Windows release packages:
#   1. Portable ZIP: dist/SundayDeck-<version>-windows-x86_64.zip
#   2. Inno Setup Installer: dist/SundayDeck-<version>-windows-x86_64-setup.exe

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

$CargoToml = Get-Content "$RootDir\Cargo.toml" -Raw
if ($CargoToml -match 'version\s*=\s*"([^"]+)"') {
    $Version = $Matches[1]
} else {
    throw "Could not parse version from Cargo.toml"
}

Write-Host "==> Building SundayDeck v$Version (windows-x86_64) release binary..."
cargo build --release -p sundaydeck-ui --bin sundaydeck

$DistDir = Join-Path $RootDir "dist"
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

$PortableDir = Join-Path $DistDir "SundayDeck-$Version-windows-x86_64"
if (Test-Path $PortableDir) { Remove-Item -Recurse -Force $PortableDir }
New-Item -ItemType Directory -Force -Path $PortableDir | Out-Null

Copy-Item "$RootDir\target\release\sundaydeck.exe" "$PortableDir\SundayDeck.exe"
if (Test-Path "$RootDir\crates\sundaydeck-ui\assets\video") {
    Copy-Item -Recurse "$RootDir\crates\sundaydeck-ui\assets\video" "$PortableDir\video"
}
Copy-Item "$RootDir\README.md" "$PortableDir\README.md"

$ZipPath = Join-Path $DistDir "SundayDeck-$Version-windows-x86_64.zip"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path "$PortableDir\*" -DestinationPath $ZipPath
Write-Host "==> Created portable archive: $ZipPath"

# Build Windows setup .exe via Inno Setup (preinstalled on GitHub windows-latest runners)
$Iscc = Get-Command iscc -ErrorAction SilentlyContinue
if (-not $Iscc) {
    $DefaultIscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
    if (Test-Path $DefaultIscc) { $Iscc = $DefaultIscc }
}

if ($Iscc) {
    Write-Host "==> Building Windows Installer with Inno Setup..."
    & $Iscc "/DMyAppVersion=$Version" "/DMySourceDir=$PortableDir" "/DMyOutputDir=$DistDir" "$RootDir\packaging\windows\sundaydeck.iss"
} else {
    Write-Warning "Inno Setup (iscc) not found; skipping .exe installer (portable .zip created)."
}

Remove-Item -Recurse -Force $PortableDir
Get-ChildItem $DistDir -Include *.zip, *.exe -Recurse

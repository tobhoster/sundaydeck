# SundayDeck one-line Windows installer (PowerShell).
# Usage:
#   irm https://tobhoster.github.io/sundaydeck/install.ps1 | iex

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$Repo = "tobhoster/sundaydeck"
$FallbackVersion = "0.0.3"

$Version = $env:SUNDAYDECK_VERSION
if (-not $Version) {
    try {
        $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "SundayDeck-Installer" }
        $Version = ($Release.tag_name -replace '^v', '')
    } catch {
        $Version = $FallbackVersion
    }
}
$Version = $Version -replace '^v', ''
$Tag = "v$Version"
$Asset = "SundayDeck-$Version-windows-x86_64-setup.exe"
$BaseUrl = "https://github.com/$Repo/releases/download/$Tag"
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("sundaydeck-install-" + [System.Guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

try {
    $InstallerPath = Join-Path $TempDir $Asset
    Write-Host "Downloading SundayDeck $Tag ($Asset)..."
    Invoke-WebRequest -Uri "$BaseUrl/$Asset" -OutFile $InstallerPath

    $Sums = (Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS.txt" -UseBasicParsing).Content
    $Line = ($Sums -split "`n") | Where-Object { $_ -match [regex]::Escape($Asset) } | Select-Object -First 1
    if (-not $Line) {
        throw "Checksum entry missing for $Asset in SHA256SUMS.txt"
    }
    $Expected = ($Line.Trim() -split '\s+')[0].ToLower()
    $Actual = (Get-FileHash -Path $InstallerPath -Algorithm SHA256).Hash.ToLower()
    if ($Actual -ne $Expected) {
        throw "SHA-256 checksum mismatch for $Asset (expected $Expected, got $Actual)"
    }
    Write-Host "Verified SHA-256 checksum."

    Write-Host "Launching SundayDeck $Tag installer..."
    Start-Process -FilePath $InstallerPath -Wait
    Write-Host "SundayDeck $Tag setup finished."
} finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $TempDir
}

# Build Custom Neolithic TERA MSI Installer
# This script harvests files from the game server and builds the MSI

param(
    [string]$GameServerPath = "D:\V100TERA\Neolithic Test Server",
    [string]$LauncherExePath = "D:\V100TERA\BACKUP_WORKING_LAUNCHER_2024-12-16\tera-rust-launcher\teralaunch\src-tauri\target\release\Neolithic TERA Launcher.exe",
    [string]$OutputDir = ".\Output"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Neolithic TERA MSI Builder" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check WiX Toolset
$wixPath = "C:\Program Files\WiX Toolset v6.0\bin\wix.exe"
if (-not (Test-Path $wixPath)) {
    Write-Host "ERROR: WiX Toolset not found!" -ForegroundColor Red
    Write-Host "Please install WiX Toolset from: https://wixtoolset.org/releases/" -ForegroundColor Yellow
    exit 1
}

Write-Host "`n[1/7] Validating source files..." -ForegroundColor Green

# Validate paths
if (-not (Test-Path $GameServerPath)) {
    Write-Host "ERROR: Game server not found at: $GameServerPath" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $LauncherExePath)) {
    Write-Host "ERROR: Launcher executable not found at: $LauncherExePath" -ForegroundColor Red
    exit 1
}

# Create staging directory
$stagingDir = ".\SourceFiles"
if (Test-Path $stagingDir) {
    Remove-Item $stagingDir -Recurse -Force
}
New-Item -ItemType Directory -Path $stagingDir | Out-Null

Write-Host "[2/7] Copying launcher executable..." -ForegroundColor Green
Copy-Item $LauncherExePath -Destination "$stagingDir\Neolithic TERA Launcher.exe"

Write-Host "[3/7] Copying file_cache.json..." -ForegroundColor Green
$fileCachePath = Join-Path $GameServerPath "file_cache.json"
if (Test-Path $fileCachePath) {
    Copy-Item $fileCachePath -Destination "$stagingDir\file_cache.json"
} else {
    # Create empty file_cache.json if not found
    Write-Host "  Warning: file_cache.json not found, creating empty file" -ForegroundColor Yellow
    "{}" | Out-File "$stagingDir\file_cache.json" -Encoding utf8
}

Write-Host "[4/7] Copying Binaries folder..." -ForegroundColor Green
$binariesSrc = Join-Path $GameServerPath "Binaries"
$binariesDst = Join-Path $stagingDir "Binaries"
Copy-Item $binariesSrc -Destination $binariesDst -Recurse
$binariesSize = (Get-ChildItem $binariesDst -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host "  Binaries size: $([math]::Round($binariesSize, 2)) MB" -ForegroundColor Cyan

Write-Host "[5/7] Copying Engine folder..." -ForegroundColor Green
$engineSrc = Join-Path $GameServerPath "Engine"
$engineDst = Join-Path $stagingDir "Engine"
Copy-Item $engineSrc -Destination $engineDst -Recurse
$engineSize = (Get-ChildItem $engineDst -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host "  Engine size: $([math]::Round($engineSize, 2)) MB" -ForegroundColor Cyan

Write-Host "[6/7] Generating file lists..." -ForegroundColor Green

# Generate Binaries component list
$binariesWxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Fragment>
    <ComponentGroup Id="BinariesFiles" Directory="BinariesDir">
"@

$binariesFiles = Get-ChildItem "$stagingDir\Binaries" -Recurse -File
$compId = 1
foreach ($file in $binariesFiles) {
    $relativePath = $file.FullName.Substring($stagingDir.Length + 11) # Remove "SourceFiles\Binaries\"
    $source = $file.FullName
    $binariesWxs += "`n      <Component Id=`"BinFile$compId`">`n        <File Source=`"$source`" />`n      </Component>"
    $compId++
}

$binariesWxs += @"

    </ComponentGroup>
  </Fragment>
</Wix>
"@

$binariesWxs | Out-File "Binaries.wxs" -Encoding UTF8

# Generate Engine component list
$engineWxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Fragment>
    <ComponentGroup Id="EngineFiles" Directory="EngineDir">
"@

$engineFiles = Get-ChildItem "$stagingDir\Engine" -Recurse -File
$compId = 1
foreach ($file in $engineFiles) {
    $relativePath = $file.FullName.Substring($stagingDir.Length + 8) # Remove "SourceFiles\Engine\"
    $source = $file.FullName
    $engineWxs += "`n      <Component Id=`"EngFile$compId`">`n        <File Source=`"$source`" />`n      </Component>"
    $compId++
}

$engineWxs += @"

    </ComponentGroup>
  </Fragment>
</Wix>
"@

$engineWxs | Out-File "Engine.wxs" -Encoding UTF8

Write-Host "  Generated $($binariesFiles.Count) Binaries file components" -ForegroundColor Cyan
Write-Host "  Generated $($engineFiles.Count) Engine file components" -ForegroundColor Cyan

Write-Host "[7/7] Building MSI with WiX v6..." -ForegroundColor Green

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Build MSI
& "C:\Program Files\WiX Toolset v6.0\bin\wix.exe" build `
    -arch x64 `
    -ext WixToolset.UI.wixext `
    -bindpath "." `
    -o "$OutputDir\NeolithicTERA-Setup.msi" `
    NeolithicTERA.wxs CustomUI.wxs Binaries.wxs Engine.wxs

if ($LASTEXITCODE -eq 0) {
    $msiPath = Join-Path (Resolve-Path $OutputDir) "NeolithicTERA-Setup.msi"
    $msiSize = (Get-Item $msiPath).Length / 1MB
    
    Write-Host "`n========================================" -ForegroundColor Green
    Write-Host "MSI BUILT SUCCESSFULLY!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "Output: $msiPath" -ForegroundColor Cyan
    Write-Host "Size: $([math]::Round($msiSize, 2)) MB" -ForegroundColor Cyan
    Write-Host "`nTotal content size:" -ForegroundColor Yellow
    Write-Host "  Launcher: ~20 MB" -ForegroundColor Cyan
    Write-Host "  Binaries: $([math]::Round($binariesSize, 2)) MB" -ForegroundColor Cyan
    Write-Host "  Engine: $([math]::Round($engineSize, 2)) MB" -ForegroundColor Cyan
    Write-Host "  Total: ~$([math]::Round($binariesSize + $engineSize + 20, 2)) MB" -ForegroundColor Cyan
} else {
    Write-Host "`nERROR: MSI build failed!" -ForegroundColor Red
    exit 1
}

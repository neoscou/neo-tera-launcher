# Build Modern NSIS Installer for Neolithic TERA

param(
    [string]$GameServerPath = "D:\V100TERA\Neolithic Test Server",
    [string]$LauncherExePath = "D:\V100TERA\BACKUP_WORKING_LAUNCHER_2024-12-16\tera-rust-launcher\teralaunch\src-tauri\target\release\Neolithic TERA Launcher.exe",
    [string]$OutputDir = ".\Output"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Neolithic TERA NSIS Installer Builder" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check NSIS
$nsisPath = "C:\Program Files (x86)\NSIS\makensis.exe"
if (-not (Test-Path $nsisPath)) {
    Write-Host "ERROR: NSIS not found!" -ForegroundColor Red
    Write-Host "Please install NSIS from: https://nsis.sourceforge.io/Download" -ForegroundColor Yellow
    exit 1
}

Write-Host "`n[1/5] Validating source files..." -ForegroundColor Green

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

Write-Host "[2/5] Copying launcher executable..." -ForegroundColor Green
Copy-Item $LauncherExePath -Destination "$stagingDir\Neolithic TERA Launcher.exe"

Write-Host "[3/5] Copying file_cache.json..." -ForegroundColor Green
$fileCachePath = Join-Path $GameServerPath "file_cache.json"
if (Test-Path $fileCachePath) {
    Copy-Item $fileCachePath -Destination "$stagingDir\file_cache.json"
} else {
    Write-Host "  Warning: file_cache.json not found, creating empty file" -ForegroundColor Yellow
    "{}" | Out-File "$stagingDir\file_cache.json" -Encoding utf8
}

Write-Host "[4/5] Copying game files..." -ForegroundColor Green
$binariesSrc = Join-Path $GameServerPath "Binaries"
$binariesDst = Join-Path $stagingDir "Binaries"
Copy-Item $binariesSrc -Destination $binariesDst -Recurse
$binariesSize = (Get-ChildItem $binariesDst -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host "  Binaries size: $([math]::Round($binariesSize, 2)) MB" -ForegroundColor Cyan

$engineSrc = Join-Path $GameServerPath "Engine"
$engineDst = Join-Path $stagingDir "Engine"
Copy-Item $engineSrc -Destination $engineDst -Recurse
$engineSize = (Get-ChildItem $engineDst -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host "  Engine size: $([math]::Round($engineSize, 2)) MB" -ForegroundColor Cyan

Write-Host "[5/5] Building NSIS installer..." -ForegroundColor Green

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Build installer
& $nsisPath "/V3" "installer.nsi"

if ($LASTEXITCODE -eq 0) {
    $exePath = Join-Path (Resolve-Path $OutputDir) "NeolithicTERA-Setup.exe"
    if (Test-Path $exePath) {
        $exeSize = (Get-Item $exePath).Length / 1MB
        
        Write-Host "`n========================================" -ForegroundColor Green
        Write-Host "INSTALLER BUILT SUCCESSFULLY!" -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Green
        Write-Host "Output: $exePath" -ForegroundColor Cyan
        Write-Host "Size: $([math]::Round($exeSize, 2)) MB" -ForegroundColor Cyan
        Write-Host "`nTotal content size:" -ForegroundColor Yellow
        Write-Host "  Launcher: ~20 MB" -ForegroundColor Cyan
        Write-Host "  Binaries: $([math]::Round($binariesSize, 2)) MB" -ForegroundColor Cyan
        Write-Host "  Engine: $([math]::Round($engineSize, 2)) MB" -ForegroundColor Cyan
        Write-Host "  Total: ~$([math]::Round($binariesSize + $engineSize + 20, 2)) MB" -ForegroundColor Cyan
    } else {
        Write-Host "`nERROR: Installer file not found at expected location!" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "`nERROR: NSIS build failed!" -ForegroundColor Red
    exit 1
}

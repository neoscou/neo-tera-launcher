<#
.SYNOPSIS
    Updates DataCenter file to R2 in one command

.DESCRIPTION
    This script performs all steps needed to update the DataCenter file:
    1. Updates the hash-file.json with the new DataCenter hash
    2. Uploads the updated hash file to R2
    3. Uploads the DataCenter file to R2

.PARAMETER DataCenterPath
    Path to the DataCenter file (default: D:\V100TERA\Novadrop\OutputDC\DataCenter_Final_EUR.dat)

.EXAMPLE
    .\Update-DataCenter-To-R2.ps1

.EXAMPLE
    .\Update-DataCenter-To-R2.ps1 -DataCenterPath "D:\V100TERA\Novadrop\OutputDC\DataCenter_Final_EUR.dat"
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$DataCenterPath = "D:\V100TERA\Novadrop\OutputDC\DataCenter_Final_EUR.dat"
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "DataCenter Update to R2 - Complete Workflow" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

# Verify DataCenter file exists
if (-not (Test-Path $DataCenterPath)) {
    Write-Host "ERROR: DataCenter file not found at: $DataCenterPath" -ForegroundColor Red
    exit 1
}

$DataCenterFileName = Split-Path $DataCenterPath -Leaf
Write-Host "DataCenter file: $DataCenterFileName" -ForegroundColor Green
Write-Host "Source path: $DataCenterPath" -ForegroundColor Gray
Write-Host ""

# Step 1: Update hash-file.json
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host "STEP 1: Updating hash-file.json" -ForegroundColor Yellow
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host ""

try {
    & "$PSScriptRoot\Update-R2File.ps1" -LocalFilePath $DataCenterPath -R2RelativePath "S1Game/S1Data/$DataCenterFileName"
    
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne $null) {
        throw "Update-R2File.ps1 failed with exit code $LASTEXITCODE"
    }
    
    Write-Host ""
    Write-Host "[OK] Hash file updated successfully" -ForegroundColor Green
} catch {
    Write-Host ""
    Write-Host "[ERROR] Failed to update hash file: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Start-Sleep -Seconds 1

# Step 2: Upload hash file to R2
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host "STEP 2: Uploading hash-file.json to R2" -ForegroundColor Yellow
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host ""

try {
    rclone copy "$PSScriptRoot\hash-file.json" "r2:tera/TeraDirect/" --progress
    
    if ($LASTEXITCODE -ne 0) {
        throw "rclone failed with exit code $LASTEXITCODE"
    }
    
    Write-Host ""
    Write-Host "[OK] Hash file uploaded to R2 successfully" -ForegroundColor Green
} catch {
    Write-Host ""
    Write-Host "[ERROR] Failed to upload hash file: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Start-Sleep -Seconds 1

# Step 3: Upload DataCenter file to R2
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host "STEP 3: Uploading $DataCenterFileName to R2" -ForegroundColor Yellow
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host ""

try {
    rclone copy $DataCenterPath "r2:tera/TeraDirect/S1Game/S1Data/" --progress
    
    if ($LASTEXITCODE -ne 0) {
        throw "rclone failed with exit code $LASTEXITCODE"
    }
    
    Write-Host ""
    Write-Host "[OK] DataCenter file uploaded to R2 successfully" -ForegroundColor Green
} catch {
    Write-Host ""
    Write-Host "[ERROR] Failed to upload DataCenter file: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host "SUCCESS - All steps completed!" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green
Write-Host ""
Write-Host "DataCenter update is now live on R2:" -ForegroundColor Cyan
Write-Host "  File: $DataCenterFileName" -ForegroundColor White
Write-Host "  URL: https://www.neolithictera.com/TeraDirect/S1Game/S1Data/$DataCenterFileName" -ForegroundColor White
Write-Host ""
Write-Host "Players can now:" -ForegroundColor Yellow
Write-Host "  1. Restart the launcher (if running)" -ForegroundColor White
Write-Host "  2. Click Check/Repair" -ForegroundColor White
Write-Host "  3. The launcher will auto-download the updated DataCenter" -ForegroundColor White
Write-Host ""

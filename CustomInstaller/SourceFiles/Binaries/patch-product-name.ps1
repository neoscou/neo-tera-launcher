# Script to modify TERA.exe ProductName for Discord Rich Presence
# This edits the VERSION_INFO resource in the PE file

param(
    [string]$ExePath = "D:\V100TERA\Neolithic Test Server\Binaries\TERA.exe",
    [string]$BackupPath = "D:\V100TERA\Neolithic Test Server\Binaries\TERA.exe.backup",
    [string]$NewProductName = "Neolithic TERA"
)

Write-Host "=== TERA.exe Product Name Patcher for Discord ===" -ForegroundColor Cyan
Write-Host ""

# Check if file exists
if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: TERA.exe not found at: $ExePath" -ForegroundColor Red
    exit 1
}

# Create backup
if (-not (Test-Path $BackupPath)) {
    Write-Host "Creating backup..." -ForegroundColor Yellow
    Copy-Item $ExePath $BackupPath -Force
    Write-Host "Backup created: $BackupPath" -ForegroundColor Green
}

Write-Host ""
Write-Host "MANUAL STEPS REQUIRED:" -ForegroundColor Yellow
Write-Host "1. Download Resource Hacker from: http://www.angusj.com/resourcehacker/" -ForegroundColor White
Write-Host "2. Open TERA.exe in Resource Hacker"
Write-Host "3. Expand 'Version Info' in the left pane"
Write-Host "4. Click on '1' under 'Version Info'"
Write-Host "5. Find the line with 'VALUE `"ProductName`"'"
Write-Host "6. Change it to: VALUE `"ProductName`", `"$NewProductName`""
Write-Host "7. Click 'Compile Script' button"
Write-Host "8. File -> Save"
Write-Host "9. Restart Discord"
Write-Host ""
Write-Host "After these steps, Discord will show '$NewProductName' when playing." -ForegroundColor Green

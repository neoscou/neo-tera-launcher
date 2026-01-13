# Upload Launcher Update to R2
# This script uploads the launcher executable and manifest to R2 for auto-updates

param(
    [string]$BucketName = "neolithic-tera",
    [string]$R2AccountId = "YOUR_R2_ACCOUNT_ID",
    [string]$AccessKeyId = "YOUR_ACCESS_KEY_ID",
    [string]$SecretAccessKey = "YOUR_SECRET_ACCESS_KEY"
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Launcher Update R2 Upload Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Paths
$LauncherExe = "Neolithic-TERA-Launcher-v1.1.0.exe"
$ManifestJson = "launcher-update-manifest.json"
$ScriptDir = $PSScriptRoot

# Check if files exist
if (!(Test-Path "$ScriptDir\$LauncherExe")) {
    Write-Host "ERROR: $LauncherExe not found!" -ForegroundColor Red
    exit 1
}

if (!(Test-Path "$ScriptDir\$ManifestJson")) {
    Write-Host "ERROR: $ManifestJson not found!" -ForegroundColor Red
    exit 1
}

Write-Host "[1/4] Files validated" -ForegroundColor Green
Write-Host "  Launcher: $LauncherExe" -ForegroundColor Gray
Write-Host "  Manifest: $ManifestJson" -ForegroundColor Gray
Write-Host ""

# Configure AWS CLI for R2
$env:AWS_ACCESS_KEY_ID = $AccessKeyId
$env:AWS_SECRET_ACCESS_KEY = $SecretAccessKey
$R2Endpoint = "https://$R2AccountId.r2.cloudflarestorage.com"

Write-Host "[2/4] Uploading launcher executable..." -ForegroundColor Yellow
aws s3 cp "$ScriptDir\$LauncherExe" "s3://$BucketName/launcher/$LauncherExe" `
    --endpoint-url $R2Endpoint `
    --content-type "application/x-msdownload"

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to upload launcher executable" -ForegroundColor Red
    exit 1
}

Write-Host "  ✓ Launcher uploaded successfully" -ForegroundColor Green
Write-Host ""

Write-Host "[3/4] Uploading update manifest..." -ForegroundColor Yellow
aws s3 cp "$ScriptDir\$ManifestJson" "s3://$BucketName/launcher/update-manifest.json" `
    --endpoint-url $R2Endpoint `
    --content-type "application/json"

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to upload manifest" -ForegroundColor Red
    exit 1
}

Write-Host "  ✓ Manifest uploaded successfully" -ForegroundColor Green
Write-Host ""

Write-Host "[4/4] Verifying uploads..." -ForegroundColor Yellow
aws s3 ls "s3://$BucketName/launcher/" --endpoint-url $R2Endpoint | Select-String -Pattern "Neolithic-TERA-Launcher|update-manifest"

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "UPLOAD COMPLETE!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "URLs:" -ForegroundColor Cyan
Write-Host "  Launcher: https://pub-aa9e57c7b27840c0b57af8dc3ee4b62d.r2.dev/launcher/$LauncherExe" -ForegroundColor Gray
Write-Host "  Manifest: https://pub-aa9e57c7b27840c0b57af8dc3ee4b62d.r2.dev/launcher/update-manifest.json" -ForegroundColor Gray
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Test the update URL in a browser" -ForegroundColor Gray
Write-Host "2. Launch an older version of the launcher to test auto-update" -ForegroundColor Gray
Write-Host "3. Verify the changelog displays correctly" -ForegroundColor Gray

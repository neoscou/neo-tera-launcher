# Test what the launcher should detect for updates
Write-Host "=== Launcher Update Detection Test ===" -ForegroundColor Cyan
Write-Host ""

# Download hash file from server
Write-Host "1. Downloading hash-file.json from server..." -ForegroundColor Yellow
try {
    $hashFileUrl = "https://www.neolithictera.com/TeraDirect/hash-file.json"
    $response = Invoke-WebRequest -Uri $hashFileUrl -UseBasicParsing
    $hashFileJson = [System.Text.Encoding]::UTF8.GetString($response.Content) | ConvertFrom-Json
    Write-Host "   ✓ Downloaded successfully ($($hashFileJson.files.Count) files)" -ForegroundColor Green
} catch {
    Write-Host "   ✗ Failed to download: $_" -ForegroundColor Red
    exit 1
}

# Find DataCenter entry
Write-Host ""
Write-Host "2. Finding DataCenter entry in hash file..." -ForegroundColor Yellow
$dcEntry = $hashFileJson.files | Where-Object { $_.path -eq "S1Game/S1Data/DataCenter_Final_EUR.dat" }
if ($dcEntry) {
    Write-Host "   ✓ Found DataCenter entry:" -ForegroundColor Green
    Write-Host "     Server Hash: $($dcEntry.hash)" -ForegroundColor White
    Write-Host "     Server Size: $($dcEntry.size)" -ForegroundColor White
    Write-Host "     URL: $($dcEntry.url)" -ForegroundColor White
} else {
    Write-Host "   ✗ DataCenter entry not found in hash file!" -ForegroundColor Red
    exit 1
}

# Check local file
Write-Host ""
Write-Host "3. Checking local file..." -ForegroundColor Yellow
$localPath = "D:\V100TERA\Neolithic Test Server\S1Game\S1Data\DataCenter_Final_EUR.dat"
if (Test-Path $localPath) {
    $localHash = (Get-FileHash -Path $localPath -Algorithm SHA256).Hash
    $localSize = (Get-Item $localPath).Length
    Write-Host "   ✓ Local file found:" -ForegroundColor Green
    Write-Host "     Local Hash: $localHash" -ForegroundColor White
    Write-Host "     Local Size: $localSize" -ForegroundColor White
} else {
    Write-Host "   ✗ Local file not found!" -ForegroundColor Red
    exit 1
}

# Compare
Write-Host ""
Write-Host "4. Comparison:" -ForegroundColor Yellow
if ($localHash.ToUpper() -ne $dcEntry.hash.ToUpper()) {
    Write-Host "   ✓ HASHES DIFFER - UPDATE REQUIRED!" -ForegroundColor Green -BackgroundColor DarkGreen
    Write-Host ""
    Write-Host "   The launcher SHOULD detect this file for update." -ForegroundColor Cyan
} else {
    Write-Host "   ✗ Hashes match - No update needed" -ForegroundColor Red
}

if ($localSize -ne $dcEntry.size) {
    Write-Host "   ✓ SIZES DIFFER - UPDATE REQUIRED!" -ForegroundColor Green
} else {
    Write-Host "   ✗ Sizes match" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== Test Complete ===" -ForegroundColor Cyan

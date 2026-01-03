<#
.SYNOPSIS
    Updates a file in Cloudflare R2 bucket and updates the hash-file.json

.DESCRIPTION
    This script calculates the SHA-256 hash of a local file, uploads it to R2,
    and updates the hash-file.json with the new hash and size.

.PARAMETER LocalFilePath
    Path to the local file to upload

.PARAMETER R2RelativePath
    Relative path in R2 (e.g., "S1Game/S1Data/DataCenter_Final_EUR.dat")

.EXAMPLE
    .\Update-R2File.ps1 -LocalFilePath "D:\V100TERA\Novadrop\OutputDC\DataCenter_Final_EUR.dat" -R2RelativePath "S1Game/S1Data/DataCenter_Final_EUR.dat"

.NOTES
    Requires AWS CLI or rclone to be configured for R2 access
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$LocalFilePath = "D:\V100TERA\Novadrop\OutputDC\DataCenter_Final_EUR.dat",
    
    [Parameter(Mandatory=$false)]
    [string]$R2RelativePath = "S1Game/S1Data/DataCenter_Final_EUR.dat",
    
    [Parameter(Mandatory=$false)]
    [string]$HashFileUrl = "https://www.neolithictera.com/TeraDirect/hash-file.json",
    
    [Parameter(Mandatory=$false)]
    [string]$FileServerUrl = "https://www.neolithictera.com/TeraDirect"
)

function Get-SHA256Hash {
    param([string]$FilePath)
    
    $hash = Get-FileHash -Path $FilePath -Algorithm SHA256
    return $hash.Hash.ToUpper()
}

function Get-FileSize {
    param([string]$FilePath)
    
    return (Get-Item $FilePath).Length
}

# Main script
Write-Host "=" -NoNewline; Write-Host ("=" * 59)
Write-Host "R2 File Update Tool (PowerShell)"
Write-Host "=" -NoNewline; Write-Host ("=" * 59)

# Validate file exists
if (-not (Test-Path $LocalFilePath)) {
    Write-Host "ERROR: File not found: $LocalFilePath" -ForegroundColor Red
    exit 1
}

Write-Host "Local file: $LocalFilePath" -ForegroundColor Cyan
Write-Host "R2 path: tera/TeraDirect/$R2RelativePath" -ForegroundColor Cyan
Write-Host ""

# Step 1: Calculate hash and get size
Write-Host "Step 1: Calculating file hash and size..." -ForegroundColor Yellow
$fileHash = Get-SHA256Hash -FilePath $LocalFilePath
$fileSize = Get-FileSize -FilePath $LocalFilePath
$fileSizeMB = [math]::Round($fileSize / 1MB, 2)

Write-Host "  Hash (SHA-256): $fileHash" -ForegroundColor Green
Write-Host "  Size: $($fileSize.ToString('N0')) bytes ($fileSizeMB MB)" -ForegroundColor Green
Write-Host ""

# Step 2: Download current hash file
Write-Host "Step 2: Downloading current hash-file.json..." -ForegroundColor Yellow
try {
    # Download as text to preserve structure
    $hashFileText = (Invoke-WebRequest -Uri $HashFileUrl -UseBasicParsing).Content.TrimStart([char]0xFEFF)
    $hashFileContent = $hashFileText | ConvertFrom-Json
    Write-Host "  Hash file downloaded successfully" -ForegroundColor Green
} catch {
    Write-Host "  WARNING: Could not download hash file: $_" -ForegroundColor Yellow
    Write-Host "  Creating new hash file structure..." -ForegroundColor Yellow
    $hashFileContent = @{
        files = @()
        directories = @()
    }
}

# Ensure 'files' property exists (fix for incomplete hash files)
if (-not $hashFileContent.files) {
    Write-Host "  Warning: 'files' property was missing. Adding empty files array..." -ForegroundColor Yellow
    $hashFileContent | Add-Member -NotePropertyName "files" -NotePropertyValue @() -Force
}
Write-Host "  Current file count: $($hashFileContent.files.Count)" -ForegroundColor Green
Write-Host ""

# Step 3: Update hash entry
Write-Host "Step 3: Updating hash entry..." -ForegroundColor Yellow
$fileUrl = "$FileServerUrl/$R2RelativePath"

$newEntry = @{
    path = $R2RelativePath
    hash = $fileHash
    size = $fileSize
    url = $fileUrl
}

# Find and update existing entry or add new one
$existingIndex = -1
for ($i = 0; $i -lt $hashFileContent.files.Count; $i++) {
    if ($hashFileContent.files[$i].path -eq $R2RelativePath) {
        $existingIndex = $i
        break
    }
}

if ($existingIndex -ge 0) {
    Write-Host "  Updating existing entry for $R2RelativePath" -ForegroundColor Green
    $hashFileContent.files[$existingIndex] = $newEntry
} else {
    Write-Host "  Adding new entry for $R2RelativePath" -ForegroundColor Green
    $hashFileContent.files += $newEntry
}
Write-Host ""

# Step 4: Save updated hash file locally
Write-Host "Step 4: Saving updated hash-file.json locally..." -ForegroundColor Yellow
$outputHashFile = Join-Path $PSScriptRoot "hash-file.json"
# Use UTF8NoBOM to prevent BOM that breaks JSON parsing
# Ensure the structure is preserved as {"files": [...], "directories": [...]}
$json = $hashFileContent | ConvertTo-Json -Depth 10 -Compress:$false
[System.IO.File]::WriteAllText($outputHashFile, $json, [System.Text.UTF8Encoding]::new($false))
Write-Host "  Saved to: $outputHashFile (UTF-8 without BOM)" -ForegroundColor Green
Write-Host ""

# Display summary
Write-Host "=" -NoNewline; Write-Host ("=" * 59)
Write-Host "Summary" -ForegroundColor Cyan
Write-Host "=" -NoNewline; Write-Host ("=" * 59)
Write-Host ""
Write-Host "File Information:" -ForegroundColor Cyan
Write-Host "  Path: $R2RelativePath"
Write-Host "  Hash: $fileHash"
Write-Host "  Size: $($fileSize.ToString('N0')) bytes ($fileSizeMB MB)"
Write-Host "  URL: $fileUrl"
Write-Host ""
Write-Host "Updated hash file saved to:" -ForegroundColor Cyan
Write-Host "  $outputHashFile"
Write-Host ""
Write-Host "NEXT STEPS:" -ForegroundColor Yellow
Write-Host "1. Upload the file to R2:" -ForegroundColor White
Write-Host "   Local: $LocalFilePath" -ForegroundColor Gray
Write-Host "   R2: tera/teradirect/$R2RelativePath" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Upload the updated hash file to R2:" -ForegroundColor White
Write-Host "   Local: $outputHashFile" -ForegroundColor Gray
Write-Host "   R2: tera/TeraDirect/hash-file.json" -ForegroundColor Gray
Write-Host ""
Write-Host "After uploading both files, users can update by:" -ForegroundColor Yellow
Write-Host "  - Restarting the launcher" -ForegroundColor Gray
Write-Host "  - Clicking the Check/Repair button" -ForegroundColor Gray
Write-Host ""

# Copy hash entry to clipboard if available
try {
    $newEntry | ConvertTo-Json | Set-Clipboard
    Write-Host "Hash entry copied to clipboard!" -ForegroundColor Green
} catch {
    # Clipboard not available, skip
}

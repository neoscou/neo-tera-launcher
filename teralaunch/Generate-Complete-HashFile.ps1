# Generate complete hash file from local S1Game folder
param(
    [string]$S1GamePath = "D:\V100TERA\Neolithic Test Server\S1Game",
    [string]$FileServerUrl = "https://www.neolithictera.com/TeraDirect",
    [string]$OutputFile = "hash-file.json"
)

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Generate Complete Hash File" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

if (-not (Test-Path $S1GamePath)) {
    Write-Host "ERROR: S1Game folder not found at: $S1GamePath" -ForegroundColor Red
    exit 1
}

Write-Host "Scanning folder: $S1GamePath" -ForegroundColor Yellow
Write-Host "This may take several minutes for large game installations..." -ForegroundColor Yellow
Write-Host ""

$hashData = @{
    files = @()
    directories = @()
}

# Get all files recursively, excluding log files and temporary files
$allFiles = Get-ChildItem -Path $S1GamePath -Recurse -File | Where-Object {
    $relativePath = $_.FullName.Substring($S1GamePath.Length + 1)
    # Exclude log files, temp files, cache, and screenshots
    $relativePath -notmatch '\\Logs\\' -and
    $relativePath -notmatch '\\Cache\\' -and  
    $relativePath -notmatch '\\ScreenShots\\' -and
    $relativePath -notmatch '\.log$' -and
    $relativePath -notmatch '\.tmp$' -and
    $relativePath -notmatch '\.temp$'
}
$totalFiles = $allFiles.Count
$current = 0

Write-Host "Found $totalFiles files to process" -ForegroundColor Green
Write-Host ""

# Track directories for summary
$directorySummary = @{}

foreach ($file in $allFiles) {
    $current++
    $percentComplete = [math]::Round(($current / $totalFiles) * 100, 1)
    
    # Calculate relative path from S1Game
    $relativePath = $file.FullName.Substring($S1GamePath.Length + 1).Replace('\', '/')
    $relativePath = "S1Game/$relativePath"
    
    # Show progress every 100 files
    if ($current % 100 -eq 0 -or $current -eq $totalFiles) {
        Write-Host "[$current/$totalFiles] ($percentComplete%) Processing: $relativePath" -ForegroundColor Gray
    }
    
    # Calculate SHA-256 hash
    $hash = (Get-FileHash -Path $file.FullName -Algorithm SHA256).Hash
    
    # Get file size
    $size = $file.Length
    
    # Build URL
    $url = "$FileServerUrl/$relativePath"
    
    # Add to files array
    $hashData.files += @{
        path = $relativePath
        hash = $hash
        size = $size
        url = $url
    }
    
    # Track directory stats
    $dirPath = [System.IO.Path]::GetDirectoryName($relativePath).Replace('\', '/')
    if (-not $directorySummary.ContainsKey($dirPath)) {
        $directorySummary[$dirPath] = @{
            file_count = 0
            total_size = 0
            hashes = @()
        }
    }
    $directorySummary[$dirPath].file_count++
    $directorySummary[$dirPath].total_size += $size
    $directorySummary[$dirPath].hashes += $hash
}

Write-Host ""
Write-Host "Generating directory entries..." -ForegroundColor Yellow

# Generate directory entries with combined hash
foreach ($dirPath in $directorySummary.Keys | Sort-Object) {
    $dirInfo = $directorySummary[$dirPath]
    
    # Create combined hash from all file hashes in directory
    $combinedString = ($dirInfo.hashes | Sort-Object) -join ''
    $combinedBytes = [System.Text.Encoding]::UTF8.GetBytes($combinedString)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    $dirHash = [System.BitConverter]::ToString($sha256.ComputeHash($combinedBytes)).Replace('-', '').ToLower()
    
    $hashData.directories += @{
        path = $dirPath
        hash = $dirHash
        file_count = $dirInfo.file_count
        total_size = $dirInfo.total_size
    }
}

Write-Host "  Created $($hashData.directories.Count) directory entries" -ForegroundColor Green

Write-Host ""
Write-Host "Saving hash file..." -ForegroundColor Yellow

# Save as JSON without BOM
$json = $hashData | ConvertTo-Json -Depth 10 -Compress:$false
$outputPath = Join-Path $PSScriptRoot $OutputFile
[System.IO.File]::WriteAllText($outputPath, $json, [System.Text.UTF8Encoding]::new($false))

$fileSize = (Get-Item $outputPath).Length
$fileSizeMB = [math]::Round($fileSize / 1MB, 2)

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Complete!" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Hash file generated:" -ForegroundColor Green
Write-Host "  Path: $outputPath"
Write-Host "  Files tracked: $totalFiles"
Write-Host "  Size: $($fileSize.ToString('N0')) bytes ($fileSizeMB MB)"
Write-Host ""
Write-Host "Next step: Upload to R2" -ForegroundColor Yellow
Write-Host "  rclone copy `"$outputPath`" r2:tera/TeraDirect/ --progress"
Write-Host ""

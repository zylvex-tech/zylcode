#!/usr/bin/env pwsh
# Simple ZylCode Desktop Diagnostic

Write-Host "ZylCode Desktop Diagnostic" -ForegroundColor Cyan
Write-Host "=========================" -ForegroundColor Cyan
Write-Host ""

# Check binary
Write-Host "1. Checking binary..." -ForegroundColor Yellow
if (Test-Path "target\release\zylcode-desktop.exe") {
    Write-Host "   ✓ Binary found" -ForegroundColor Green
} else {
    Write-Host "   ✗ Binary not found" -ForegroundColor Red
    exit 1
}

# Check file size
Write-Host "2. Checking file size..." -ForegroundColor Yellow
$fileSize = (Get-Item "target\release\zylcode-desktop.exe").Length
$fileSizeMB = [math]::Round($fileSize / 1MB, 2)
Write-Host "   File size: $fileSizeMB MB" -ForegroundColor Gray

# Try to launch
Write-Host "3. Testing launch..." -ForegroundColor Yellow
Write-Host "   Launching application..." -ForegroundColor Gray

$process = Start-Process -FilePath "target\release\zylcode-desktop.exe" -PassThru
Start-Sleep -Seconds 3

if ($process.HasExited) {
    Write-Host "   ✗ Application exited immediately" -ForegroundColor Red
    Write-Host "   Exit code: $($process.ExitCode)" -ForegroundColor Red
} else {
    Write-Host "   ✓ Application is running" -ForegroundColor Green
    Write-Host "   Process ID: $($process.Id)" -ForegroundColor Gray
    
    # Wait a bit more
    Start-Sleep -Seconds 2
    
    # Stop the process
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    Write-Host "   Application stopped" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Diagnostic complete." -ForegroundColor Cyan
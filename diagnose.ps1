#!/usr/bin/env pwsh
# ZylCode Desktop App Diagnostic Script

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "ZylCode Desktop App Diagnostics" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check 1: Binary exists
Write-Host "1. Checking binary..." -ForegroundColor Yellow
if (Test-Path "target\release\zylcode-desktop.exe") {
    Write-Host "   ✓ Binary found" -ForegroundColor Green
} else {
    Write-Host "   ✗ Binary not found" -ForegroundColor Red
    Write-Host "   Please build the project: cargo build --release" -ForegroundColor Yellow
    exit 1
}

# Check 2: File size
Write-Host "2. Checking file size..." -ForegroundColor Yellow
$fileSize = (Get-Item "target\release\zylcode-desktop.exe").Length
$fileSizeMB = [math]::Round($fileSize / 1MB, 2)
Write-Host "   File size: $fileSizeMB MB" -ForegroundColor Gray
if ($fileSize -gt 1MB) {
    Write-Host "   ✓ File size looks good" -ForegroundColor Green
} else {
    Write-Host "   ✗ File size is too small" -ForegroundColor Red
}

# Check 3: Dependencies
Write-Host "3. Checking dependencies..." -ForegroundColor Yellow
$vcRedist = Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" -ErrorAction SilentlyContinue
if ($vcRedist) {
    Write-Host "   ✓ Visual C++ Redistributable installed" -ForegroundColor Green
} else {
    Write-Host "   ✗ Visual C++ Redistributable not found" -ForegroundColor Red
    Write-Host "   Download from: https://aka.ms/vs/17/release/vc_redist.x64.exe" -ForegroundColor Yellow
}

# Check 4: Windows Defender
Write-Host "4. Checking Windows Defender..." -ForegroundColor Yellow
$defenderStatus = Get-MpPreference -ErrorAction SilentlyContinue
if ($defenderStatus) {
    Write-Host "   Windows Defender is active" -ForegroundColor Gray
    Write-Host "   If app is blocked, add exclusion:" -ForegroundColor Yellow
    Write-Host "   Add-MpPreference -ExclusionPath 'C:\Projects\zylcode'" -ForegroundColor Gray
}

# Check 5: Antivirus
Write-Host "5. Checking antivirus..." -ForegroundColor Yellow
$avProducts = Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntivirusProduct -ErrorAction SilentlyContinue
if ($avProducts) {
    Write-Host "   Antivirus products found:" -ForegroundColor Gray
    foreach ($av in $avProducts) {
        Write-Host "   - $($av.displayName)" -ForegroundColor Gray
    }
    Write-Host "   If app is blocked, add exclusion in your antivirus" -ForegroundColor Yellow
}

# Check 6: Event Log
Write-Host "6. Checking Event Log..." -ForegroundColor Yellow
$events = Get-WinEvent -LogName Application -FilterXPath "*[System[Provider[@Name='Application Error'] and TimeCreated[timediff(@SystemTime) <= 300000]]]" -MaxEvents 5 -ErrorAction SilentlyContinue
if ($events) {
    Write-Host "   Recent application errors found:" -ForegroundColor Yellow
    foreach ($event in $events) {
        Write-Host "   - $($event.TimeCreated): $($event.Message)" -ForegroundColor Gray
    }
} else {
    Write-Host "   No recent application errors" -ForegroundColor Green
}

# Check 7: Try to launch with debug output
Write-Host "7. Testing launch..." -ForegroundColor Yellow
Write-Host "   Launching with debug output..." -ForegroundColor Gray

$env:RUST_LOG = "debug"
$process = Start-Process -FilePath "target\release\zylcode-desktop.exe" -PassThru -RedirectStandardError "zylcode-stderr.log" -RedirectStandardOutput "zylcode-stdout.log"

Start-Sleep -Seconds 3

if ($process.HasExited) {
    Write-Host "   ✗ Application exited immediately" -ForegroundColor Red
    Write-Host "   Exit code: $($process.ExitCode)" -ForegroundColor Red
    
    if (Test-Path "zylcode-stderr.log") {
        Write-Host "   Error output:" -ForegroundColor Yellow
        Get-Content "zylcode-stderr.log" -Tail 20
    }
    
    if (Test-Path "zylcode-stdout.log") {
        Write-Host "   Standard output:" -ForegroundColor Yellow
        Get-Content "zylcode-stdout.log" -Tail 20
    }
} else {
    Write-Host "   ✓ Application is running" -ForegroundColor Green
    Write-Host "   Process ID: $($process.Id)" -ForegroundColor Gray
    
    # Check if window is visible
    Start-Sleep -Seconds 2
    $window = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
    if ($window) {
        Write-Host "   ✓ Process is still running" -ForegroundColor Green
    } else {
        Write-Host "   ✗ Process disappeared" -ForegroundColor Red
    }
    
    # Stop the process
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
}

# Summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Diagnostic Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Common issues and solutions:" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Application starts but no window:" -ForegroundColor White
Write-Host "   - Check if window is off-screen (Win+Arrow keys)" -ForegroundColor Gray
Write-Host "   - Try running as administrator" -ForegroundColor Gray
Write-Host "   - Check antivirus/Windows Defender" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Application crashes immediately:" -ForegroundColor White
Write-Host "   - Install Visual C++ Redistributable" -ForegroundColor Gray
Write-Host "   - Check Event Log for error details" -ForegroundColor Gray
Write-Host "   - Run with debug output" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Build issues:" -ForegroundColor White
Write-Host "   - Ensure Rust, Node.js, pnpm are installed" -ForegroundColor Gray
Write-Host "   - Run: cargo clean && cargo build --release" -ForegroundColor Gray
Write-Host ""
Write-Host "For more help, see INSTALLATION_GUIDE.md" -ForegroundColor Gray
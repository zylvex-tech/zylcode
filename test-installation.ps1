#!/usr/bin/env pwsh
# ZylCode Installation Test Script
# This script tests if ZylCode is properly installed and working

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "ZylCode Installation Test" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Test 1: Check if Rust is installed
Write-Host "Test 1: Checking Rust installation..." -ForegroundColor Yellow
try {
    $rustVersion = rustc --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Rust is installed: $rustVersion" -ForegroundColor Green
    } else {
        Write-Host "✗ Rust is not installed" -ForegroundColor Red
        Write-Host "  Please install Rust from https://rustup.rs/" -ForegroundColor Yellow
        exit 1
    }
} catch {
    Write-Host "✗ Rust is not installed" -ForegroundColor Red
    Write-Host "  Please install Rust from https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

# Test 2: Check if Node.js is installed
Write-Host ""
Write-Host "Test 2: Checking Node.js installation..." -ForegroundColor Yellow
try {
    $nodeVersion = node --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Node.js is installed: $nodeVersion" -ForegroundColor Green
    } else {
        Write-Host "✗ Node.js is not installed" -ForegroundColor Red
        Write-Host "  Please install Node.js from https://nodejs.org/" -ForegroundColor Yellow
        exit 1
    }
} catch {
    Write-Host "✗ Node.js is not installed" -ForegroundColor Red
    Write-Host "  Please install Node.js from https://nodejs.org/" -ForegroundColor Yellow
    exit 1
}

# Test 3: Check if pnpm is installed
Write-Host ""
Write-Host "Test 3: Checking pnpm installation..." -ForegroundColor Yellow
try {
    $pnpmVersion = pnpm --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ pnpm is installed: $pnpmVersion" -ForegroundColor Green
    } else {
        Write-Host "✗ pnpm is not installed" -ForegroundColor Red
        Write-Host "  Installing pnpm..." -ForegroundColor Yellow
        npm install -g pnpm
        if ($LASTEXITCODE -ne 0) {
            Write-Host "  Failed to install pnpm" -ForegroundColor Red
            exit 1
        }
    }
} catch {
    Write-Host "✗ pnpm is not installed" -ForegroundColor Red
    Write-Host "  Installing pnpm..." -ForegroundColor Yellow
    npm install -g pnpm
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Failed to install pnpm" -ForegroundColor Red
        exit 1
    }
}

# Test 4: Check if cargo is installed
Write-Host ""
Write-Host "Test 4: Checking cargo installation..." -ForegroundColor Yellow
try {
    $cargoVersion = cargo --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ cargo is installed: $cargoVersion" -ForegroundColor Green
    } else {
        Write-Host "✗ cargo is not installed" -ForegroundColor Red
        Write-Host "  Please install Rust from https://rustup.rs/" -ForegroundColor Yellow
        exit 1
    }
} catch {
    Write-Host "✗ cargo is not installed" -ForegroundColor Red
    Write-Host "  Please install Rust from https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

# Test 5: Check if we're in the ZylCode directory
Write-Host ""
Write-Host "Test 5: Checking ZylCode directory..." -ForegroundColor Yellow
if (Test-Path "Cargo.toml") {
    Write-Host "✓ ZylCode directory found" -ForegroundColor Green
} else {
    Write-Host "✗ ZylCode directory not found" -ForegroundColor Red
    Write-Host "  Please run this script from the ZylCode directory" -ForegroundColor Yellow
    exit 1
}

# Test 6: Build the project
Write-Host ""
Write-Host "Test 6: Building ZylCode..." -ForegroundColor Yellow
Write-Host "  This may take a few minutes..." -ForegroundColor Gray

cargo build --release 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Backend build successful" -ForegroundColor Green
} else {
    Write-Host "✗ Backend build failed" -ForegroundColor Red
    Write-Host "  Check the error messages above" -ForegroundColor Yellow
    exit 1
}

# Test 7: Build frontend
Write-Host ""
Write-Host "Test 7: Building frontend..." -ForegroundColor Yellow
Set-Location apps/zylcode-desktop
pnpm install 2>&1 | Out-Null
pnpm build 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Frontend build successful" -ForegroundColor Green
} else {
    Write-Host "✗ Frontend build failed" -ForegroundColor Red
    Write-Host "  Check the error messages above" -ForegroundColor Yellow
    exit 1
}
Set-Location ../..

# Test 8: Run tests
Write-Host ""
Write-Host "Test 8: Running tests..." -ForegroundColor Yellow
cargo test --package zylcode-mcp 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ All tests passed" -ForegroundColor Green
} else {
    Write-Host "✗ Some tests failed" -ForegroundColor Red
    Write-Host "  Check the error messages above" -ForegroundColor Yellow
    exit 1
}

# Test 9: Check binary exists
Write-Host ""
Write-Host "Test 9: Checking binary..." -ForegroundColor Yellow
if (Test-Path "target/release/zylcode.exe") {
    Write-Host "✓ ZylCode binary found" -ForegroundColor Green
} else {
    Write-Host "✗ ZylCode binary not found" -ForegroundColor Red
    exit 1
}

# Test 10: Run quick test
Write-Host ""
Write-Host "Test 10: Running quick test..." -ForegroundColor Yellow
$testOutput = & "target/release/zylcode.exe" --version 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ ZylCode is working: $testOutput" -ForegroundColor Green
} else {
    Write-Host "✗ ZylCode failed to run" -ForegroundColor Red
    exit 1
}

# Summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Installation Test Complete!" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "✓ All tests passed!" -ForegroundColor Green
Write-Host ""
Write-Host "You can now run ZylCode:" -ForegroundColor Yellow
Write-Host "  1. GUI: .\target\release\zylcode-desktop.exe" -ForegroundColor White
Write-Host "  2. CLI: .\target\release\zylcode.exe --help" -ForegroundColor White
Write-Host ""
Write-Host "For more information, see the User Guide." -ForegroundColor Gray
#!/usr/bin/env pwsh
# Simple ZylCode Test Script

Write-Host "Testing ZylCode Installation..." -ForegroundColor Cyan

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "Error: Please run this script from the ZylCode directory" -ForegroundColor Red
    exit 1
}

# Test 1: Check Rust
Write-Host "1. Checking Rust..." -ForegroundColor Yellow
$rustVersion = rustc --version 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ Rust: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "   ✗ Rust not found" -ForegroundColor Red
    exit 1
}

# Test 2: Check Node.js
Write-Host "2. Checking Node.js..." -ForegroundColor Yellow
$nodeVersion = node --version 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ Node.js: $nodeVersion" -ForegroundColor Green
} else {
    Write-Host "   ✗ Node.js not found" -ForegroundColor Red
    exit 1
}

# Test 3: Check pnpm
Write-Host "3. Checking pnpm..." -ForegroundColor Yellow
$pnpmVersion = pnpm --version 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ pnpm: $pnpmVersion" -ForegroundColor Green
} else {
    Write-Host "   ✗ pnpm not found" -ForegroundColor Red
    exit 1
}

# Test 4: Build MCP package
Write-Host "4. Building MCP package..." -ForegroundColor Yellow
cargo build --package zylcode-mcp 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ MCP package built successfully" -ForegroundColor Green
} else {
    Write-Host "   ✗ MCP package build failed" -ForegroundColor Red
    exit 1
}

# Test 5: Run MCP tests
Write-Host "5. Running MCP tests..." -ForegroundColor Yellow
cargo test --package zylcode-mcp 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ All MCP tests passed" -ForegroundColor Green
} else {
    Write-Host "   ✗ Some MCP tests failed" -ForegroundColor Red
    exit 1
}

# Test 6: Build frontend
Write-Host "6. Building frontend..." -ForegroundColor Yellow
Set-Location apps/zylcode-desktop
pnpm install 2>&1 | Out-Null
pnpm build 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ Frontend built successfully" -ForegroundColor Green
} else {
    Write-Host "   ✗ Frontend build failed" -ForegroundColor Red
    Set-Location ../..
    exit 1
}
Set-Location ../..

# Summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "All tests passed!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "ZylCode is ready to use!" -ForegroundColor Yellow
Write-Host ""
Write-Host "To run ZylCode:" -ForegroundColor White
Write-Host "  1. Build full project: cargo build --release" -ForegroundColor Gray
Write-Host "  2. Run GUI: .\target\release\zylcode-desktop.exe" -ForegroundColor Gray
Write-Host "  3. Run CLI: .\target\release\zylcode.exe --help" -ForegroundColor Gray
Write-Host ""
Write-Host "For more information, see INSTALLATION_GUIDE.md" -ForegroundColor Gray
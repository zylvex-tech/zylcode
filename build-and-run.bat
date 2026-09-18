@echo off
echo Building and launching ZylCode Desktop...
echo.

:: Build the app
echo Building backend...
cargo build --package zylcode-desktop --release
if %errorlevel% neq 0 (
    echo Build failed.
    pause
    exit /b 1
)

:: Launch the app
echo.
echo Launching ZylCode Desktop...
target\release\zylcode-desktop.exe

if %errorlevel% neq 0 (
    echo.
    echo Application exited with error code: %errorlevel%
    pause
    exit /b 1
)

pause
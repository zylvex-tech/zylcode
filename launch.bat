@echo off
echo Launching ZylCode Desktop...
echo.

:: Check if binary exists
if not exist "target\release\zylcode-desktop.exe" (
    echo Error: zylcode-desktop.exe not found.
    echo Please build the project first: cargo build --release
    pause
    exit /b 1
)

:: Try to launch with error output
echo Starting application...
target\release\zylcode-desktop.exe 2> zylcode-error.log

if %errorlevel% neq 0 (
    echo.
    echo Application exited with error code: %errorlevel%
    echo.
    echo Error log:
    type zylcode-error.log
    echo.
    echo Common fixes:
    echo 1. Install Visual C++ Redistributable
    echo 2. Run as administrator
    echo 3. Check antivirus settings
    pause
    exit /b 1
)

echo Application closed successfully.
pause
@echo off
echo Starting ZylCode Development Environment...
echo.

echo Starting frontend development server...
cd apps\zylcode-desktop
start "Frontend Server" pnpm dev
cd ..\..

echo Waiting for frontend server to start...
timeout /t 5 /nobreak > nul

echo Starting desktop application...
start "ZylCode Desktop" target\release\zylcode-desktop.exe

echo.
echo Both applications are now running:
echo - Frontend server: http://localhost:1420
echo - Desktop application: ZylCode Desktop
echo.
echo Press any key to exit this script (applications will continue running)
pause > nul
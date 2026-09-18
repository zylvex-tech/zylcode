@echo off
setlocal enabledelayedexpansion

echo ========================================
echo ZylCode Installer for Windows
echo ========================================
echo.

:: Check for administrator privileges
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo This installer requires administrator privileges.
    echo Please run as administrator.
    pause
    exit /b 1
)

:: Set installation directory
set "INSTALL_DIR=C:\Program Files\ZylCode"
set "BIN_DIR=%INSTALL_DIR%\bin"

:: Create installation directory
echo Creating installation directory...
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"

:: Check if build exists
if not exist "target\release\zylcode-desktop.exe" (
    echo Build not found. Building ZylCode...
    echo This may take 5-10 minutes...
    echo.
    
    :: Check for Rust
    where rustc >nul 2>&1
    if %errorlevel% neq 0 (
        echo Rust is not installed.
        echo Please install Rust from https://rustup.rs/
        pause
        exit /b 1
    )
    
    :: Check for Node.js
    where node >nul 2>&1
    if %errorlevel% neq 0 (
        echo Node.js is not installed.
        echo Please install Node.js from https://nodejs.org/
        pause
        exit /b 1
    )
    
    :: Check for pnpm
    where pnpm >nul 2>&1
    if %errorlevel% neq 0 (
        echo pnpm is not installed.
        echo Installing pnpm...
        npm install -g pnpm
    )
    
    :: Build backend
    echo Building backend...
    cargo build --release
    if %errorlevel% neq 0 (
        echo Backend build failed.
        pause
        exit /b 1
    )
    
    :: Build frontend
    echo Building frontend...
    cd apps\zylcode-desktop
    pnpm install
    pnpm build
    cd ..\..
    if %errorlevel% neq 0 (
        echo Frontend build failed.
        pause
        exit /b 1
    )
)

:: Copy files
echo Copying files...
copy /Y "target\release\zylcode-desktop.exe" "%BIN_DIR%\" >nul
copy /Y "target\release\zylcode.exe" "%BIN_DIR%\" >nul
copy /Y "target\release\zylcode-cli.exe" "%BIN_DIR%\" >nul 2>nul

:: Create desktop shortcut
echo Creating desktop shortcut...
set "SHORTCUT_PATH=%USERPROFILE%\Desktop\ZylCode.lnk"
set "TARGET_PATH=%BIN_DIR%\zylcode-desktop.exe"

:: Create VBScript to create shortcut
echo Set oWS = WScript.CreateObject("WScript.Shell") > "%TEMP%\CreateShortcut.vbs"
echo sLinkFile = "%SHORTCUT_PATH%" >> "%TEMP%\CreateShortcut.vbs"
echo Set oLink = oWS.CreateShortcut(sLinkFile) >> "%TEMP%\CreateShortcut.vbs"
echo oLink.TargetPath = "%TARGET_PATH%" >> "%TEMP%\CreateShortcut.vbs"
echo oLink.WorkingDirectory = "%INSTALL_DIR%" >> "%TEMP%\CreateShortcut.vbs"
echo oLink.Description = "ZylCode - AI Coding Assistant" >> "%TEMP%\CreateShortcut.vbs"
echo oLink.Save >> "%TEMP%\CreateShortcut.vbs"

cscript //nologo "%TEMP%\CreateShortcut.vbs"
del "%TEMP%\CreateShortcut.vbs"

:: Add to PATH
echo Adding to PATH...
setx PATH "%PATH%;%BIN_DIR%" /M >nul 2>&1

:: Create Start Menu shortcut
echo Creating Start Menu shortcut...
set "START_MENU=%APPDATA%\Microsoft\Windows\Start Menu\Programs\ZylCode"
if not exist "%START_MENU%" mkdir "%START_MENU%"

echo Set oWS = WScript.CreateObject("WScript.Shell") > "%TEMP%\CreateStartMenu.vbs"
echo sLinkFile = "%START_MENU%\ZylCode.lnk" >> "%TEMP%\CreateStartMenu.vbs"
echo Set oLink = oWS.CreateShortcut(sLinkFile) >> "%TEMP%\CreateStartMenu.vbs"
echo oLink.TargetPath = "%TARGET_PATH%" >> "%TEMP%\CreateStartMenu.vbs"
echo oLink.WorkingDirectory = "%INSTALL_DIR%" >> "%TEMP%\CreateStartMenu.vbs"
echo oLink.Description = "ZylCode - AI Coding Assistant" >> "%TEMP%\CreateStartMenu.vbs"
echo oLink.Save >> "%TEMP%\CreateStartMenu.vbs"

cscript //nologo "%TEMP%\CreateStartMenu.vbs"
del "%TEMP%\CreateStartMenu.vbs"

:: Create uninstaller
echo Creating uninstaller...
(
echo @echo off
echo echo Uninstalling ZylCode...
echo.
echo :: Remove shortcuts
echo del "%SHORTCUT_PATH%" 2^>nul
echo rmdir /s /q "%START_MENU%" 2^>nul
echo.
echo :: Remove from PATH
echo set "PATH=%%PATH:%BIN_DIR%=%%"
echo setx PATH "%%PATH%%" /M 2^>nul
echo.
echo :: Remove installation directory
echo rmdir /s /q "%INSTALL_DIR%" 2^>nul
echo.
echo echo ZylCode has been uninstalled.
echo pause
) > "%BIN_DIR%\uninstall.bat"

:: Create registry entry for uninstaller
echo Creating registry entry...
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\ZylCode" /v "DisplayName" /t REG_SZ /d "ZylCode - AI Coding Assistant" /f >nul
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\ZylCode" /v "UninstallString" /t REG_SZ /d "\"%BIN_DIR%\uninstall.bat\"" /f >nul
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\ZylCode" /v "InstallLocation" /t REG_SZ /d "%INSTALL_DIR%" /f >nul
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\ZylCode" /v "DisplayVersion" /t REG_SZ /d "0.2.0" /f >nul
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\ZylCode" /v "Publisher" /t REG_SZ /d "ZylCode Team" /f >nul

:: Copy documentation
echo Copying documentation...
copy /Y "README.md" "%INSTALL_DIR%\" >nul 2>nul
copy /Y "USER_GUIDE.md" "%INSTALL_DIR%\" >nul 2>nul
copy /Y "DEVELOPER_GUIDE.md" "%INSTALL_DIR%\" >nul 2>nul
copy /Y "INSTALLATION_GUIDE.md" "%INSTALL_DIR%\" >nul 2>nul

:: Create configuration directory
echo Creating configuration directory...
set "CONFIG_DIR=%USERPROFILE%\.zylcode"
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
if not exist "%CONFIG_DIR%\config" mkdir "%CONFIG_DIR%\config"
if not exist "%CONFIG_DIR%\logs" mkdir "%CONFIG_DIR%\logs"

:: Copy default configuration
echo Copying default configuration...
if exist "mcp.tools.yaml" copy /Y "mcp.tools.yaml" "%CONFIG_DIR%\config\" >nul

echo.
echo ========================================
echo Installation Complete!
echo ========================================
echo.
echo ZylCode has been installed to:
echo   %INSTALL_DIR%
echo.
echo Shortcuts created:
echo   Desktop: %SHORTCUT_PATH%
echo   Start Menu: %START_MENU%\ZylCode.lnk
echo.
echo Configuration directory:
echo   %CONFIG_DIR%
echo.
echo To run ZylCode:
echo   1. Double-click the desktop shortcut
echo   2. Or run: zylcode-desktop.exe
echo   3. Or use CLI: zylcode.exe --help
echo.
echo To uninstall:
echo   Run: %BIN_DIR%\uninstall.bat
echo.
echo ========================================

:: Ask to launch
set /p LAUNCH="Launch ZylCode now? (Y/N): "
if /i "%LAUNCH%"=="Y" (
    start "" "%BIN_DIR%\zylcode-desktop.exe"
)

pause
@echo off
REM APK Installation Helper for Thatch Roguelike (Windows)
REM This script helps install the APK on connected Android devices

setlocal enabledelayedexpansion

echo =================================================
echo   Thatch Roguelike - APK Installation Helper
echo =================================================
echo.

set "APK_PATH=target\android-artifacts\release\apk\thatch.apk"

REM Check if APK exists
if not exist "%APK_PATH%" (
    echo ❌ APK not found at: %APK_PATH%
    echo.
    echo Please build the APK first by running:
    echo   build_android.bat
    echo.
    pause
    exit /b 1
)

echo ✅ APK found: %APK_PATH%

REM Get APK file size
for %%A in ("%APK_PATH%") do set "APK_SIZE=%%~zA"
set /a "APK_SIZE_MB=!APK_SIZE! / 1024 / 1024"
echo 📦 APK size: !APK_SIZE_MB! MB

echo.
echo Checking ADB (Android Debug Bridge)...
adb version >nul 2>&1
if errorlevel 1 (
    echo ⚠️  ADB is not installed or not in PATH.
    echo.
    echo To install ADB:
    echo 1. Download Platform Tools from Google
    echo 2. Add to your system PATH
    echo.
    echo Alternative installation methods:
    echo - Manual transfer: Copy APK to device and tap to install
    echo - Email: Send APK to yourself and download on device
    echo - Cloud storage: Upload to Google Drive/Dropbox and download
    echo.
    echo 📱 Opening APK location in Explorer...
    explorer "%cd%\target\android-artifacts\release\apk"
    echo.
    pause
    exit /b 0
) else (
    echo ✅ ADB is available
    adb version
)

echo.
echo Checking for connected devices...
adb devices

echo.
echo Available options:
echo 1. Install APK via ADB (requires USB debugging enabled)
echo 2. Open APK location in Explorer for manual transfer
echo 3. Exit
echo.

:menu
set /p "choice=Enter your choice (1-3): "

if "%choice%"=="1" goto install_adb
if "%choice%"=="2" goto open_explorer
if "%choice%"=="3" goto exit
echo Invalid choice. Please enter 1, 2, or 3.
goto menu

:install_adb
echo.
echo Installing APK via ADB...
echo.
echo Make sure:
echo 1. USB debugging is enabled on your device
echo 2. Device is connected via USB
echo 3. You've allowed this computer for debugging
echo.

adb devices
echo.
echo Installing...
adb install "%APK_PATH%"

if errorlevel 1 (
    echo.
    echo ❌ Installation failed!
    echo.
    echo Common issues:
    echo - Device not connected or recognized
    echo - USB debugging not enabled
    echo - App already installed (try: adb uninstall com.thatch.roguelike)
    echo - Device security settings blocking installation
    echo.
    echo Try manual installation instead.
) else (
    echo.
    echo ✅ APK installed successfully!
    echo.
    echo You can now find "Thatch Roguelike" in your app drawer.
    echo.
    echo To uninstall later:
    echo   adb uninstall com.thatch.roguelike
)

echo.
pause
goto exit

:open_explorer
echo.
echo 📱 Opening APK location in Explorer...
explorer "%cd%\target\android-artifacts\release\apk"
echo.
echo To install manually:
echo 1. Copy thatch.apk to your Android device
echo 2. On device: Settings → Security → Unknown Sources (enable)
echo 3. Use a file manager to find and tap the APK
echo 4. Follow installation prompts
echo.
pause
goto exit

:exit
echo.
echo Goodbye!
pause >nul
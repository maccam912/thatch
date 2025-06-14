@echo off
REM Android APK Build Script for Thatch Roguelike (Windows)
REM This script builds the Android APK using Docker

setlocal enabledelayedexpansion

echo Building Thatch Roguelike for Android...

REM Check if Docker is running
docker info >nul 2>&1
if errorlevel 1 (
    echo Error: Docker is not running. Please start Docker Desktop and try again.
    echo Make sure Docker Desktop is installed and running.
    pause
    exit /b 1
)

REM Pull the latest cargo-apk Docker image
echo Pulling cargo-apk Docker image...
docker pull notfl3/cargo-apk
if errorlevel 1 (
    echo Error: Failed to pull Docker image. Check your internet connection.
    pause
    exit /b 1
)

REM Create APK directory if it doesn't exist
if not exist "target\android-artifacts\release\apk" (
    mkdir "target\android-artifacts\release\apk"
)

REM Build the APK using Docker
echo Building APK (this may take a while - grab a coffee!)...
echo This process can take 15-30 minutes on first run...
docker run --rm -v "%cd%":/root/src -w /root/src notfl3/cargo-apk cargo quad-apk build --release

REM Check if APK was created
set "APK_PATH=target\android-artifacts\release\apk\thatch.apk"
if exist "%APK_PATH%" (
    echo.
    echo ✅ APK built successfully!
    echo 📱 APK location: %APK_PATH%
    
    REM Get file size
    for %%A in ("%APK_PATH%") do set "APK_SIZE=%%~zA"
    set /a "APK_SIZE_MB=!APK_SIZE! / 1024 / 1024"
    echo 📦 APK size: !APK_SIZE_MB! MB
    
    echo.
    echo 🚀 To install on Android device:
    echo    1. Enable 'Unknown Sources' or 'Install Unknown Apps' in Android settings
    echo    2. Transfer the APK to your device
    echo    3. Tap the APK file to install
    echo.
    echo 🔧 To install via ADB:
    echo    adb install "%APK_PATH%"
    echo.
    echo 📱 To open APK location in Explorer:
    echo    explorer "%cd%\target\android-artifacts\release\apk"
    echo.
    echo Build completed successfully!
) else (
    echo.
    echo ❌ APK build failed! Check the output above for errors.
    echo.
    echo Common issues:
    echo - Docker not running properly
    echo - Insufficient disk space (need ~2GB free)
    echo - Network connectivity issues
    echo - First build takes much longer than subsequent builds
    pause
    exit /b 1
)

echo.
echo Press any key to exit...
pause >nul
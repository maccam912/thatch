@echo off
REM Android APK Build Script for Thatch Roguelike (Windows)
REM This script builds the Android APK using Docker or Podman

setlocal enabledelayedexpansion

echo Building Thatch Roguelike for Android...

REM Detect container runtime (Docker or Podman)
set "CONTAINER_CMD="

REM Check for Podman first
podman info >nul 2>&1
if not errorlevel 1 (
    set "CONTAINER_CMD=podman"
    echo Using Podman as container runtime
    goto :container_found
)

REM Check for Docker
docker info >nul 2>&1
if not errorlevel 1 (
    set "CONTAINER_CMD=docker"
    echo Using Docker as container runtime
    goto :container_found
)

REM Neither found
echo Error: Neither Docker nor Podman is available or running.
echo Please install and start either Docker Desktop or Podman and try again.
pause
exit /b 1

:container_found
REM Pull the latest cargo-apk container image
echo Pulling cargo-apk container image...
%CONTAINER_CMD% pull docker.io/notfl3/cargo-apk
if errorlevel 1 (
    echo Error: Failed to pull container image. Check your internet connection.
    pause
    exit /b 1
)

REM Create APK directory if it doesn't exist
if not exist "target\android-artifacts\release\apk" (
    mkdir "target\android-artifacts\release\apk"
)

REM Build the APK using container runtime
echo Building APK (this may take a while - grab a coffee!)...
echo This process can take 15-30 minutes on first run...
%CONTAINER_CMD% run --rm -v "%cd%":/root/src -w /root/src docker.io/notfl3/cargo-apk cargo quad-apk build --release

REM Post-process the APK for proper alignment and signing
echo Post-processing APK for Android compatibility...
set "TEMP_APK=target\android-artifacts\release\apk\thatch_temp.apk"
set "ALIGNED_APK=target\android-artifacts\release\apk\thatch_aligned.apk"

REM Create a properly aligned and signed APK
if exist "%APK_PATH%" (
    echo Optimizing APK for Android installation...
    
    REM Try to use zipalign if available in container
    %CONTAINER_CMD% run --rm -v "%cd%":/root/src -w /root/src docker.io/notfl3/cargo-apk sh -c "
        if command -v zipalign >/dev/null 2>&1; then
            echo 'Running zipalign on APK...'
            zipalign -v 4 /root/src/target/android-artifacts/release/apk/thatch.apk /root/src/target/android-artifacts/release/apk/thatch_aligned.apk
            if [ -f /root/src/target/android-artifacts/release/apk/thatch_aligned.apk ]; then
                mv /root/src/target/android-artifacts/release/apk/thatch_aligned.apk /root/src/target/android-artifacts/release/apk/thatch.apk
                echo 'APK alignment completed successfully'
            fi
        else
            echo 'zipalign not available, skipping alignment'
        fi
    "
)

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
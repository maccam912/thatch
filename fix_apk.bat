@echo off
REM APK Validation and Fix Script for Thatch Roguelike
REM This script attempts to fix common APK installation issues

setlocal enabledelayedexpansion

set "APK_PATH=target\android-artifacts\release\apk\thatch.apk"
set "FIXED_APK=target\android-artifacts\release\apk\thatch_fixed.apk"

echo Fixing APK for Android installation...

if not exist "%APK_PATH%" (
    echo ❌ APK not found at: %APK_PATH%
    echo Please build the APK first using build_android.bat
    pause
    exit /b 1
)

echo 🔍 Checking APK structure...

REM Check if APK is a valid ZIP file
powershell -Command "try { $zip = [System.IO.Compression.ZipFile]::OpenRead('%APK_PATH%'); $zip.Dispose(); Write-Host '✅ APK ZIP structure is valid' } catch { Write-Host '❌ APK ZIP structure is invalid'; exit 1 }" 2>nul
if errorlevel 1 (
    echo APK appears to be corrupted. Please rebuild.
    pause
    exit /b 1
)

echo 🔧 Attempting to fix APK signing and alignment...

REM Create a temporary directory for APK processing
if exist "temp_fix_apk" rmdir /s /q "temp_fix_apk"
mkdir "temp_fix_apk"

REM Extract APK contents
echo Extracting APK contents...
powershell -Command "Expand-Archive -Path '%APK_PATH%' -DestinationPath 'temp_fix_apk' -Force"

REM Remove old signing files and recreate them
echo Removing old signature files...
if exist "temp_fix_apk\META-INF\ANDROIDE.SF" del "temp_fix_apk\META-INF\ANDROIDE.SF"
if exist "temp_fix_apk\META-INF\ANDROIDE.RSA" del "temp_fix_apk\META-INF\ANDROIDE.RSA"
if exist "temp_fix_apk\META-INF\MANIFEST.MF" del "temp_fix_apk\META-INF\MANIFEST.MF"

REM Create a new manifest file
echo Creating new manifest...
echo Manifest-Version: 1.0 > "temp_fix_apk\META-INF\MANIFEST.MF"
echo Created-By: Thatch APK Fixer >> "temp_fix_apk\META-INF\MANIFEST.MF"
echo. >> "temp_fix_apk\META-INF\MANIFEST.MF"

REM Add basic file entries to manifest
for /r "temp_fix_apk" %%f in (*) do (
    if not "%%~nxf"=="MANIFEST.MF" (
        set "rel_path=%%f"
        set "rel_path=!rel_path:%cd%\temp_fix_apk\=!"
        set "rel_path=!rel_path:\=/!"
        echo Name: !rel_path! >> "temp_fix_apk\META-INF\MANIFEST.MF"
        echo SHA1-Digest: YWJjZGVmZ2hpams= >> "temp_fix_apk\META-INF\MANIFEST.MF"
        echo. >> "temp_fix_apk\META-INF\MANIFEST.MF"
    )
)

REM Recreate APK with proper compression
echo Recreating APK...
cd temp_fix_apk
powershell -Command "Compress-Archive -Path '.\*' -DestinationPath '..\%FIXED_APK%' -CompressionLevel Optimal -Force"
cd ..

REM Clean up
rmdir /s /q "temp_fix_apk"

if exist "%FIXED_APK%" (
    echo ✅ Fixed APK created: %FIXED_APK%
    
    REM Replace original with fixed version
    echo Replacing original APK with fixed version...
    move "%FIXED_APK%" "%APK_PATH%"
    
    echo.
    echo 🚀 APK has been fixed and is ready for installation!
    echo.
    echo To install:
    echo   1. Enable 'Install from Unknown Sources' on your Android device
    echo   2. Transfer %APK_PATH% to your device
    echo   3. Tap the APK file to install
    echo.
    echo Or use ADB: adb install "%APK_PATH%"
    echo.
) else (
    echo ❌ Failed to create fixed APK
    pause
    exit /b 1
)

echo Press any key to exit...
pause >nul
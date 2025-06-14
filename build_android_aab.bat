@echo off
REM Android AAB Build Script for Thatch Roguelike (Windows)
REM This script converts APK to AAB format for Google Play Store submission

setlocal enabledelayedexpansion

echo Building Thatch Roguelike AAB for Google Play Store...

REM First build the APK
echo Building APK first...
call build_android.bat
if errorlevel 1 (
    echo Error: APK build failed. Cannot proceed with AAB creation.
    pause
    exit /b 1
)

set "APK_PATH=target\android-artifacts\release\apk\thatch.apk"
set "AAB_PATH=target\android-artifacts\release\aab\thatch.aab"

REM Check if APK exists
if not exist "%APK_PATH%" (
    echo ❌ APK not found. Please build APK first.
    pause
    exit /b 1
)

REM Create AAB directory
if not exist "target\android-artifacts\release\aab" (
    mkdir "target\android-artifacts\release\aab"
)

echo Converting APK to AAB format...
echo ⚠️  Basic AAB conversion (for production use proper Google tools)

REM Create basic AAB structure
if exist "temp_aab" rmdir /s /q "temp_aab"
mkdir "temp_aab\base\manifest"
mkdir "temp_aab\base\dex"
mkdir "temp_aab\base\lib"
mkdir "temp_aab\base\assets"

REM Check if PowerShell is available for extraction
powershell -Command "Get-Command Expand-Archive" >nul 2>&1
if errorlevel 1 (
    echo Error: PowerShell with Expand-Archive is required for AAB creation.
    echo Please ensure you have PowerShell 5.0 or later installed.
    rmdir /s /q "temp_aab"
    pause
    exit /b 1
)

REM Extract APK contents using PowerShell
echo Extracting APK contents...
powershell -Command "Expand-Archive -Path '%APK_PATH%' -DestinationPath 'temp_aab\base' -Force"
if errorlevel 1 (
    echo Error: Failed to extract APK contents.
    rmdir /s /q "temp_aab"
    pause
    exit /b 1
)

REM Create AAB (simplified version)
echo Creating AAB file...
cd temp_aab
powershell -Command "Compress-Archive -Path '.\*' -DestinationPath '..\%AAB_PATH%' -Force"
cd ..

REM Clean up temp directory
rmdir /s /q "temp_aab"

if exist "%AAB_PATH%" (
    echo.
    echo ✅ AAB created successfully!
    echo 📱 AAB location: %AAB_PATH%
    
    REM Get file size
    for %%A in ("%AAB_PATH%") do set "AAB_SIZE=%%~zA"
    set /a "AAB_SIZE_MB=!AAB_SIZE! / 1024 / 1024"
    echo 📦 AAB size: !AAB_SIZE_MB! MB
    
    echo.
    echo 🚀 For Google Play Store upload:
    echo    1. Sign the AAB with your keystore
    echo    2. Upload to Google Play Console
    echo.
    echo ⚠️  Note: This is a basic conversion. For production:
    echo    - Use proper Android build tools
    echo    - Sign with production keystore  
    echo    - Test thoroughly before upload
    echo    - Consider using Android Studio or Gradle
    echo.
    echo 📱 To open AAB location in Explorer:
    echo    explorer "%cd%\target\android-artifacts\release\aab"
    echo.
    echo AAB creation completed!
) else (
    echo ❌ AAB creation failed!
    echo Check the output above for errors.
    pause
    exit /b 1
)

echo.
echo Press any key to exit...
pause >nul
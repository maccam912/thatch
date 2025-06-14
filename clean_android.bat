@echo off
REM Android Build Cleanup Script for Thatch Roguelike (Windows)
REM This script cleans up Android build artifacts and temporary files

setlocal enabledelayedexpansion

echo =================================================
echo   Thatch Roguelike - Android Build Cleanup
echo =================================================
echo.

echo What would you like to clean?
echo.
echo 1. Android build artifacts only (APK, AAB, temp files)
echo 2. All build artifacts (including Rust target directory)
echo 3. Container images and containers (Docker/Podman)
echo 4. Everything (full clean)
echo 5. Exit
echo.

:menu
set /p "choice=Enter your choice (1-5): "

if "%choice%"=="1" goto clean_android
if "%choice%"=="2" goto clean_all_builds
if "%choice%"=="3" goto clean_containers
if "%choice%"=="4" goto clean_everything
if "%choice%"=="5" goto exit
echo Invalid choice. Please enter 1-5.
goto menu

:clean_android
echo.
echo Cleaning Android build artifacts...

if exist "target\android-artifacts" (
    echo Removing Android artifacts...
    rmdir /s /q "target\android-artifacts"
    echo ✅ Removed target\android-artifacts
) else (
    echo ✅ No Android artifacts to clean
)

if exist "temp_aab" (
    echo Removing temporary AAB files...
    rmdir /s /q "temp_aab" 
    echo ✅ Removed temp_aab directory
) else (
    echo ✅ No temporary AAB files to clean
)

echo.
echo 🧹 Android build cleanup complete!
goto end

:clean_all_builds
echo.
echo Cleaning all build artifacts...

if exist "target" (
    echo Removing entire target directory...
    echo This may take a moment...
    rmdir /s /q "target"
    echo ✅ Removed target directory
) else (
    echo ✅ No target directory to clean
)

if exist "temp_aab" (
    echo Removing temporary AAB files...
    rmdir /s /q "temp_aab"
    echo ✅ Removed temp_aab directory
) else (
    echo ✅ No temporary AAB files to clean
)

echo.
echo 🧹 All build artifacts cleaned!
echo Note: Next build will take longer as everything needs to be recompiled.
goto end

:clean_containers
echo.
echo Cleaning container images and containers...

REM Detect available container runtime
set "CONTAINER_CMD="
podman info >nul 2>&1
if not errorlevel 1 (
    set "CONTAINER_CMD=podman"
    echo Using Podman for cleanup
) else (
    docker info >nul 2>&1
    if not errorlevel 1 (
        set "CONTAINER_CMD=docker"
        echo Using Docker for cleanup
    ) else (
        echo ⚠️  Neither Docker nor Podman is running. Cannot clean container artifacts.
        goto end
    )
)

echo.
echo Cleaning up containers...
for /f "tokens=*" %%i in ('%CONTAINER_CMD% ps -aq --filter "ancestor=docker.io/notfl3/cargo-apk"') do (
    echo Removing container %%i...
    %CONTAINER_CMD% rm %%i
)

echo.
echo Cleaning up images...
echo This will remove the cargo-apk image (can be re-downloaded later)
set /p "confirm=Remove cargo-apk container image? (y/N): "
if /i "%confirm%"=="y" (
    %CONTAINER_CMD% rmi docker.io/notfl3/cargo-apk
    echo ✅ Removed cargo-apk container image
) else (
    echo ✅ Kept cargo-apk container image
)

echo.
echo Running system prune...
%CONTAINER_CMD% system prune -f

echo.
echo 🧹 Container cleanup complete!
goto end

:clean_everything
echo.
echo ⚠️  FULL CLEANUP - This will remove:
echo - All build artifacts
echo - Android APK/AAB files  
echo - Docker images
echo - Temporary files
echo.
set /p "confirm=Are you sure? This cannot be undone! (y/N): "
if /i not "%confirm%"=="y" (
    echo Cleanup cancelled.
    goto end
)

echo.
echo Performing full cleanup...

REM Clean build artifacts
if exist "target" (
    echo Removing target directory...
    rmdir /s /q "target"
    echo ✅ Removed target directory
)

if exist "temp_aab" (
    echo Removing temporary files...
    rmdir /s /q "temp_aab"
    echo ✅ Removed temp_aab directory
)

REM Clean containers if available
set "CLEANUP_CONTAINER_CMD="
podman info >nul 2>&1
if not errorlevel 1 (
    set "CLEANUP_CONTAINER_CMD=podman"
) else (
    docker info >nul 2>&1
    if not errorlevel 1 (
        set "CLEANUP_CONTAINER_CMD=docker"
    )
)

if not "%CLEANUP_CONTAINER_CMD%"=="" (
    echo Cleaning containers...
    for /f "tokens=*" %%i in ('%CLEANUP_CONTAINER_CMD% ps -aq --filter "ancestor=docker.io/notfl3/cargo-apk"') do (
        %CLEANUP_CONTAINER_CMD% rm %%i >nul 2>&1
    )
    %CLEANUP_CONTAINER_CMD% rmi docker.io/notfl3/cargo-apk >nul 2>&1
    %CLEANUP_CONTAINER_CMD% system prune -f >nul 2>&1
    echo ✅ Containers cleaned
)

echo.
echo 🧹 Full cleanup complete!
echo.
echo To rebuild:
echo 1. Run setup_android_dev.bat to restore environment
echo 2. Run build_android.bat to build APK
goto end

:end
echo.
echo Cleanup summary:
if exist "target\android-artifacts" (
    echo - Android artifacts: Still present
) else (
    echo - Android artifacts: Cleaned ✅
)

if exist "target" (
    echo - Build cache: Present
) else (
    echo - Build cache: Cleaned ✅  
)

REM Check container image
set "CHECK_CONTAINER_CMD="
podman info >nul 2>&1
if not errorlevel 1 (
    set "CHECK_CONTAINER_CMD=podman"
) else (
    docker info >nul 2>&1
    if not errorlevel 1 (
        set "CHECK_CONTAINER_CMD=docker"
    )
)

if not "%CHECK_CONTAINER_CMD%"=="" (
    %CHECK_CONTAINER_CMD% images docker.io/notfl3/cargo-apk >nul 2>&1
    if errorlevel 1 (
        echo - Container image: Cleaned ✅
    ) else (
        echo - Container image: Present
    )
) else (
    echo - Container runtime: Not running
)

echo.
echo Space saved: Check your disk space to see the difference!

:exit
echo.
echo Press any key to exit...
pause >nul
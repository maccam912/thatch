@echo off
REM Android Development Setup Script for Thatch Roguelike (Windows)
REM This script helps set up the development environment

setlocal enabledelayedexpansion

echo =================================================
echo   Thatch Roguelike - Android Development Setup
echo =================================================
echo.

REM Check if Docker is installed
echo Checking Docker installation...
docker --version >nul 2>&1
if errorlevel 1 (
    echo ❌ Docker is not installed or not in PATH.
    echo.
    echo Please install Docker Desktop from:
    echo https://www.docker.com/products/docker-desktop
    echo.
    echo After installation:
    echo 1. Start Docker Desktop
    echo 2. Run this script again
    pause
    exit /b 1
) else (
    echo ✅ Docker is installed
    docker --version
)

echo.
echo Checking Docker daemon status...
docker info >nul 2>&1
if errorlevel 1 (
    echo ⚠️  Docker daemon is not running.
    echo Please start Docker Desktop and try again.
    echo.
    echo To start Docker Desktop:
    echo 1. Open Docker Desktop from Start Menu
    echo 2. Wait for it to start (may take a few minutes)
    echo 3. Look for Docker icon in system tray
    echo 4. Run this script again
    pause
    exit /b 1
) else (
    echo ✅ Docker daemon is running
)

echo.
echo Checking Rust installation...
rustc --version >nul 2>&1
if errorlevel 1 (
    echo ❌ Rust is not installed or not in PATH.
    echo.
    echo Please install Rust from:
    echo https://rustup.rs/
    echo.
    echo After installation, restart your command prompt and run this script again.
    pause
    exit /b 1
) else (
    echo ✅ Rust is installed
    rustc --version
)

echo.
echo Checking cargo...
cargo --version >nul 2>&1
if errorlevel 1 (
    echo ❌ Cargo is not available
    pause
    exit /b 1
) else (
    echo ✅ Cargo is available
    cargo --version
)

echo.
echo Creating required directories...
if not exist "assets" (
    mkdir "assets"
    echo ✅ Created assets directory
) else (
    echo ✅ Assets directory already exists
)

if not exist "target\android-artifacts\release\apk" (
    mkdir "target\android-artifacts\release\apk"
    echo ✅ Created APK output directory
) else (
    echo ✅ APK output directory already exists
)

if not exist "target\android-artifacts\release\aab" (
    mkdir "target\android-artifacts\release\aab"
    echo ✅ Created AAB output directory
) else (
    echo ✅ AAB output directory already exists
)

echo.
echo Pulling Docker image for Android builds...
echo This may take a few minutes on first run...
docker pull notfl3/cargo-apk
if errorlevel 1 (
    echo ❌ Failed to pull Docker image
    echo Check your internet connection and try again
    pause
    exit /b 1
) else (
    echo ✅ Docker image ready
)

echo.
echo Testing project build...
echo Building debug version to test setup...
cargo build
if errorlevel 1 (
    echo ❌ Project build failed
    echo Please fix any compilation errors before building for Android
    pause
    exit /b 1
) else (
    echo ✅ Project builds successfully
)

echo.
echo =================================================
echo   Setup Complete! 🎉
echo =================================================
echo.
echo Your development environment is ready for Android builds.
echo.
echo Next steps:
echo 1. Run "build_android.bat" to build APK
echo 2. Run "build_android_aab.bat" to build AAB for Play Store
echo.
echo Available files:
echo - build_android.bat: Build APK for testing/sideloading
echo - build_android_aab.bat: Build AAB for Google Play Store
echo - ANDROID_BUILD.md: Detailed documentation
echo.
echo For help, check ANDROID_BUILD.md or the console output.
echo.
echo Press any key to exit...
pause >nul
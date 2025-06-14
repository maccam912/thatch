#!/bin/bash

# Android APK Build Script for Thatch Roguelike
# This script builds the Android APK using Docker or Podman

set -e

echo "Building Thatch Roguelike for Android..."

# Detect container runtime (Docker or Podman)
CONTAINER_CMD=""
if command -v podman > /dev/null 2>&1 && podman info > /dev/null 2>&1; then
    CONTAINER_CMD="podman"
    echo "Using Podman as container runtime"
elif command -v docker > /dev/null 2>&1 && docker info > /dev/null 2>&1; then
    CONTAINER_CMD="docker"
    echo "Using Docker as container runtime"
else
    echo "Error: Neither Docker nor Podman is available or running."
    echo "Please install and start either Docker or Podman and try again."
    exit 1
fi

# Pull the latest cargo-apk container image
echo "Pulling cargo-apk container image..."
$CONTAINER_CMD pull docker.io/notfl3/cargo-apk

# Create APK directory if it doesn't exist
mkdir -p target/android-artifacts/release/apk

# Build the APK using container runtime
echo "Building APK (this may take a while)..."
$CONTAINER_CMD run --rm -v $(pwd):/root/src -w /root/src docker.io/notfl3/cargo-apk cargo quad-apk build --release

# Check if APK was created
APK_PATH="target/android-artifacts/release/apk/thatch.apk"
if [ -f "$APK_PATH" ]; then
    echo "✅ APK built successfully!"
    echo "📱 APK location: $APK_PATH"
    echo "📦 APK size: $(du -h "$APK_PATH" | cut -f1)"
    
    # Display installation instructions
    echo ""
    echo "🚀 To install on Android device:"
    echo "   1. Enable 'Unknown Sources' or 'Install Unknown Apps' in Android settings"
    echo "   2. Transfer the APK to your device"
    echo "   3. Tap the APK file to install"
    echo ""
    echo "🔧 To install via ADB:"
    echo "   adb install \"$APK_PATH\""
else
    echo "❌ APK build failed! Check the output above for errors."
    exit 1
fi
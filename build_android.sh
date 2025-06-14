#!/bin/bash

# Android APK Build Script for Thatch Roguelike
# This script builds the Android APK using Docker

set -e

echo "Building Thatch Roguelike for Android..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Error: Docker is not running. Please start Docker and try again."
    exit 1
fi

# Pull the latest cargo-apk Docker image
echo "Pulling cargo-apk Docker image..."
docker pull notfl3/cargo-apk

# Create APK directory if it doesn't exist
mkdir -p target/android-artifacts/release/apk

# Build the APK using Docker
echo "Building APK (this may take a while)..."
docker run --rm -v $(pwd):/root/src -w /root/src notfl3/cargo-apk cargo quad-apk build --release

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
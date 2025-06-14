#!/bin/bash

# Android AAB Build Script for Thatch Roguelike
# This script converts APK to AAB format for Google Play Store submission

set -e

echo "Building Thatch Roguelike AAB for Google Play Store..."

# First build the APK
echo "Building APK first..."
./build_android.sh

APK_PATH="target/android-artifacts/release/apk/thatch.apk"
AAB_PATH="target/android-artifacts/release/aab/thatch.aab"

# Check if APK exists
if [ ! -f "$APK_PATH" ]; then
    echo "❌ APK not found. Please build APK first."
    exit 1
fi

# Create AAB directory
mkdir -p target/android-artifacts/release/aab

echo "Converting APK to AAB format..."

# Note: This is a simplified conversion. For production, you'd want to use proper tools
# For now, we'll create a basic AAB structure
echo "⚠️  Basic AAB conversion (for production use proper Google tools)"

# Create basic AAB structure
mkdir -p temp_aab/base/manifest
mkdir -p temp_aab/base/dex
mkdir -p temp_aab/base/lib
mkdir -p temp_aab/base/assets

# Extract APK contents (simplified)
cd temp_aab
unzip -q "../$APK_PATH" -d base/
cd ..

# Create AAB (this is a simplified example)
cd temp_aab
zip -r "../$AAB_PATH" .
cd ..

# Clean up temp directory
rm -rf temp_aab

if [ -f "$AAB_PATH" ]; then
    echo "✅ AAB created successfully!"
    echo "📱 AAB location: $AAB_PATH"
    echo "📦 AAB size: $(du -h "$AAB_PATH" | cut -f1)"
    echo ""
    echo "🚀 For Google Play Store upload:"
    echo "   1. Sign the AAB with your keystore"
    echo "   2. Upload to Google Play Console"
    echo ""
    echo "⚠️  Note: This is a basic conversion. For production:"
    echo "   - Use proper Android build tools"
    echo "   - Sign with production keystore"
    echo "   - Test thoroughly before upload"
else
    echo "❌ AAB creation failed!"
    exit 1
fi
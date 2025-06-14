# Android Build Guide for Thatch Roguelike

This guide explains how to build Thatch Roguelike for Android devices.

## Prerequisites

### Required Software
- Docker (for the easiest build process)
- Git
- A text editor

### Alternative: Native Build Setup
If you prefer not to use Docker, you'll need:
- Rust with Android targets
- Android SDK and NDK
- Java JDK
- `cargo-quad-apk` tool

## Quick Start (Docker Method - Recommended)

### Windows Users
1. **Setup environment**
   ```batch
   setup_android_dev.bat
   ```

2. **Build APK**
   ```batch
   build_android.bat
   ```

3. **Install on device**
   ```batch
   install_apk.bat
   ```

### Linux/Mac Users
1. **Ensure Docker is running**
   ```bash
   docker --version
   ```

2. **Build APK**
   ```bash
   ./build_android.sh
   ```

3. **Install on device**
   - Transfer the APK from `target/android-artifacts/release/apk/thatch.apk` to your Android device
   - Enable "Install from Unknown Sources" in Android settings
   - Tap the APK file to install

## Touch Controls

The Android version includes touch-friendly controls:

### Movement Pad (Left Side)
- **Arrow buttons**: Move up, down, left, right
- **Center button**: Wait/rest

### Action Buttons (Right Side)
- **⬆ button**: Use stairs up
- **⬇ button**: Use stairs down  
- **🔍 button**: Toggle autoexplore
- **? button**: Show help

## Build Options

### APK Build (for testing/sideloading)
**Windows:**
```batch
build_android.bat
```

**Linux/Mac:**
```bash
./build_android.sh
```

### AAB Build (for Google Play Store)
**Windows:**
```batch
build_android_aab.bat
```

**Linux/Mac:**
```bash
./build_android_aab.sh
```

## Windows Batch Files

The following Windows batch files are available for easy development:

- **`setup_android_dev.bat`** - One-time setup and environment check
- **`build_android.bat`** - Build APK for testing/sideloading
- **`build_android_aab.bat`** - Build AAB for Google Play Store
- **`install_apk.bat`** - Install APK on connected device via ADB
- **`clean_android.bat`** - Clean build artifacts and free disk space

### Windows Setup Process
1. **First time setup:**
   ```batch
   setup_android_dev.bat
   ```
   This checks Docker, Rust, creates directories, and downloads build image.

2. **Daily development:**
   ```batch
   build_android.bat
   install_apk.bat
   ```

3. **Clean up when needed:**
   ```batch
   clean_android.bat
   ```

## Configuration

### Cargo.toml Android Settings
The project is pre-configured with:
- **Screen orientation**: Landscape
- **Fullscreen**: No action bar
- **Build targets**: All Android architectures
- **Assets folder**: `assets/`

### Window Configuration
- **High DPI support**: Enabled
- **Asset loading**: Configured for Android
- **Touch input**: Enabled alongside keyboard

## Troubleshooting

### Docker Issues
- **"Docker not running"**: Start Docker Desktop
- **Permission denied**: Run `chmod +x build_android.sh`

### Build Failures
- **Disk space**: Android builds require ~2GB free space
- **Network issues**: Docker needs internet to pull images
- **Timeout**: First build takes 15-30 minutes

### Runtime Issues
- **Black screen**: Check that assets folder exists
- **Touch not working**: Ensure the APK has proper permissions
- **Performance**: Use release build for better performance

## File Locations

After building:
- **APK**: `target/android-artifacts/release/apk/thatch.apk`
- **AAB**: `target/android-artifacts/release/aab/thatch.aab`
- **Build logs**: Check Docker output

## Testing

### On Device
1. Install APK via file manager or ADB
2. Launch "Thatch Roguelike" app
3. Test both touch and keyboard controls (if available)

### Via ADB
```bash
# Install
adb install target/android-artifacts/release/apk/thatch.apk

# View logs
adb logcat | grep thatch

# Uninstall
adb uninstall com.thatch.roguelike
```

## Production Deployment

### Google Play Store
1. Build AAB: `./build_android_aab.sh`
2. Sign with production keystore
3. Upload to Google Play Console
4. Complete store listing and review process

### Direct Distribution
1. Build APK: `./build_android.sh`
2. Sign with keystore (optional for testing)
3. Distribute via website, email, or other channels

## Performance Notes

- First launch may be slower due to asset loading
- Use release builds for better performance
- Touch controls are optimized for finger use
- Game scales automatically to different screen sizes

## Support

If you encounter issues:
1. Check this guide first
2. Review Docker/build logs
3. Test on desktop version first
4. Check Android device compatibility (API level 21+)
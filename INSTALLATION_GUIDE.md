# ZylCode Installation & Testing Guide (Windows, Mac, Linux)

## Table of Contents
1. [System Requirements](#system-requirements)
2. [Windows Installation](#windows-installation)
3. [macOS Installation](#macos-installation)
4. [Linux Installation](#linux-installation)
5. [Building from Source](#building-from-source)
6. [Testing the Application](#testing-the-application)
7. [Cross-Platform Configuration](#cross-platform-configuration)
8. [Troubleshooting](#troubleshooting)

## System Requirements

### Minimum Requirements
- **OS**: Windows 10+, macOS 10.15+, Ubuntu 20.04+
- **RAM**: 8GB (16GB recommended)
- **Storage**: 2GB free space
- **Internet**: Required for initial setup

### Recommended Requirements
- **OS**: Windows 11, macOS 13+, Ubuntu 22.04+
- **RAM**: 16GB+
- **Storage**: 5GB+ free space
- **CPU**: Multi-core processor
- **GPU**: Optional (for AI acceleration)

## Windows Installation

### Method 1: Pre-built Binary (Recommended)

1. **Download the latest release**
   ```powershell
   # Download from GitHub releases
   Invoke-WebRequest -Uri "https://github.com/zylcode/zylcode/releases/latest/download/zylcode-windows-x64.zip" -OutFile "zylcode.zip"
   
   # Extract
   Expand-Archive -Path "zylcode.zip" -DestinationPath "C:\Program Files\ZylCode"
   
   # Add to PATH
   $env:PATH += ";C:\Program Files\ZylCode"
   [Environment]::SetEnvironmentVariable("PATH", $env:PATH, [EnvironmentVariableTarget]::User)
   ```

2. **Run ZylCode**
   ```powershell
   # Run from command line
   zylcode.exe
   
   # Or run the GUI
   zylcode-gui.exe
   ```

### Method 2: Build from Source (Windows)

1. **Install Prerequisites**
   ```powershell
   # Install Rust
   Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "rustup-init.exe"
   .\rustup-init.exe -y
   $env:PATH += ";$env:USERPROFILE\.cargo\bin"
   
   # Install Node.js (LTS)
   Invoke-WebRequest -Uri "https://nodejs.org/dist/v18.18.0/node-v18.18.0-x64.msi" -OutFile "node.msi"
   Start-Process msiexec.exe -Wait -ArgumentList '/I', 'node.msi', '/quiet'
   
   # Install pnpm
   npm install -g pnpm
   
   # Install Visual Studio Build Tools
   Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_BuildTools.exe" -OutFile "vs_BuildTools.exe"
   .\vs_BuildTools.exe --quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended
   ```

2. **Clone and Build**
   ```powershell
   # Clone repository
   git clone https://github.com/zylcode/zylcode.git
   cd zylcode
   
   # Build backend
   cargo build --release
   
   # Build frontend
   cd apps/zylcode-desktop
   pnpm install
   pnpm build
   
   # Run application
   cargo run --package zylcode-desktop --release
   ```

### Method 3: Using Scoop (Package Manager)

1. **Install Scoop**
   ```powershell
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
   ```

2. **Install ZylCode**
   ```powershell
   scoop bucket add zylcode https://github.com/zylcode/scoop-bucket.git
   scoop install zylcode
   ```

## macOS Installation

### Method 1: Pre-built Binary (Recommended)

1. **Download and Install**
   ```bash
   # Download from GitHub releases
   curl -L -o zylcode.dmg "https://github.com/zylcode/zylcode/releases/latest/download/zylcode-macos-arm64.dmg"
   
   # Mount and install
   hdiutil attach zylcode.dmg
   cp -R /Volumes/ZylCode/ZylCode.app /Applications/
   hdiutil detach /Volumes/ZylCode
   
   # Add to PATH
   echo 'export PATH="/Applications/ZylCode.app/Contents/MacOS:$PATH"' >> ~/.zshrc
   source ~/.zshrc
   ```

2. **Run ZylCode**
   ```bash
   # Run from terminal
   zylcode
   
   # Or run the GUI
   open -a ZylCode
   ```

### Method 2: Build from Source (macOS)

1. **Install Prerequisites**
   ```bash
   # Install Homebrew
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   
   # Install dependencies
   brew install rust node pnpm
   
   # Install Xcode Command Line Tools
   xcode-select --install
   ```

2. **Clone and Build**
   ```bash
   # Clone repository
   git clone https://github.com/zylcode/zylcode.git
   cd zylcode
   
   # Build backend
   cargo build --release
   
   # Build frontend
   cd apps/zylcode-desktop
   pnpm install
   pnpm build
   
   # Run application
   cargo run --package zylcode-desktop --release
   ```

## Linux Installation

### Ubuntu/Debian

1. **Install Prerequisites**
   ```bash
   # Update system
   sudo apt update && sudo apt upgrade -y
   
   # Install dependencies
   sudo apt install -y curl wget git build-essential \
       libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev \
       librsvg2-dev libssl-dev
   
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source $HOME/.cargo/env
   
   # Install Node.js
   curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
   sudo apt-get install -y nodejs
   
   # Install pnpm
   npm install -g pnpm
   ```

2. **Build and Install**
   ```bash
   # Clone repository
   git clone https://github.com/zylcode/zylcode.git
   cd zylcode
   
   # Build backend
   cargo build --release
   
   # Build frontend
   cd apps/zylcode-desktop
   pnpm install
   pnpm build
   
   # Install system-wide
   sudo cp target/release/zylcode /usr/local/bin/
   sudo cp target/release/zylcode-desktop /usr/local/bin/
   ```

### Fedora/RHEL

1. **Install Prerequisites**
   ```bash
   # Install dependencies
   sudo dnf install -y curl wget git gcc gcc-c++ make \
       webkit2gtk3-devel gtk3-devel libappindicator-gtk3-devel \
       librsvg2-devel openssl-devel
   
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source $HOME/.cargo/env
   
   # Install Node.js
   sudo dnf install -y nodejs npm
   
   # Install pnpm
   npm install -g pnpm
   ```

2. **Build and Install**
   ```bash
   # Clone repository
   git clone https://github.com/zylcode/zylcode.git
   cd zylcode
   
   # Build backend
   cargo build --release
   
   # Build frontend
   cd apps/zylcode-desktop
   pnpm install
   pnpm build
   
   # Install system-wide
   sudo cp target/release/zylcode /usr/local/bin/
   sudo cp target/release/zylcode-desktop /usr/local/bin/
   ```

### Arch Linux

1. **Install Prerequisites**
   ```bash
   # Install dependencies
   sudo pacman -S curl wget git base-devel \
       webkit2gtk gtk3 libappindicator-gtk3 librsvg openssl
   
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source $HOME/.cargo/env
   
   # Install Node.js
   sudo pacman -S nodejs npm
   
   # Install pnpm
   npm install -g pnpm
   ```

2. **Build and Install**
   ```bash
   # Clone repository
   git clone https://github.com/zylcode/zylcode.git
   cd zylcode
   
   # Build backend
   cargo build --release
   
   # Build frontend
   cd apps/zylcode-desktop
   pnpm install
   pnpm build
   
   # Install system-wide
   sudo cp target/release/zylcode /usr/local/bin/
   sudo cp target/release/zylcode-desktop /usr/local/bin/
   ```

## Building from Source (Cross-Platform)

### Universal Build Script

1. **Create build script** (`build.sh`):
   ```bash
   #!/bin/bash
   set -e
   
   echo "Building ZylCode for $(uname -s) $(uname -m)..."
   
   # Clean previous builds
   cargo clean
   rm -rf apps/zylcode-desktop/dist
   
   # Build backend
   echo "Building backend..."
   cargo build --release
   
   # Build frontend
   echo "Building frontend..."
   cd apps/zylcode-desktop
   pnpm install
   pnpm build
   cd ../..
   
   # Create distribution
   echo "Creating distribution..."
   mkdir -p dist
   
   # Copy binaries
   cp target/release/zylcode dist/
   cp target/release/zylcode-desktop dist/
   
   # Copy frontend assets
   cp -r apps/zylcode-desktop/dist/* dist/
   
   # Create platform-specific package
   case "$(uname -s)" in
       Linux*)
           echo "Creating Linux package..."
           tar -czf zylcode-linux-$(uname -m).tar.gz -C dist .
           ;;
       Darwin*)
           echo "Creating macOS package..."
           # Create .app bundle
           mkdir -p dist/ZylCode.app/Contents/MacOS
           mkdir -p dist/ZylCode.app/Contents/Resources
           cp dist/zylcode-desktop dist/ZylCode.app/Contents/MacOS/
           cp dist/zylcode dist/ZylCode.app/Contents/MacOS/
           cp -r dist/* dist/ZylCode.app/Contents/Resources/
           
           # Create Info.plist
           cat > dist/ZylCode.app/Contents/Info.plist << EOF
   <?xml version="1.0" encoding="UTF-8"?>
   <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
   <plist version="1.0">
   <dict>
       <key>CFBundleExecutable</key>
       <string>zylcode-desktop</string>
       <key>CFBundleIdentifier</key>
       <string>com.zylcode.desktop</string>
       <key>CFBundleName</key>
       <string>ZylCode</string>
       <key>CFBundleVersion</key>
       <string>1.0.0</string>
       <key>CFBundleShortVersionString</key>
       <string>1.0.0</string>
       <key>NSHighResolutionCapable</key>
       <true/>
   </dict>
   </plist>
   EOF
           
           # Create DMG
           hdiutil create -volname "ZylCode" -srcfolder dist/ZylCode.app -ov -format UDZO zylcode-macos-$(uname -m).dmg
           ;;
       MINGW*|MSYS*|CYGWIN*)
           echo "Creating Windows package..."
           zip -r zylcode-windows-$(uname -m).zip dist/*
           ;;
   esac
   
   echo "Build complete!"
   ```

2. **Run the build script**:
   ```bash
   chmod +x build.sh
   ./build.sh
   ```

## Testing the Application

### 1. Basic Functionality Test

1. **Launch the application**
   ```bash
   # Windows
   zylcode-gui.exe
   
   # macOS
   open -a ZylCode
   
   # Linux
   zylcode-desktop
   ```

2. **Test MCP Bridge**
   ```bash
   # List available tools
   zylcode tools list
   
   # Execute a tool
   zylcode tools execute git.commit --params '{"message": "test commit"}'
   ```

3. **Test Skills System**
   ```bash
   # List available skills
   zylcode skills list
   
   # Execute a skill
   zylcode skills execute code.review --params '{"file": "src/main.rs"}'
   ```

4. **Test Plugin Marketplace**
   ```bash
   # List available plugins
   zylcode plugins list
   
   # Install a plugin
   zylcode plugins install code.review.bot
   ```

### 2. Performance Testing

1. **Run benchmarks**
   ```bash
   cargo bench
   ```

2. **Test tool execution speed**
   ```bash
   # Time tool execution
   time zylcode tools execute git.commit --params '{"message": "test"}'
   
   # Expected: <100ms
   ```

3. **Test memory usage**
   ```bash
   # Monitor memory usage
   top -p $(pgrep zylcode)
   
   # Expected: <512MB
   ```

### 3. Cross-Platform Testing

1. **Test on Windows**
   ```powershell
   # Test GUI
   zylcode-gui.exe
   
   # Test CLI
   zylcode.exe --help
   
   # Test tools
   zylcode.exe tools list
   ```

2. **Test on macOS**
   ```bash
   # Test GUI
   open -a ZylCode
   
   # Test CLI
   zylcode --help
   
   # Test tools
   zylcode tools list
   ```

3. **Test on Linux**
   ```bash
   # Test GUI
   zylcode-desktop
   
   # Test CLI
   zylcode --help
   
   # Test tools
   zylcode tools list
   ```

### 4. Integration Testing

1. **Run all tests**
   ```bash
   cargo test
   ```

2. **Run specific test suites**
   ```bash
   # MCP Bridge tests
   cargo test --package zylcode-mcp
   
   # Skills system tests
   cargo test --package zylcode-mcp --lib enhanced_skills
   
   # Plugin marketplace tests
   cargo test --package zylcode-mcp --lib enhanced_plugin_marketplace
   ```

3. **Run frontend tests**
   ```bash
   cd apps/zylcode-desktop
   pnpm test
   ```

## Cross-Platform Configuration

### Platform-Specific Settings

1. **Windows Configuration**
   ```json
   // config/windows.json
   {
     "platform": "windows",
     "shell": "powershell",
     "path_separator": "\\",
     "executable_extension": ".exe",
     "install_path": "C:\\Program Files\\ZylCode"
   }
   ```

2. **macOS Configuration**
   ```json
   // config/macos.json
   {
     "platform": "macos",
     "shell": "zsh",
     "path_separator": "/",
     "executable_extension": "",
     "install_path": "/Applications/ZylCode.app"
   }
   ```

3. **Linux Configuration**
   ```json
   // config/linux.json
   {
     "platform": "linux",
     "shell": "bash",
     "path_separator": "/",
     "executable_extension": "",
     "install_path": "/usr/local/bin"
   }
   ```

### Environment Variables

1. **Common Environment Variables**
   ```env
   ZYLCODE_HOME=~/.zylcode
   ZYLCODE_CONFIG=~/.zylcode/config
   ZYLCODE_DATA=~/.zylcode/data
   ZYLCODE_LOGS=~/.zylcode/logs
   ```

2. **Windows Environment Variables**
   ```powershell
   [Environment]::SetEnvironmentVariable("ZYLCODE_HOME", "$env:USERPROFILE\.zylcode", [EnvironmentVariableTarget]::User)
   [Environment]::SetEnvironmentVariable("ZYLCODE_CONFIG", "$env:USERPROFILE\.zylcode\config", [EnvironmentVariableTarget]::User)
   ```

3. **macOS/Linux Environment Variables**
   ```bash
   echo 'export ZYLCODE_HOME="$HOME/.zylcode"' >> ~/.zshrc
   echo 'export ZYLCODE_CONFIG="$HOME/.zylcode/config"' >> ~/.zshrc
   source ~/.zshrc
   ```

## Troubleshooting

### Common Issues

#### Windows Issues

1. **"VCRUNTIME140.dll not found"**
   ```powershell
   # Install Visual C++ Redistributable
   Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vc_redist.x64.exe" -OutFile "vc_redist.exe"
   .\vc_redist.exe /install /quiet
   ```

2. **"Windows Defender blocks the application"**
   ```powershell
   # Add exclusion
   Add-MpPreference -ExclusionPath "C:\Program Files\ZylCode"
   ```

3. **"Rust not found"**
   ```powershell
   # Restart terminal after Rust installation
   # Or manually add to PATH
   $env:PATH += ";$env:USERPROFILE\.cargo\bin"
   ```

#### macOS Issues

1. **"App is damaged and can't be opened"**
   ```bash
   # Remove quarantine attribute
   xattr -cr /Applications/ZylCode.app
   ```

2. **"Developer cannot be verified"**
   ```bash
   # Open System Preferences > Security & Privacy > General
   # Click "Open Anyway" next to the blocked app message
   ```

#### Linux Issues

1. **"Permission denied"**
   ```bash
   # Make executable
   chmod +x /usr/local/bin/zylcode
   chmod +x /usr/local/bin/zylcode-desktop
   ```

2. **"Missing dependencies"**
   ```bash
   # Ubuntu/Debian
   sudo apt install -y libwebkit2gtk-4.0-dev libgtk-3-dev
   
   # Fedora
   sudo dnf install -y webkit2gtk3-devel gtk3-devel
   ```

### Debug Mode

1. **Enable debug logging**
   ```bash
   # Set log level
   export RUST_LOG=debug
   
   # Run with debug output
   zylcode --debug
   ```

2. **View logs**
   ```bash
   # Windows
   Get-Content "$env:USERPROFILE\.zylcode\logs\zylcode.log" -Tail 100
   
   # macOS/Linux
   tail -f ~/.zylcode/logs/zylcode.log
   ```

### Performance Issues

1. **Slow startup**
   ```bash
   # Clear cache
   rm -rf ~/.zylcode/cache
   
   # Rebuild
   cargo clean && cargo build --release
   ```

2. **High memory usage**
   ```bash
   # Monitor memory
   # Windows: Task Manager
   # macOS: Activity Monitor
   # Linux: htop
   ```

## Quick Start Guide

### Windows Quick Start
```powershell
# 1. Download and extract
Invoke-WebRequest -Uri "https://github.com/zylcode/zylcode/releases/latest/download/zylcode-windows-x64.zip" -OutFile "zylcode.zip"
Expand-Archive -Path "zylcode.zip" -DestinationPath "C:\ZylCode"

# 2. Run
cd C:\ZylCode
.\zylcode-gui.exe

# 3. Test
.\zylcode.exe tools list
```

### macOS Quick Start
```bash
# 1. Download and install
curl -L -o zylcode.dmg "https://github.com/zylcode/zylcode/releases/latest/download/zylcode-macos-arm64.dmg"
hdiutil attach zylcode.dmg
cp -R /Volumes/ZylCode/ZylCode.app /Applications/
hdiutil detach /Volumes/ZylCode

# 2. Run
open -a ZylCode

# 3. Test
zylcode tools list
```

### Linux Quick Start
```bash
# 1. Download and extract
wget https://github.com/zylcode/zylcode/releases/latest/download/zylcode-linux-x86_64.tar.gz
tar -xzf zylcode-linux-x86_64.tar.gz

# 2. Install
sudo cp zylcode /usr/local/bin/
sudo cp zylcode-desktop /usr/local/bin/

# 3. Run
zylcode-desktop

# 4. Test
zylcode tools list
```

## Conclusion

ZylCode is a cross-platform AI coding assistant that runs on Windows, macOS, and Linux. By following this guide, you can install, build, and test ZylCode on any platform.

For more information, refer to the User Guide and Developer Guide.
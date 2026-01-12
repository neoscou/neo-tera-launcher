# Custom MSI Installer for Neolithic TERA

This directory contains the WiX source files and build scripts for creating a custom MSI installer that bundles the game client files.

## Prerequisites

- **WiX Toolset 3.11+**: Download from https://wixtoolset.org/releases/
- PowerShell (included with Windows)

## Files Included in MSI

The installer bundles:
- `Neolithic TERA Launcher.exe` - Main launcher application
- `file_cache.json` - File integrity cache
- `Binaries/` - Game binaries folder (~110 MB)
- `Engine/` - Game engine folder (~10 MB)

**Total MSI size: ~140-150 MB**

## Excluded Files

The following are NOT included and will be downloaded by the launcher on first run:
- `tera_config.ini` - User-specific configuration (auto-generated)
- `debug.log` - Runtime log file
- `S1Game/` - Large game data files (~56 GB, downloaded via patcher)

## Building the MSI

1. **Build the launcher executable first:**
   ```powershell
   cd ..\teralaunch
   npm run tauri build
   ```

2. **Run the build script:**
   ```powershell
   cd ..\CustomInstaller
   .\build-msi.ps1
   ```

3. **Custom paths (optional):**
   ```powershell
   .\build-msi.ps1 `
       -GameServerPath "D:\Custom\Path\To\Server" `
       -LauncherExePath "D:\Custom\Path\To\Launcher.exe" `
       -OutputDir ".\Release"
   ```

## Output

The MSI installer will be created at:
```
.\Output\NeolithicTERA-Setup.msi
```

## Installation

Users can install by:
1. Double-clicking the MSI file
2. Following the installation wizard
3. Launching from Desktop or Start Menu shortcut

On first launch, the launcher will:
- Prompt for game folder location
- Create `tera_config.ini` automatically
- Begin downloading missing game files

## Files Generated During Build

- `BinariesFiles.wxs` - Auto-generated WiX source for Binaries folder
- `EngineFiles.wxs` - Auto-generated WiX source for Engine folder
- `*.wixobj` - Compiled WiX objects
- `SourceFiles/` - Staging directory (can be deleted after build)

## Troubleshooting

**Error: WiX Toolset not found**
- Install WiX Toolset from https://wixtoolset.org/releases/
- Ensure `%WIX%` environment variable is set (installer does this automatically)

**Error: Game server not found**
- Verify the path in the build script matches your server location
- Use `-GameServerPath` parameter to specify custom location

**Error: ICE validation errors**
- These are usually warnings and can be ignored
- Add `-sval` to the light.exe command in build-msi.ps1 to suppress

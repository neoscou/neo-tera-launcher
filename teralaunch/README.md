# Neolithic TERA Launcher

A modern game launcher built with Tauri (Rust + JavaScript) for TERA Online private servers.

## Features

- Account registration and login
- Server selection and character management
- Game file verification and patching
- Automatic updates
- TeraToolbox integration (install/uninstall)
- Modern UI with progress tracking

## Prerequisites

Before building the launcher, ensure you have the following installed:

### Required Software

1. **Rust** (latest stable)
   - Download from: https://rustup.rs/
   - Install with default settings
   - Verify: `rustc --version`

2. **Node.js** (v18 or later)
   - Download from: https://nodejs.org/
   - Verify: `node --version` and `npm --version`

3. **Visual Studio Build Tools** (Windows only)
   - Download: https://visualstudio.microsoft.com/downloads/
   - Install "Desktop development with C++" workload
   - Required for compiling native dependencies

### Optional Tools

- **Git** for version control
- **VS Code** with Rust Analyzer and Tauri extensions

## Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/neoscou/neo-tera-launcher.git
   cd neo-tera-launcher/tera-rust-launcher/teralaunch
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Configure environment variables**
   
   Edit `src-tauri/.env` with your API endpoints:
   ```env
   LOGIN_ACTION_URL=https://yourdomain.com/tera/LauncherLoginAction
   ACCOUNT_INFO_URL=https://yourdomain.com/launcher/GetAccountInfoAction
   SERVER_LIST_URL=https://yourdomain.com/tera/ServerList.json
   HASH_FILE_URL=https://www.neolithictera.com/TeraDirect/hash-file.json
   FILE_SERVER_URL=https://www.neolithictera.com/TeraDirect
   ```

## Building

### Development Mode

Run the launcher in development mode with hot-reload:

```bash
npm run tauri dev
```

This will:
- Compile the Rust backend
- Start the development server
- Launch the application window
- Auto-reload on file changes

**Note:** The `target` folder will be created automatically during the first build. This contains compiled Rust binaries and is excluded from Git (it can be 1-2GB).

### Production Build

Create an optimized production build:

```bash
npm run tauri build
```

The installer will be generated in:
```
src-tauri/target/release/bundle/
```

Available formats:
- **Windows**: `.exe` installer and `.msi` package in `bundle/msi/` and `bundle/nsis/`
- Executable: `target/release/teralaunch.exe` or `Neolithic TERA Launcher.exe`

## Project Structure

```
teralaunch/
├── src/                    # Frontend source code
│   ├── app.js             # Main application logic
│   ├── home.html          # Home page UI
│   └── styles.css         # Styling
├── src-tauri/             # Rust backend
│   ├── src/
│   │   └── main.rs        # Tauri commands and API
│   ├── .env               # API configuration
│   ├── Cargo.toml         # Rust dependencies
│   └── target/            # Build output (auto-generated, not in Git)
├── package.json           # Node.js dependencies
└── README.md
```

## Build Artifacts (Not in Git)

These folders are auto-generated during build and excluded from Git:

- `src-tauri/target/` - Rust compilation output (1-2GB)
- `node_modules/` - Node.js dependencies
- `dist/` - Frontend build output

These will be created automatically when you run `npm install` and `npm run tauri dev/build`.

## Troubleshooting

### "target folder doesn't exist"
This is normal - the `target` folder is created automatically during your first build. Just run `npm run tauri dev` and it will be generated.

### Build errors on Windows
Ensure Visual Studio Build Tools are installed with C++ development tools.

### Environment variable issues
Make sure `src-tauri/.env` exists and contains all required URLs.

### Port conflicts
If port 1420 is in use, the dev server will automatically try the next available port.

## Development

- Frontend changes auto-reload in dev mode
- Rust changes require restart (Ctrl+C and re-run `npm run tauri dev`)
- Check `src-tauri/debug.log` for backend logs

## License

Private repository - access restricted to invited collaborators only.

## Support

For issues or questions, contact the repository maintainers.

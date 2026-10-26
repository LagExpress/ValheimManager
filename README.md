# 🛡️ Valheim Modpack Studio

A fast, cross-platform Rust GUI application designed to detect mods and configurations from your **r2modman** profiles, provide granular selection, automatically package Thunderstore-ready modpack archives (`.zip`), and deploy BepInEx files to a dedicated Valheim server over SFTP.

Runs natively on **Linux** (as a standalone binary and **AppImage**) and **Windows** (as a **.exe**).

---

## ✨ Features

- **⚡ Native Rust Performance**: Built with `egui` / `eframe` for an immediate-mode, lightweight, responsive desktop GUI.
- **📂 Profile Auto-Detection**:
  - Automatically locates r2modman profiles across Linux (`~/.config/r2modmanPlus-local/Valheim/profiles`) and Windows (`%APPDATA%/r2modmanPlus-local/Valheim/profiles`).
  - Supports custom profile directories via folder browser.
- **📦 Granular Mod Dependencies**:
  - Automatically parses `mods.yml` to extract exact mod versions, authors, and Thunderstore IDs.
  - Interactive search and filter.
  - Quick buttons: *Select All*, *Select Enabled Only*, *Deselect All*.
  - **Inline Version Overrides**: Click and edit any mod's version directly inside the table.
- **⚙️ Curated Configuration Selection**:
  - Reads all `.cfg` files and preset folders from `BepInEx/config/`.
  - **Security Filter**: Auto-excludes sensitive files (such as `dezzycode.betrvalheim.quickconnect.dat` containing server passwords and player-specific `.dat` files).
  - **Performance Filter**: Auto-excludes runtime caches (`MultiplayerClientCache` world records and performance analyzer profiling dumps).
  - Categorization badges: `SAFE`, `SENSITIVE`, and `CACHE`.
- **🏷️ Thunderstore Modpack Compliant**:
  - Generates valid `manifest.json` with dependency array.
  - Auto-generates markdown `README.md` with a clean table of mods, versions, and installation instructions.
  - Generates/embeds a compliant 256×256 `icon.png` (or accepts a custom image).
  - Packs into a clean `.zip` in `dist/<Name>-<Version>.zip`.
- **🚀 One-Click Build & Reveal**: Build in seconds and open the output folder directly from the GUI.
- **🔄 Server Sync**: Deploy `BepInEx/config` and `BepInEx/plugins` to a dedicated server over SFTP, with hash-based uploads, safe deletion of removed files, staging refresh, ignore patterns, and automatic repair of the modded server wrapper after game updates.

---

## 🔄 Server Sync

The **Server Sync** tab keeps a dedicated Valheim server's BepInEx folder aligned with the selected profile.

1. Select the source profile on the **Profile & Info** tab.
2. Enter the SFTP host, port, username, and password.
3. Choose a staging folder. The default is `~/Desktop/valheim` (`%USERPROFILE%\Desktop\valheim` on Windows).
4. Use **Preview Changes** to inspect pending uploads and deletions.
5. Use **Sync Changes** to upload changed files and remove remote files that are no longer present locally.

Additional actions:

- **Refresh Staging** mirrors the profile's `BepInEx/config` and `BepInEx/plugins` into the staging folder. Obsolete non-ignored files are pruned.
- **Force Upload All** re-uploads every local file.
- **Upload Only** uploads changes without deleting anything remotely.
- **Check & Fix Wrapper** restores `/valheim_server.x86_64`, backs up a game-update ELF to `/valheim_server.x86_64.bin`, and ensures the data symlink exists.
- **Test Connection** verifies the SFTP credentials and remote BepInEx folder.

Sync state is stored in `<staging>/.sync_state.json`. Ignore patterns are read from `<staging>/.sync_ignore`; the built-in defaults protect runtime state files and large blueprint folders. The server password is stored in plain text in the application config folder (`sync.json`), with owner-only permissions on Unix. SSH host keys are accepted on first use; compare the SHA-256 fingerprint in the sync log with your server's fingerprint.

> **Safety:** the sync engine refuses to delete remote files when the local scan finds zero files, and it never deletes outside `config/` or `plugins/`.

---

## 🖥️ Running the Application

### Linux (NixOS / devenv)

Run the GUI directly in development or release mode:

```bash
# Launch GUI in release mode
devenv shell -- gui

# Or with cargo
cargo run --release
```

---

## 📦 Packaging

### 1. Linux AppImage

To build a standalone, portable `.AppImage` that runs on any Linux distribution:

```bash
# Build the AppImage
devenv shell -- appimage

# Output is generated at:
dist/ValheimModpackStudio-x86_64.AppImage
```

Run the AppImage:
```bash
./dist/ValheimModpackStudio-x86_64.AppImage
```

---

### 2. Windows Executable (`.exe`)

#### Option A: Direct Build on Windows
If you are on Windows, open a command prompt / PowerShell in this directory and run:

```cmd
scripts\build-windows.bat
```
*(Or simply `cargo build --release`)*

The output binary will be created at:
```text
dist\ValheimModpackStudio-x86_64.exe
```

#### Option B: Automated GitHub Actions CI/CD
This repository includes a ready-to-use GitHub Actions workflow at [`.github/workflows/release.yml`](.github/workflows/release.yml):
- On every push or tag (e.g. `v1.0.0`), GitHub Actions automatically compiles both:
  - `ValheimModpackStudio-x86_64.AppImage` (Linux)
  - `ValheimModpackStudio-x86_64.exe` (Windows)
- Builds are uploaded to GitHub Artifacts and published directly to GitHub Releases.

---

## 🛠️ Project Architecture

```text
├── Cargo.toml                      # Rust dependencies & metadata
├── devenv.nix                      # Declarative NixOS environment with GUI libs
├── src/
│   ├── main.rs                     # Native window setup and application entry
│   ├── app.rs                      # egui UI implementation (tabs, tables, events)
│   ├── profile.rs                  # r2modman scanner (mods.yml & BepInEx/config)
│   ├── packager.rs                 # Manifest, README, icon, and ZIP builder
│   ├── sync.rs                     # Staging refresh, hashing, ignore rules, sync engine
│   ├── sftp.rs                     # SFTP client and server wrapper repair
│   └── assets/
│       ├── icon.png                # Embedded 256x256 Viking rune icon
│       └── valheim_server_wrapper.sh # Embedded modded server launch wrapper
├── scripts/
│   ├── build-appimage.sh           # Linux AppImage builder
│   ├── build-windows.sh            # Linux-to-Windows cross-build helper
│   └── build-windows.bat           # Native Windows .exe builder
├── valheim-modpack-studio.desktop  # Linux desktop entry
├── valheim-pack.js                 # Original CLI script (maintained as reference)
└── .github/workflows/release.yml   # Multi-platform CI/CD builder
```

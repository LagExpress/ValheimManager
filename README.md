# 🛡️ Valheim Modpack Studio

A fast, cross-platform Rust GUI application designed to detect mods and configurations from your **r2modman** profiles, provide granular selection, and automatically package Thunderstore-ready modpack archives (`.zip`).

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
│   └── assets/
│       └── icon.png                # Embedded 256x256 Viking rune icon
├── scripts/
│   ├── build-appimage.sh           # Linux AppImage builder
│   └── build-windows.bat           # Windows .exe builder
├── valheim-modpack-studio.desktop  # Linux desktop entry
├── valheim-pack.js                 # Original CLI script (maintained as reference)
└── .github/workflows/release.yml   # Multi-platform CI/CD builder
```

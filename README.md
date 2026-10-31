# 🛡️ Valheim Modpack Studio

A fast, cross-platform Rust GUI application designed to detect mods and configurations from your **r2modman** profiles, provide granular selection, automatically package Thunderstore-ready modpack archives (`.zip`), and deploy BepInEx files to a dedicated Valheim server over SFTP.

Runs natively on **Linux** (as a standalone binary and **AppImage**) and **Windows** (as a **.exe**).

---

## ✨ Features

- **Appearance**: Modern Nordic Light, Dark, Glass Light, and Glass Dark presets. The first launch follows the OS preference; subsequent explicit choices are saved in `appearance.json` beside the application settings.
- **Saved modpack configurations**: Save and switch between modpacks, restoring names, descriptions, declared versions, source profiles, output folders, icons, website URLs, mod selections/version overrides, and configuration selections. The last saved modpack loads at startup.
- **Version choice on build**: Both Build ZIP actions ask whether to keep the declared version or increment its patch number. An **Always increment** checkbox skips the question for that saved modpack. The increment is remembered only when the ZIP succeeds; failed or cancelled builds do not consume a version.
- **Readable activity logs**: Long messages, paths, and fingerprints wrap onto additional lines. A cached layout keeps the bounded 2,000-entry log list virtualized.
- **Responsive layout**: Supports windows from 640 × 480 logical pixels, compact navigation and list rows, wide-screen layouts, and OS display scaling.
- **Native desktop glass**: Uses the background-effect protocol on COSMIC/Wayland, KDE's blur protocol where available, and documented Desktop Acrylic on Windows 11 build 22621 or newer. Compositor and system transparency settings control the effect; unsupported systems use a readable app-drawn glass backdrop.
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
  - Reads files and folders from `BepInEx/config/` and selects ordinary configurations by default.
  - **Security Filter**: Labels sensitive files (such as quick-connect credentials and player-specific `.dat` files) and excludes them by default.
  - **Performance Filter**: Labels runtime caches and reports, which are excluded by default.
  - Review selected configs before sharing; sensitive files and caches can be included explicitly.
  - Categorization badges: `SAFE`, `SENSITIVE`, and `CACHE`.
- **🏷️ Thunderstore Modpack Compliant**:
  - Generates valid `manifest.json` with dependency array.
  - Previews `README.md` as formatted Markdown or source, with custom information before independently optional mod and configuration lists.
  - Includes an editable `CHANGELOG.md`, generated from mod version and selection changes since the previous successful build.
  - Generates/embeds a compliant 256×256 `icon.png` (or accepts a custom image).
  - Packs into a clean `.zip` in `dist/<Name>-<Version>.zip`.
- **🚀 Build & Open**: Build the archive and open its containing output folder directly from the GUI.
- **🔄 Server Sync**: Deploy `BepInEx/config` and `BepInEx/plugins` to a dedicated server over SFTP, with hash-based uploads, safe deletion of removed files, staging refresh, ignore patterns, and optional custom launcher repair for compatible hosts.

---

## Saving a modpack and updating it

1. Enter the modpack metadata and choose its source profile, mods, configurations, and output folder.
2. On **Profile & Info**, use **Save configuration**. Use **Save as new** to keep another modpack separately, and the saved-configuration selector to return to one.
3. Press **Build ZIP** or the Build & Export action. Choose **Keep** to rebuild that version, or **Increment to** to advance the patch version (for example, `1.2.9` → `1.2.10`). Cancelling the dialog leaves the declaration untouched.
4. Enable **Always increment the patch version when building a ZIP** in the metadata/build view, or select the checkbox in the dialog. This preference is stored with that modpack and can be turned off again.

Builds also save the current configuration automatically. Only a successful build commits an incremented version. Installed mod updates are picked up when reloading a profile; explicit dependency version overrides remain pinned.

Presets are stored in `modpacks.json` beside `appearance.json` and `sync.json`; they do not include server credentials. An unreadable saved-configuration file is reported rather than overwritten.

### Package description and changelog

On **Profile & Info**, choose whether the README should include the generated mod
and configuration lists, and enter any additional Markdown. These switches affect
the description only; selected dependencies and configuration files remain in the
package. Generated prose contains no application or store branding; package links
still point to their download pages.

On **Build & Export → Package documents**, preview `README.md` or `CHANGELOG.md`,
switch between formatted output and Markdown source, or copy the generated text.
The preview displays its build version, including the next patch when automatic
incrementing is enabled. Choosing a different version in the build dialog updates
the documents for that build.

Changelog notes compare mod versions and selected mods/configuration entries with
the previous successful version. Edit the current notes to describe gameplay fixes
or changes inside configuration files; file contents are not compared
automatically. **Regenerate current release notes** replaces the current draft
with generated notes. Earlier versions remain in the exported changelog. Rebuilding
the same version updates that entry without duplicating it. Notes, README options,
and history are saved per modpack; failed builds retain the draft and do not advance
release history.

## 🔄 Server Sync

Save named servers under **Server Connection** and switch using the server selector.
Each server remembers its connection credentials, trusted key, staging folder, and
hosting compatibility preference. Switching preserves the current form and clears
the previous server's preview, summary, and activity log. Server management is
disabled during an active operation. Preview and activity panels now show up to
480 and 520 points of content respectively; the page remains vertically scrollable.

The **Server Sync** tab keeps a dedicated Valheim server's BepInEx folder aligned with the selected profile.

**Hosting compatibility:** custom BepInEx launcher checks and repair are **off by default**, including when older saved settings have no value for this option. Leave this off when the provider already supplies BepInEx or does not allow executable replacement. To use the original custom-host behavior, enable **Enable custom BepInEx launcher checks and repair** under **Server Connection → Hosting compatibility**. The checkbox saves immediately. Enabled checks inspect the server launcher during preview and repair it before uploads; they may rename/replace `valheim_server.x86_64` and create its data symlink. When disabled, these operations are skipped and **Check & Fix Wrapper** is disabled. Config/plugin sync, SSH host-key verification, and `/BepInEx` folder access checks remain available.

1. Select the source profile on the **Profile & Info** tab.
2. Enter the SFTP host, port, username, and password.
3. Choose a staging folder. The default is `~/Desktop/valheim` (`%USERPROFILE%\Desktop\valheim` on Windows).
4. On first connection, the app displays the server's SHA-256 host-key fingerprint and blocks authentication until you verify it out of band and enter its 64-character hex value in **Trusted server key**. The `SHA256:` prefix is optional.
5. Use **Preview Changes** to inspect pending uploads and deletions.
6. Use **Sync Changes** to upload changed files and remove remote files that are no longer present locally. Deletions are skipped if any upload fails.

Additional actions:

- **Refresh Staging** mirrors the profile's `BepInEx/config` and `BepInEx/plugins` into the staging folder. Obsolete non-ignored files are pruned.
- **Force Upload All** re-uploads every local file.
- **Upload Only** uploads changes without deleting anything remotely.
- **Check & Fix Wrapper**, available when custom launcher management is enabled, restores `/valheim_server.x86_64`, backs up a game-update ELF to `/valheim_server.x86_64.bin`, and ensures the data symlink exists.
- **Test Connection** verifies the SFTP credentials and remote BepInEx folder.

Sync state is stored in `<staging>/.sync_state.json`. Ignore patterns are read from `<staging>/.sync_ignore`; the built-in defaults protect runtime state files and large blueprint folders. The server password is stored in plain text in the application config folder (`sync.json`), with owner-only permissions on Unix.

> **Safety:** the sync engine refuses to delete remote files when the local scan finds zero files, never deletes outside `config/` or `plugins/`, rejects remote symlinks, and skips deletions if an upload fails.

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

To build an AppImage for x86-64 Linux desktops, including NixOS:

```bash
# Build the AppImage
devenv shell -- appimage

# Output is generated at:
dist/ValheimManager-0.1.10.AppImage
```

Run the AppImage:
```bash
./dist/ValheimManager-0.1.10.AppImage
```

If FUSE mounting is unavailable, use
`./dist/ValheimManager-0.1.10.AppImage --appimage-extract-and-run`.

CI and local builds use the same packager. It bundles the ELF loader, glibc, and
the Wayland/X11/OpenGL/Vulkan client libraries with their dependencies, keyboard
layouts, and compose tables. The
launcher uses a newer host glibc when available to support the host's graphics
drivers, and does not require `nix-ld` or `appimage-run` on NixOS. The host still
provides its kernel, desktop session, and compatible graphics drivers; the
AppImage does not bundle GPU drivers or support other CPU architectures.

All project build and validation tools are declared in `devenv.nix`. Check a
packaged runtime or open it on an isolated headless test display with:

```bash
devenv shell -- bash scripts/check-appimage.sh dist/ValheimManager-0.1.10.AppImage
devenv shell -- bash scripts/smoke-appimage.sh dist/ValheimManager-0.1.10.AppImage
```

The smoke test uses temporary settings and home directories and never connects
to a hosting server. Logs are kept under `target/appimage-smoke.*`. It checks
startup, not full visual correctness. Local and CI sizes/hashes can differ due
to their compiler and library builds; verify each download with its own checksum.

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
dist\ValheimManager-0.1.10.exe
```

#### Option B: Automated GitHub Actions CI/CD
This repository includes a ready-to-use GitHub Actions workflow at [`.github/workflows/release.yml`](.github/workflows/release.yml):
- Pushes to `main`, pull requests to `main`, and version tags run quality checks and build both:
  - `ValheimManager-0.1.10.AppImage` (Linux)
  - `ValheimManager-0.1.10.exe` (Windows)
- Builds are uploaded to GitHub Actions artifacts. A tag matching the Cargo version (for example `v0.1.10`) also publishes a GitHub release containing the EXE, AppImage, and SHA-256 checksums. Source is available through GitHub's built-in Source code (zip) and Source code (tar.gz) downloads for the tag; CI does not upload duplicate source archives.
- Release descriptions come from [CHANGELOG.md](CHANGELOG.md). Before tagging a version, add its entry under `## [<version>] - YYYY-MM-DD`, using Added, Changed, Fixed, Removed, or Security sections as needed. CI rejects a version without a nonempty changelog entry and publishes that entry as the release description.

---

## 🛠️ Project Architecture

The shared appearance tokens, reusable UI components, and native effect adapter keep screen layouts independent of platform-specific transparency. [Dependency review](docs/dependency-audit-2026-10-29.md), [UI validation](docs/ui-validation-2026-10-29.md), and [release 0.1.7](docs/release-0.1.7.md) document the checks and platform limits.

```text
├── Cargo.toml                      # Rust dependencies & metadata
├── devenv.nix                      # Declarative NixOS environment with GUI libs
├── src/
│   ├── main.rs                     # Native window setup and application entry
│   ├── app.rs                      # Application state and worker events
│   ├── app/ui.rs                   # Responsive screens and navigation
│   ├── appearance.rs               # Theme palettes, bundled font, preferences
│   ├── modpack_config.rs           # Saved modpack library and checked version increments
│   ├── app/saved.rs                # Preset restoration and build-version lifecycle
│   ├── app/readme.rs               # Cached package-document previews
│   ├── package_docs.rs             # README choices and editable package changelog history
│   ├── ui_components.rs            # Shared cards, forms, icons, and backdrop
│   ├── platform_effects.rs         # Optional Wayland and Windows effects
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

# Changelog

Release notes describe changes introduced in each version. Use Added, Changed,
Fixed, Removed, and Security sections where applicable. Keep entries focused on
user-visible behavior; omit build reports, test counts, and development history.

## [Unreleased]

### Added

- Named server profiles with independent connection settings, trusted keys, staging folders, and hosting preferences.

### Changed

- Increased the Server Sync preview and activity log heights to show more content before scrolling.

## [0.1.9] - 2026-10-30

### Added

- Formatted and Markdown-source previews of package README and changelog documents, with a copy action.
- Saved custom Markdown before the generated lists, with separate switches for including mod and configuration lists in descriptions.
- Editable package changelogs generated from mod version and selection changes, with history saved after successful builds.

### Changed

- Simplified generated descriptions to package information and optional lists, without application branding or installation boilerplate.
- Used canonical package-page links and a neutral Package column in generated mod tables.

## [0.1.8] - 2026-10-30

### Changed

- Moved Server Sync actions into a full-width toolbar above ignore settings and the connection/inspection panels.
- Ignore settings open by default on wide windows and start collapsed on compact windows.

## [0.1.7] - 2026-10-29

### Fixed

- Fixed AppImage startup on NixOS by launching through a real ELF loader instead of the system's compatibility stub.
- Included the C runtime and required Wayland, X11, OpenGL, Vulkan-loader, and compression libraries in AppImages.
- Removed build-machine library search paths from packaged libraries.
- Bundled XKB keyboard layouts and compose tables to prevent Wayland startup failures on systems without standard X11 data paths.
- Kept bundled library paths out of the environment inherited by host programs such as the output-folder opener.

### Changed

- CI and local builds now use the same AppImage packager, launcher, and checksum-verified runtime.
- Packaging checks library resolution and CI opens the packaged application in a test display before uploading it.
- Declared packaging, analysis, and headless test tools in the devenv environment.

## [0.1.6] - 2026-10-29

### Changed

- Standardized desktop download names as `ValheimManager-<version>.exe` and `ValheimManager-<version>.AppImage`.
- Build scripts now read the application version from `Cargo.toml` when naming output files.

### Removed

- Duplicate source ZIP and tar.gz attachments from releases. Source remains available through GitHub's built-in downloads.

## [0.1.5] - 2026-10-29

### Added

- **Enable custom BepInEx launcher checks and repair** under **Server Sync → Server Connection → Hosting compatibility**, saved between sessions.

### Changed

- Custom launcher management is now disabled by default, including for existing installations. Plugin and configuration sync remains available without permission to replace the server executable.
- **Check & Fix Wrapper** is available only when custom launcher management is enabled.

### Fixed

- Suppressed missing-wrapper and data-symlink warnings when custom launcher checks are disabled.

## [0.1.4] - 2026-10-29

### Added

- Light, Dark, Glass Light, and Glass Dark themes with a saved appearance preference.
- Saved modpack configurations for metadata, source profiles, output folders, mod selections, and configuration selections. The last saved configuration loads at startup.
- A build-time choice to keep or increment the modpack version, with an automatic patch-version increment option for each configuration.
- Native glass effects on supported compositors, with an opaque fallback where effects are unavailable.

### Changed

- Redesigned all six screens with consistent typography, controls, vector icons, and status colors.
- Adapted navigation, forms, tables, and sync panels for windows down to 640 × 480.
- Cached filtered list results and text layouts while retaining virtualized mod, configuration, preview, and log lists.
- Opening the output folder now runs in the background and reports launch errors in the status area.

### Fixed

- Long activity-log messages now wrap within the log panel, including paths and unbroken strings.
- The Sync Inspection scrollbar stays at the panel's right edge when paths or filters change.
- The Dedicated Server Sync page scrollbar now sits at the window's right edge, keeping all controls reachable.
- Failed ZIP builds preserve the previous archive and modpack version.

### Security

- Updated `zip` to 9.1.3, resolving known ZIP dependency advisories.
- Removed unmaintained transitive dependencies from the output-folder integration.

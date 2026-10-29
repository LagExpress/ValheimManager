# Changelog

Release notes describe changes introduced in each version. Use Added, Changed,
Fixed, Removed, and Security sections where applicable. Keep entries focused on
user-visible behavior; omit build reports, test counts, and development history.

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

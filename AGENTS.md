# Agent guide

This guide applies to the entire repository. It contains the project context and
constraints needed to work without earlier conversations. Follow the current
user's request; verify implementation details in the owning source files rather
than assuming this guide or an old report describes the latest code.

## Start here

1. Run `git status --short --branch`. Preserve unrelated changes and inspect the
   current branch before editing. `main` is the integration/default branch; the
   former `modernization` branch was merged and deleted. Do not recreate it by
   assumption. Follow the user's branch choice for new work.
2. Read `README.md`, `Cargo.toml`, and the relevant module below. Read
   `CHANGELOG.md` before changing versioning or releases.
3. Make the smallest complete change that meets the request. Keep business logic
   outside rendering code where possible. Add regression coverage for meaningful
   behavior changes, especially persistence, packaging, and sync safety.
4. Run checks appropriate to the change. Report what changed, which checks
   actually ran, and any remaining limitation. Do not describe a cross-build or
   offscreen rendering test as a native desktop visual check.
5. Update this guide when module ownership, commands, or important behavior
   changes. It should remain useful without chat history.

## What the application does

ValheimManager is a Rust desktop application for selecting mods/configurations
from r2modman, Gale, or custom profiles, building Thunderstore-ready modpack ZIPs,
and syncing BepInEx files to a dedicated server over **SFTP**, using `ssh2`.
The GUI has six screens: Profile & Info, Mods, Configs, Build & Export, Server
Sync, and Instructions. Both English and Spanish are supported.

The displayed product name is still **Valheim Modpack Studio**. Its Cargo binary,
desktop ID, and settings directory use `valheim-modpack-studio`. Release downloads
use **ValheimManager**. These names intentionally differ: changing artifact names
does not authorize renaming internal IDs or moving existing user settings.

## Where changes belong

| Area | Owning files |
| --- | --- |
| Window setup, application entry, desktop integration | `src/main.rs` |
| Application state, workers, events, list/layout caches | `src/app.rs` |
| Six screens, navigation, forms, preview/log rendering | `src/app/ui.rs` |
| Saved-configuration restoration and build-version lifecycle | `src/app/saved.rs` |
| Serializable modpack library and patch-version increments | `src/modpack_config.rs` |
| Theme presets, palettes, fonts, appearance persistence | `src/appearance.rs` |
| Reusable cards, fields, badges, vector icons, layout constants | `src/ui_components.rs` |
| Native Wayland/Windows effects and fallback activation | `src/platform_effects.rs` |
| English/Spanish labels | `src/i18n.rs`, plus localized helpers in `src/app/ui.rs` |
| Profile discovery, YAML parsing, configuration classification | `src/profile.rs` |
| Manifest, README, icon, streaming ZIP creation | `src/packager.rs` |
| Staging, hashes, ignore matching, sync requests/events | `src/sync.rs` |
| SSH host-key validation, SFTP operations, launcher repair | `src/sftp.rs` |
| UI fixtures, responsive tests, optional visual/native tools | `src/app/ui/tests.rs` |
| Desktop packaging | `scripts/build-appimage.sh`, `scripts/build-windows.sh`, `scripts/build-windows.bat` |
| Release descriptions and CI | `CHANGELOG.md`, `scripts/release-notes.sh`, `.github/workflows/release.yml` |

`valheim-pack.js` is the original CLI/reference implementation, not the desktop
application entry point. It has no npm application to build. Keep its syntax
valid when editing it; do not port desktop work into it by mistake.

## Development environment: NixOS and devenv

- Use the existing `devenv.nix`, `devenv.yaml`, `devenv.lock`, and
  `rust-toolchain.toml`. Run project commands from the repository root using
  `devenv shell -- <command>`; do not assume direnv has activated the environment.
- Declare persistent project tools, runtimes, GUI/C libraries, or target changes
  in the environment files. Keep the Rust version consistent between devenv,
  `rust-toolchain.toml`, and CI when changing it.
- Do not use `apt`, `dnf`, `brew`, global npm installs, or global pip installs on
  this development host. Commands in GitHub's Ubuntu runner are a separate
  environment; their `apt-get` steps are not local setup instructions.
- Use `nix-shell -p <tool> --run '<command>'` or `nix run` for one-off tools.
  Do not add temporary analysis tools to the project environment unnecessarily.
- Use `#!/usr/bin/env bash` for shell scripts. Do not assume `/bin/bash` or a
  conventional Linux dynamic linker exists. Prefer Nix tooling over downloaded
  Linux tool binaries. Diagnose missing ELF interpreters/libraries through Nix.
- If sandbox permissions block the Nix daemon, network, or GUI access, use the
  environment's approval mechanism. Do not replace the toolchain to work around it.

Common commands:

```sh
devenv shell -- cargo check --locked
devenv shell -- gui
devenv shell -- cargo fmt --all -- --check
devenv shell -- cargo clippy --locked --all-targets -- -D warnings
devenv shell -- cargo test --locked --all-targets
devenv shell -- node --check valheim-pack.js
devenv shell -- actionlint .github/workflows/release.yml
devenv shell -- cargo-audit audit
```

Use `cargo fmt --all` inside devenv to apply formatting. `--offline` is useful
when dependencies are already cached; a missing cache is not an application bug.
Update `Cargo.lock` intentionally when dependencies or the package version change.
Audit current advisory data for dependency work; do not treat a historical clean
audit as proof today, or suppress an advisory simply to get a passing result.
The YAML dependency is the maintained `yaml_serde` package aliased as
`serde_yaml`; the import name is not evidence that the deprecated crate is used.

## User data and build behavior to preserve

Settings are under the OS configuration directory in `valheim-modpack-studio/`;
use the source's path helpers rather than hardcoding a developer's home directory.

| File | Contract |
| --- | --- |
| `appearance.json` | Optional `ThemePreset` selection. Missing/malformed preferences safely fall back to the OS's plain Light/Dark choice. |
| `modpacks.json` | Versioned `ModpackLibrary`, active configuration, metadata, source, selections, explicit dependency overrides, and automatic-increment preference. No server credentials. |
| `sync.json` | SFTP settings, trusted host fingerprint, language, and hosting compatibility preference. Contains a plaintext password; never print, copy into fixtures, or commit it. Preserve owner-only Unix permissions. |
| `<staging>/.sync_state.json` | Saved path-to-SHA-256 mapping. Preserve the existing hex hash format and compatibility. |
| `<staging>/.sync_ignore` | User ignore patterns, combined with built-in protections. |

- Preserve existing saved data and backward-compatible defaults when adding
  fields. Malformed or unsupported modpack libraries must be reported and left
  intact, not silently replaced with an empty library.
- Saved configurations restore metadata, declared version, source, selections,
  and explicit version overrides. New installed dependency versions remain
  visible unless the user explicitly pinned an override.
- Both ZIP build actions share the same version-choice flow. Users can keep the
  declared version, increment the patch version, or cancel. **Always increment**
  is saved per modpack. A new version is remembered only after ZIP success;
  cancelled/failed/disconnected builds do not consume a version. Keep build
  success and a later settings-save failure distinguishable.
- Keep streaming ZIP writes, path validation, symlink rejection, and transactional
  archive replacement. Errors must propagate; do not silently omit failed files
  or destroy the previous archive. Sensitive configurations and runtime caches
  remain excluded by default, with explicit selection available.
- Use temporary directories and synthetic data for tests. Do not overwrite real
  settings, mod profiles, staging files, or credentials during validation.

## Server sync and hosting compatibility

- `SyncConfig.manage_bepinex_launcher` defaults to **false**, including when
  loading older settings. This supports hosts that already supply BepInEx or
  forbid executable modification.
- When disabled, skip custom executable/wrapper inspection and repair, binary
  renaming/replacement, and data-symlink inspection/creation. Return the explicit
  skipped status without false missing-wrapper warnings. Disable **Check & Fix
  Wrapper**, and enforce the policy in the backend before staging/network work,
  not only by disabling a UI button.
- When enabled, preserve the custom-host workflow that can manage
  `valheim_server.x86_64`, its `.bin` backup, and its data symlink. The preference
  saves immediately, invalidates previews, and cannot change during active sync.
- Ordinary plugin/config sync, SSH host-key validation, and access checks on
  `/BepInEx` work in both modes. Verify the pinned SHA-256 host fingerprint before
  password authentication; do not automatically trust a missing or changed key.
- Remote deletion is limited to safe relative paths under `/BepInEx/config` and
  `/BepInEx/plugins`. Reject traversal and remote symlinks. Never delete remote
  files for an empty local scan or after an upload failure. **Upload Only** must
  not delete remote files. Preserve cancellation and ignore protections.
- Staging refresh can prune obsolete files. Source and staging must not overlap,
  and symlink safeguards must remain. Even Preview can refresh local staging.
  Do not use a real hosting server or real staging data as an implicit test just
  because saved credentials are available; use fixtures/policy tests unless the
  user explicitly authorizes live operations.

## UI, performance, and native effects

- Use shared palettes/components, bundled Inter with its license, vector icons,
  visible keyboard focus, and readable status labels in all four themes. Add
  English and Spanish labels together. Keep paths/logs monospace.
- Theme changes apply immediately without recreating the window or losing
  selections, build progress, sync state, or cached data unnecessarily.
- Support 640 × 480 logical pixels. Center content up to 1440 points. Use the
  shared constants: full tables at the 900-point breakpoint and two-column Sync
  at 1100 points. Measure available content width, not physical screen pixels.
  Compact forms use labels above full-width inputs, and action groups wrap.
- Keep top tabs when localized labels fit, otherwise use the compact page
  selector. Appearance, language, and build actions must remain reachable.
- The Sync page's outer scrollbar belongs at the window's right edge, outside
  centered content. Preview and activity logs have independent, fixed-width
  viewports. Do not let the preview viewport shrink to the longest visible path.
- Long paths need truncation, full-path tooltips, and copy actions where used.
  Activity messages must wrap, including unbroken strings and explicit newlines.
- Preserve virtualization and revision-keyed filter caches. During rendering, do
  not clone full preview lists or scan/filter every item each frame. Wrapped logs
  use cached variable-height layouts/offsets and a maximum of 2,000 entries; resizing,
  filtering, font/scale changes, and appended messages must invalidate the cache.
- Keep ZIP work, SFTP, and output-folder launching off the UI thread. Preserve
  worker event/progress handling and repaint on relevant changes. Idle glass
  windows must not continuously repaint; do not add application blur shaders.
- Native effects stay behind `PlatformEffects`: Wayland background-effect/KDE
  protocols and documented Windows DWM APIs, with a readable opaque fallback.
  Respect compositor capabilities/settings and resize regions. eframe owns the
  native window, surface, and display connection: release only adapter-owned
  protocol objects, never destroy eframe's surface or disconnect its display.

## Validation choices

- For Rust changes, run formatting, Clippy, and relevant tests; run the full
  suite for changes spanning UI state, persistence, packaging, or sync. The
  responsive matrix can take several minutes. Do not disable it to shorten a run.
- UI changes need all four themes, English/Spanish, and these logical sizes:
  640 × 480, 800 × 600, 980 × 656, 1280 × 720, and 1920 × 1080, including 150% and
  200% scaling. Check long paths, focus, wrapping, reachable actions, and large
  lists. Existing fixtures include 1,000-item lists and 2,000 logs.
- Optional UI tools below are ignored by normal tests. Run the needed exporter
  explicitly; the native probe opens a window and requires a desktop session.
  Do not blindly run every ignored test in a headless environment.

```sh
devenv shell -- cargo test export_visual_previews -- --ignored --nocapture
devenv shell -- cargo test export_configuration_previews -- --ignored --nocapture
devenv shell -- env VALHEIM_REQUIRE_NATIVE_BLUR=1 timeout 45s cargo test native_glass_probe -- --ignored --nocapture
```

Captures are written under `target/ui-previews/`. `VALHEIM_PREVIEW_FILTER` filters
the general exporter; `VALHEIM_PREVIEW_HOVER=x,y` simulates pointer hover. Inspect
actual captures rather than assuming successful export proves correct layout.
Native COSMIC, KDE, and Windows visual results must be recorded separately;
compilation does not verify compositor blur or Windows decorations.

For documentation-only changes, check accuracy, links/paths, and run
`git diff --check`; rebuilding the application is unnecessary. Workflow/script
changes need syntax/actionlint checks and the relevant script exercised.

## Packaging and release conventions

- Read the application version from `Cargo.toml`; never assume a version from
  this guide. Application versions and user modpack versions are independent.
- For an application release, update the package version and lockfile, add a
  matching `CHANGELOG.md` entry, and update current README examples as needed.
  Do not bump application versions or move existing tags just to edit release
  descriptions or documentation. Commit/push/tag/publish within the scope the
  user has authorized; do not infer a new release from every edit.
- Changelog headings use `## [<version>] - YYYY-MM-DD`. Use **Added**, **Changed**,
  **Fixed**, **Removed**, and **Security** only when applicable. Write concise,
  factual entries for that version, without conversational summaries, test
  reports, branch housekeeping, or claims that old features are newly added.
  `scripts/release-notes.sh <version>` extracts the published body; CI rejects
  missing/empty entries and uses this body instead of generated release prose.
- Keep these desktop output names, with no architecture suffix:
  `ValheimManager-<version>.AppImage` and `ValheimManager-<version>.exe`.
  Packaging scripts derive filenames from the manifest, not hardcoded versions.

```sh
devenv shell -- appimage
devenv shell -- bash scripts/build-windows.sh
# On native Windows, run scripts\build-windows.bat instead.
```

- Local outputs go to `dist/`. The Linux-to-Windows helper sets MinGW target
  tools and linker shims; do not globally replace the native compiler with
  MinGW. CI builds Windows natively and Linux on its configured Ubuntu runner.
- CI and local AppImages must use `scripts/build-appimage.sh` and `scripts/AppRun`.
  Bundle the loader, glibc, and client-library dependency closure; remove absolute
  build RPATHs and use a compatible newer host glibc for graphics drivers. Do not
  export bundled library paths to host child processes or bundle GPU drivers.
  Keep all packaging/test tools declared in `devenv.nix`. Run
  `scripts/check-appimage.sh <image>` and `scripts/smoke-appimage.sh <image>` inside
  devenv. Check actual CI artifacts on NixOS; a valid container and checksum do
  not prove its payload can start.
- `main` pushes/PRs run checks and desktop builds. A `v<version>` tag must match
  the Cargo version and also triggers release publication. Releases attach only
  the EXE, AppImage, and `SHA256SUMS-<version>.txt`. Use GitHub's built-in source
  ZIP/tar.gz downloads; do not upload duplicate source archives.
- When delivering builds, verify file formats, AppImage runtime/filesystem, and
  SHA-256 hashes. For published releases, download and verify the actual assets
  against their attached checksum file. Local Nix/MinGW and CI builds may differ;
  do not substitute local hashes for CI asset hashes. State artifact paths/links.
- Keep `build/`, `dist/`, `target/`, `.devenv*`, and `.direnv/` out of commits.

## Further evidence

`docs/dependency-audit-2026-10-29.md`,
`docs/ui-validation-2026-10-29.md`, and `docs/release-*.md` contain dated audits,
validation limits, and historical build records. Their versions, branch names,
artifact filenames, test counts, and hashes are historical. Use the current
source, scripts, workflow, changelog, and release assets for present behavior.

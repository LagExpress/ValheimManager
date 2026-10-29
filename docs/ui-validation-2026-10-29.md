# UI modernization validation — 2026-10-29

This UI validation was performed for version **0.1.1** on branch `modernization`. The implementation was subsequently versioned and rebuilt as **0.1.2**; see the [0.1.2 release build](release-0.1.2.md). Appearance preferences live in `appearance.json`, independently of sync credentials. Inter and its SIL Open Font License are embedded in the application; the license is available in the Appearance popover.

## Automated coverage

Formatting and Clippy (`--all-targets -- -D warnings`) pass. The standard Rust suite reports **20 passed, zero failures**, with the screenshot exporter and native-window probe intentionally opt-in.

- The responsive matrix renders all six screens with each of four themes in English and Spanish at 640 × 480, 800 × 600, 980 × 656, 1280 × 720, and 1920 × 1080 logical pixels, each at 100%, 150%, and 200% scaling: **720 combinations**.
- Fixtures contain 1,000 mods, 1,000 configurations, 1,000 pending uploads, and 2,000 log entries. Rendering stays bounded rather than drawing every list item. Cached filter allocations survive theme changes, including while build/sync state is active.
- Preference tests cover missing files, explicit selections, unknown themes, malformed JSON, and persistence. The SHA-256 known-vector test preserves saved-state compatibility. Archive tests read back ZIP contents and verify that invalid inputs and a directory disappearing during compression leave existing output intact. Traversal errors propagate instead of silently skipping files.
- Glass pages settle without requesting continuous repaints. Log validation errors invalidate cached views and retain only the latest 2,000 entries.
- Palette tests check body, muted, and semantic text contrast against glass panels over both black and white desktops. A font test checks every non-ASCII character in the localized interface sources against the bundled fonts.

## Visual and native checks

Offscreen GPU captures are generated under `target/ui-previews/`. The capture set includes all screens at minimum and reference widths, all four themes, English/Spanish compact layouts, wider Sync layouts, scaled samples, and pages scrolled to their bottom. Reviewed samples include long paths, truncated list names, selected and unselected rows, editable versions, status badges, and the final controls on compact pages.

The native-window probe ran successfully in the current **COSMIC/Wayland** session. It requested the background-effect extension, resized the window, and cycled Glass Dark → Light → Glass Light → Dark → Glass Dark. The compositor capability/effect state was enabled for all three glass samples and disabled for both plain samples. Results are saved to `target/ui-previews/native-glass-probe.json`. The adapter releases its own effect objects without destroying eframe's surface or display connection.

Portable fallback captures use an opaque Nordic gradient under the panels. The probe confirms native protocol operation on COSMIC; it is not a pixel-based measurement of the compositor's blur strength.

Two initial COSMIC probe runs completed successfully. Later background repeats stalled after the first frame and were terminated with a timeout. The user-assisted foreground recheck then **passed in 9.07 seconds**: it completed the theme cycle, both window resizes, all five capability/effect samples, and clean adapter shutdown. The exact cause of the background redraw stalls is not established; the successful foreground run does not prove their cause or constitute a pixel-based blur measurement.

**Windows 11 and KDE desktop visual verification remain outstanding.** The Windows GNU release cross-compiles successfully, including the documented DWM Acrylic adapter, but this Linux session cannot verify Windows decorations, system transparency settings, or native Acrylic visually. Cross-compilation is not recorded as a Windows visual pass.

## Reproduction

```sh
devenv shell -- cargo fmt --all -- --check
devenv shell -- cargo clippy --all-targets -- -D warnings
devenv shell -- cargo test --all-targets
devenv shell -- cargo test export_visual_previews -- --ignored --nocapture
devenv shell -- env VALHEIM_REQUIRE_NATIVE_BLUR=1 timeout 45s cargo test native_glass_probe -- --ignored --nocapture
devenv shell -- appimage
devenv shell -- ./scripts/build-windows.sh
```

The preview exporter accepts `VALHEIM_PREVIEW_FILTER` to regenerate matching filenames. Neither the fixtures nor the native probe connects to an SFTP server. See [the dependency audit](dependency-audit-2026-10-29.md) for the RustSec results and versions.

## Artifacts

Both outputs are in `dist/` and retain version **0.1.1**. The AppImage has the Type 2 magic and a valid zstd SquashFS payload; its runtime responds to `--appimage-version`. The Windows artifact is an x86-64 PE32+ executable. The embedded Inter license is present in both release binaries.

| Artifact | SHA-256 |
| --- | --- |
| `ValheimModpackStudio-0.1.1-x86_64.AppImage` | `22afc39b86b0d2c824820941d0ed1c52d97f6f358f6e289c576abddb01400a22` |
| `ValheimModpackStudio-0.1.1-x86_64.exe` | `d888d9e85c8c57ec3669c34ec55d7978c3b7171abddd076456d3e9250fa844f1` |

The same hashes are saved in `dist/SHA256SUMS-0.1.1.txt` and verified with `sha256sum -c`.

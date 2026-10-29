# Release 0.1.4 — 2026-10-29

- The **Sync Inspection & Preview** list keeps the full panel width instead of shrinking to the longest visible path. Its scrollbar stays anchored to the panel's right side when paths, filters, or preview tabs change. Rows remain virtualized, with full-path tooltips for truncated names.
- The **Dedicated Server Sync** page now has a full-window scroll viewport, placing its page scrollbar at the far right of the app. The heading and cards remain centered within the existing 1440-point content limit, and compact forms retain their side padding.
- The same content-width breakpoints continue to control stacked and two-column Sync layouts. The heading stays above the scrolling page content; preview and log lists keep their own independent scroll areas.

Formatting, Clippy with warnings denied, and all **27 Rust tests** pass, including the 720-combination layout matrix and idle-glass checks. GPU captures were reviewed at 1920 × 1080 and 640 × 480 across the four themes, with English/Spanish compact layouts. Preview captures can simulate a pointer using `VALHEIM_PREVIEW_HOVER=x,y` to make floating scrollbar positions visible.

Version 0.1.4 is recorded in Cargo metadata, packaging scripts, CI artifact names, and README paths on branch `modernization`. No dependency or native-effect binding was changed. Windows runtime visual verification remains outside this Linux session's checks.

## Local build artifacts

Both release builds completed successfully. The AppImage runtime responds to `--appimage-version` and contains a valid zstd SquashFS payload. The Windows artifact is an x86-64 PE32+ executable. Reviewed screenshots are retained in `target/ui-previews/0.1.4/`.

| Artifact in `dist/` | SHA-256 |
| --- | --- |
| `ValheimModpackStudio-0.1.4-x86_64.AppImage` | `9bea9251b2b073d1b1c5ab1357d9b75d7ada0e1e199394cbd0fa654c4638a8dc` |
| `ValheimModpackStudio-0.1.4-x86_64.exe` | `58fe4b2dc36990d5da09a645f6fa75719017bd5d13a979139556e91a760a3616` |

These local Nix build hashes are saved in `dist/SHA256SUMS-0.1.4.txt`; both pass `sha256sum -c`. GitHub release CI rebuilds the binaries from tag `v0.1.4` and publishes its own checksum file alongside the EXE, AppImage, and tagged source archives. Use the checksum file attached to the release to verify those downloads.

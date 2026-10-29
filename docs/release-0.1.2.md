# Release build 0.1.2 — 2026-10-29

Version 0.1.2 packages the Nordic UI modernization on branch `modernization`. The app reads its displayed version from Cargo package metadata. The local packaging scripts, Windows batch script, GitHub Actions artifact names, and README paths all use 0.1.2.

The dependency lockfile records the application as 0.1.2. Formatting, Clippy (`--offline --all-targets -- -D warnings`), shell-script syntax, and Git whitespace checks pass. The functional UI, dependency, and COSMIC foreground checks are documented in the [UI validation report](ui-validation-2026-10-29.md).

Rebuild with the configured Nix environment:

```sh
devenv shell -- appimage
devenv shell -- ./scripts/build-windows.sh
```

Artifacts are written to `dist/ValheimModpackStudio-0.1.2-x86_64.AppImage` and `dist/ValheimModpackStudio-0.1.2-x86_64.exe`.

Both release builds completed successfully. The AppImage's x86-64 ELF runtime responds to `--appimage-version` and contains a valid zstd SquashFS payload. The Windows output is an x86-64 PE32+ executable; this build does not add Windows desktop visual verification.

| Artifact | SHA-256 |
| --- | --- |
| `ValheimModpackStudio-0.1.2-x86_64.AppImage` | `1cc2433269d10d0059ac6b3de3d85e4c67f1f8840944a014008908a0f9ee926a` |
| `ValheimModpackStudio-0.1.2-x86_64.exe` | `cb52c8e26f2e1827aab74ebb16bece31b0f4db2c925f019d18ab3684fa50090b` |

These hashes are saved in `dist/SHA256SUMS-0.1.2.txt`; both pass `sha256sum -c`. Versioning applies to the app and artifact filenames; no Git release tag was created.

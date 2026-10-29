# Release 0.1.6 — 2026-10-29

- Desktop downloads now use `ValheimManager-<version>.exe` and `ValheimManager-<version>.AppImage`, with no architecture suffix. Local shell/batch scripts read the version from `Cargo.toml`; CI continues to use its validated Cargo metadata output.
- Release CI uploads the two binaries and their SHA-256 checksum file. Source downloads use GitHub's built-in **Source code (zip)** and **Source code (tar.gz)** links for the tag. CI no longer creates or uploads duplicate source archives.
- All modernization work is integrated into `main`. The merged `modernization` branch is removed after the updated main branch is pushed; `main` remains the default branch.

Application behavior and saved configuration formats are unchanged from 0.1.5. The BepInEx hosting compatibility setting remains available and defaults off.

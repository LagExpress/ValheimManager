# Release 0.1.5 — 2026-10-29

## Optional custom BepInEx launcher management

**Server Connection → Hosting compatibility** now has **Enable custom BepInEx launcher checks and repair**. It defaults to **off**, including when loading older settings without this preference. The checkbox saves immediately in `sync.json`, invalidates an old preview, and cannot change while sync is running.

Leave it off for providers that supply BepInEx or restrict executable replacement. The application skips custom launcher/executable inspection, replacement/renaming, and data-symlink checks/creation. Preview reports an explicit disabled state, and logs do not show false missing-wrapper or missing-symlink warnings. The manual **Check & Fix Wrapper** action is disabled, and direct repair requests are rejected before staging or network work.

For the original custom-host workflow, enable the setting. Preview can inspect the launcher; sync/upload actions can run the existing repair routine; and **Check & Fix Wrapper** becomes available. This may rename or replace `valheim_server.x86_64` and create `valheim_server.x86_64_Data`, so the host must permit those operations.

Plugin/configuration synchronization, SSH host-key validation, and `/BepInEx` access checks remain available in both modes. This switch controls the provider-specific launcher workflow; the application continues to use SFTP.

## Validation and release automation

- All 33 Rust tests passed, including the responsive matrix and idle checks.
- New regression coverage verifies disabled operations never invoke remote-operation closures, enabled behavior and errors are preserved, manual repair is rejected before connection, legacy settings default off, enabled settings round-trip with existing fields, and skipped reports avoid missing-wrapper/symlink warnings.
- Formatting, Clippy with warnings denied, Git whitespace checks, and release-workflow actionlint pass.
- No live hosting server is contacted by the regression tests. The provider-owned executable is not modified during validation.
- Tags `v0.1.4` and `v0.1.5` identify separate commits on `modernization`. Tag-triggered CI reads the Cargo version, verifies the tag, builds the Windows EXE and Linux AppImage, and publishes both plus tagged source ZIP/tar.gz and SHA-256 checksums. GitHub also provides its standard tag source downloads.

Use the checksum file attached to each GitHub release to verify CI-built assets. Local Nix/cross-compiled artifacts can have different hashes from CI builds.

## Local builds

The local Linux AppImage and Windows GNU EXE both build successfully. Their formats are x86-64 ELF/AppImage and PE32+, and their hashes pass `sha256sum -c dist/SHA256SUMS-0.1.5.txt`.

| Local artifact in `dist/` | SHA-256 |
| --- | --- |
| `ValheimModpackStudio-0.1.5-x86_64.AppImage` | `96373d18f36d1b87657b23eeabae60d1dde0aa24b6da738ee92c99da110d77f1` |
| `ValheimModpackStudio-0.1.5-x86_64.exe` | `952e1933dbe259f4096101f5407d6489f66cc10cc36ce40243250a061e26c6f6` |

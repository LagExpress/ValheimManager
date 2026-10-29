# Release 0.1.7 — 2026-10-29

## AppImage startup and packaging

The 0.1.6 CI AppImage was 8,825,336 bytes; the local Nix build was
10,160,632 bytes. CI included no runtime client libraries, while the local
packager included Wayland/X11/Vulkan libraries. Their launchers and AppImage
runtime revisions also differed. Compression and binary build inputs can change
sizes, but the missing library bundle was a concrete packaging difference.

The CI executable requested `/lib64/ld-linux-x86-64.so.2`, which points to the
diagnostic stub on the tested NixOS system. The local executable instead
referenced the development machine's glibc and library paths in `/nix/store`.
A valid AppImage container and correct checksum did not establish that either
payload was portable.

Both build paths now invoke `scripts/build-appimage.sh`. The image includes:

- A checksum-verified, pinned Type 2 AppImage runtime.
- An ELF loader, glibc, and compatibility libraries from the build system.
- Explicitly selected dlopen-only client libraries and their transitive imports.
- XKB keyboard layouts and X11 locale/compose tables, with explicit resource paths.
- Relative library search paths instead of development-store RPATHs.

`scripts/AppRun` launches via the bundled loader, or a newer real host glibc
identified from the running shell's process mappings. This avoids the NixOS stub
and permits newer host graphics drivers to use their required glibc symbols.
Library paths are passed to the loader instead of exported to host subprocesses.
GPU drivers remain host-provided. The target is x86-64 graphical Linux desktops;
other CPU architectures, arbitrary kernels, and incompatible GPU stacks are not
implied by the AppImage format.

## Validation

- Formatting, Clippy, ShellCheck, actionlint, and 33 Rust tests passed locally.
- `check-appimage.sh` resolves the executable and bundled libraries with the
  development library path removed, checks relocatable imports/RPATHs, and
  rejects the old CI artifact because its loader is missing.
- `smoke-appimage.sh` opened a mapped window on Xvfb with temporary home/settings
  directories. It terminates only its own process group and retains logs.
- The local AppImage launched directly in COSMIC/Wayland outside devenv, without
  an FHS wrapper. The protocol trace showed attached buffers and committed
  frames; the probe was stopped intentionally after ten seconds.
- CI now runs the same packaging checks and opens the packaged application on a
  headless display before uploading it. Cross-distribution coverage must be
  reported from the actual artifacts tested, not inferred from file format.
- The first Ubuntu-built candidate passed Xvfb but exposed a missing XKB root
  when started on NixOS/Wayland. The packager now requires and bundles XKB and
  compose data, and the checker rejects an image missing either resource.
- The Ubuntu payload rendered Wayland frames on NixOS after adding these
  resources. Real libxkbcommon calls loaded the packaged context and compose
  tables for English, Spanish, Russian, and Traditional Chinese. Compose includes
  use the relocatable `%S` prefix instead of build-store or system paths.

Packaging and validation tools are declared in `devenv.nix`, including ELF tools,
shell/workflow checks, the dependency auditor, Python, Xvfb, and xwininfo. The
application continues to use host graphics drivers.

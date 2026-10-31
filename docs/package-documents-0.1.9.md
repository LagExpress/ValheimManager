# Package documents and local build validation — 2026-10-30

Application version: **0.1.9**. These are local builds, not published release assets.

## Behavior

- Generated README text contains the package title, summary, and version, followed
  by user Markdown and independently optional mod/configuration lists. It contains
  no promotional application/store text or installation boilerplate. Actual mod
  names/authors and canonical package-page URLs remain.
- Build & Export previews README and changelog documents as rendered Markdown or
  source. The copy action copies the complete generated document. Both preview
  and ZIP generation use the same document generators.
- Changelog notes compare mod versions and mod/configuration selections against
  the previous successful version. They do not compare configuration file bytes;
  users can describe those changes manually. Current notes can be edited or
  regenerated. Successful same-version builds replace one history entry.
- README choices, custom Markdown, notes, and history persist per saved modpack.
  Failed builds keep the draft and previous successful history/version. Older
  saved configurations without document settings remain readable.
- Markdown image loaders are disabled. Images referenced in user Markdown remain
  in the exported source but are not fetched by the preview.

## Checks performed

- Formatting and Clippy with warnings denied passed.
- Full Rust suite: **43 passed, 4 ignored**, no failures. The ignored tests are
  opt-in capture/native-window tools, not disabled regressions.
- The suite covers preview/ZIP equality, all four independent list-checkbox
  combinations, preserved manifest dependencies/config files when lists are
  hidden, Markdown field escaping, manual notes/regeneration, history deduplication,
  draft persistence after failure, and backward-compatible saved settings.
- The six-screen responsive matrix covers all four themes, English/Spanish, five
  logical sizes (640×480, 800×600, 980×656, 1280×720, 1920×1080), and scales 1/1.5/2.
  A separate document matrix covers both document tabs and rendered/source modes
  at the same sizes/scales, using 1,000-item fixtures. Existing list/log tests retain
  virtualization/cache and idle repaint assertions, including 2,000 activity logs.
- The document capture exporter ran explicitly. Captures are under
  `target/ui-previews/0.1.9/documents/`; representative captures in every theme,
  both languages, compact layouts, scaling, and scrolled tables were inspected.
  A missing heading glyph found during review was fixed by enabling egui's bundled
  monochrome emoji fallback; glyph coverage is tested.
- JavaScript reference syntax and `git diff --check` passed.
- Fresh RustSec audit: no reported vulnerabilities or unmaintained advisories;
  see [dependency review](dependency-audit-2026-10-30.md).

## Artifacts

Built with the existing devenv environment and shared packaging scripts:

| Path | Format | SHA-256 |
| --- | --- | --- |
| `dist/ValheimManager-0.1.9.AppImage` | x86-64 ELF AppImage, static PIE runtime + SquashFS | `8501acacb5f452a1ca50a611866563f8a754ff98a730e2180901030f075f20e6` |
| `dist/ValheimManager-0.1.9.exe` | x86-64 PE32+ Windows executable | `f37e47723f82bccd8c3aeaa638f39665954dfa6da6f933e12dec154cb9ea5bc0` |

`dist/SHA256SUMS-0.1.9.txt` was generated and both entries verified. The AppImage
packager's runtime/library/resource checks passed. The packaged app opened a mapped
window on NixOS in an isolated Xvfb display using temporary settings; logs are in
`target/appimage-smoke.v5uHqu/`.

The UI captures are offscreen rendering checks. This work did not repeat native
COSMIC/KDE glass verification or perform Windows desktop visual verification.
The EXE was cross-compiled with the declared MinGW toolchain. These local hashes
must not be presented as hashes of future CI/release assets.

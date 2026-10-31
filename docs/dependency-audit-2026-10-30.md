# Package-document dependency review — 2026-10-30

The package-document preview uses `egui_commonmark` 0.26.0, matching the existing
egui/eframe 0.37 UI. Its default features are disabled; only `pulldown_cmark` is
enabled. Image, HTTP, filesystem-image, SVG, and syntax-highlighting loaders are
not enabled. Rendering a document does not fetch its referenced images.

The lockfile adds four packages: `egui_commonmark` 0.26.0,
`egui_commonmark_backend` 0.26.0, `pulldown-cmark` 0.13.6, and `unicase` 2.10.0.
Other existing crate versions remain unchanged. A direct egui 0.37 declaration
enables its bundled monochrome emoji fonts for the generated section headings;
it resolves to the same egui instance as eframe.

`devenv shell -- cargo-audit audit` fetched the RustSec database and scanned 392
locked packages on 2026-10-30. It completed successfully with no reported
vulnerabilities or unmaintained-crate advisories. This result describes that
audit, not future advisory status. No new system tooling or runtime library was
needed outside the existing `devenv.nix` environment.

# Dependency review — 2026-10-29

Reviewed direct dependency versions against the crates.io API and project repositories, then checked the complete Cargo lockfile with `cargo-audit 0.22.1`. The RustSec database used commit `40f5be6fb12ed93fc591bf4ab23b7d793b3f2239` (updated 2026-10-28).

| Dependency | Previous lockfile | Updated lockfile | Reason |
| --- | --- | --- | --- |
| zip | 8.6.0 | 9.1.3 | Current stable release; removes RUSTSEC-2026-0367, 0368, and 0369. These advisories concern extraction, which this application does not perform. |
| sha2 | 0.10.9 | 0.11.1 | Current stable release; a known-vector regression test confirms SHA-256 state compatibility. |
| opener | 0.8.5 with `reveal` | 0.9.0 without `reveal` | Opens the containing output folder on a worker thread. Removes the optional zbus 5 dependency chain. |

The removed chain contained `zvariant`, `zvariant_derive`, and `zbus_names`, marked unmaintained by RUSTSEC-2026-0370, 0371, and 0372. Their functionality moved into zbus 6. The application does not need those crates for its “Open Output Folder” action. No advisory is suppressed.

The final audit reports **zero vulnerabilities and zero informational/maintenance warnings** across 388 locked packages. The native file-dialog crate `rfd` remains in use; it does not introduce this removed chain.

The current eframe/egui 0.37 stack, rfd 0.17.2, dirs 7, yaml_serde 0.10.7, and other direct dependencies were retained after review. An older release date alone is not evidence that a stable library is abandoned. The YAML implementation is the maintained `yaml_serde` fork, not the deprecated `serde_yaml` package.

Native effects use target-specific maintained Wayland bindings and Windows system bindings, aligned with versions already used by the GUI stack. Inter is bundled under its SIL Open Font License; its license is included beside the font. Image, pollster, and winit are explicitly declared as development dependencies for the offscreen and native-window validation tools.

## Reproducing the checks

```sh
devenv shell -- cargo fmt --all -- --check
devenv shell -- cargo clippy --all-targets -- -D warnings
devenv shell -- cargo test --all-targets
nix-shell -p cargo-audit --run 'cargo-audit audit'
```

Audit results describe the database and lockfile at this date; run the check again when dependencies change.

Sources: [RustSec ZIP advisory](https://rustsec.org/advisories/RUSTSEC-2026-0367.html), [zvariant maintenance advisory](https://rustsec.org/advisories/RUSTSEC-2026-0370.html), [crates.io](https://crates.io/).

# Release 0.1.3 — 2026-10-29

## Saved modpack configurations

The **Profile & Info** screen has a saved-configuration selector, **Save configuration**, and **Save as new**. The last saved configuration loads at startup. Each preset stores the modpack name, description, declared version, website, icon path, output folder, source manager/root/profile, mod selections, explicit dependency version overrides, configuration selections, and the automatic-increment preference.

Presets live in `modpacks.json` beside the existing application settings. They do not contain SFTP credentials. Writes use a temporary file and replacement; malformed or unsupported saved files are reported and are not silently replaced. New installed mod versions are retained unless an explicit dependency override was saved.

Both ZIP build actions use the same version-choice dialog. Users can keep the declared version, increment its patch number, or cancel. The **Always increment** checkbox is saved per modpack and is available in the metadata/build views and the dialog. Incrementing `1.2.9` produces `1.2.10`; prerelease/build suffixes are removed when incrementing. Invalid versions and patch overflow report errors.

Metadata is saved before the worker starts. An incremented version is committed to the preset only after ZIP creation succeeds. Failed or disconnected builds restore the previous version, and cancelling the dialog does not save or increment anything. A failure to persist the successful build's version is reported separately from ZIP creation success.

## Wrapped activity logs

Log messages now wrap to the available width, including long paths, fingerprints, and explicit newlines. The log remains capped at 2,000 entries. Text layout and row offsets are cached by filter, revision, width, font, scale, and UI context; only visible variable-height rows are painted. Resizing, filtering, and new log messages invalidate the relevant layout. Copying logs retains the original full text.

## Validation

- 27 Rust tests passed, including the existing 720-combination responsive matrix and idle-glass checks. The focused suite was rerun after the dialog contrast adjustment and passed.
- Regression tests exercise two saved modpacks, startup restoration, mod/config selections, dependency overrides versus installed updates, declared-version persistence, both version choices, repeated automatic builds, failure rollback, invalid inputs, unwritable settings, Escape cancellation, checked patch increments, and corrupt/future configuration files.
- Wrapping tests cover prose, a 512-character unbroken string, explicit newlines, cache reuse, resizing, and filtering.
- Formatting, Clippy with warnings denied, and Git whitespace checks pass. No new dependency was added.
- GPU captures under `target/ui-previews/0.1.3/` cover saved configurations, settled version dialogs, and wrapped logs in all four themes in English and Spanish at minimum window size.
- Native Windows/KDE visual verification remains outstanding as recorded in the [earlier UI validation](ui-validation-2026-10-29.md). The platform-effect adapter was not changed in this release.

The app version, Cargo lockfile, packaging scripts, CI artifact names, and README paths use **0.1.3** on branch `modernization`.

## Built artifacts

Both release builds completed successfully. The AppImage has a valid x86-64 Type 2 runtime and zstd SquashFS payload, and responds to `--appimage-version`. The Windows output is an x86-64 PE32+ executable.

| Artifact in `dist/` | SHA-256 |
| --- | --- |
| `ValheimModpackStudio-0.1.3-x86_64.AppImage` | `bbdef4ac672d8bc711dfaaa7e929a88d5976da1d13fc7c58bbed470d18528ab4` |
| `ValheimModpackStudio-0.1.3-x86_64.exe` | `c52f95c6a5e657c530f1fde6d33404ff59c20a5b9735f474ae9e608f9030eadb` |

The hashes are also saved in `dist/SHA256SUMS-0.1.3.txt`; both pass `sha256sum -c`. Versioning applies to the application and artifact filenames; no Git tag was created.

use semver::Version;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

type ProgressCallback<'a> = dyn Fn(f32, &str) + 'a;

use crate::profile::{ConfigCategory, ConfigItem, ModItem};

pub const EMBEDDED_ICON: &[u8] = include_bytes!("assets/icon.png");

#[derive(Debug, Serialize)]
pub struct ManifestJson {
    pub name: String,
    pub version_number: String,
    pub website_url: String,
    pub description: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModpackOptions {
    pub name: String,
    pub version: String,
    pub description: String,
    pub website_url: String,
    pub selected_mods: Vec<ModItem>,
    pub selected_configs: Vec<ConfigItem>,
    pub custom_icon: Option<PathBuf>,
    pub out_dir: PathBuf,
    pub dist_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub build_dir: PathBuf,
    pub zip_path: PathBuf,
    pub mod_count: usize,
    pub config_count: usize,
    pub zip_size_bytes: u64,
}

pub fn generate_readme(
    name: &str,
    version: &str,
    description: &str,
    mods: &[ModItem],
    configs: &[ConfigItem],
) -> String {
    let mut readme = String::new();
    readme.push_str(&format!("# {}\n\n", name));
    readme.push_str(&format!("> {}\n\n", description));
    readme.push_str(&format!("**Version:** `{}`  \n", version));
    readme.push_str("**Powered by:** Valheim r2modman / Thunderstore\n\n---\n\n");

    readme.push_str(&format!("## 📦 Included Mods ({})\n\n", mods.len()));
    readme.push_str("| Mod | Author | Version | Thunderstore Package |\n");
    readme.push_str("| --- | --- | --- | --- |\n");
    for m in mods {
        let url = if m.website_url.is_empty() {
            format!("https://thunderstore.io/c/valheim/p/{}/", m.name)
        } else {
            m.website_url.clone()
        };
        readme.push_str(&format!(
            "| **{}** | {} | `{}` | [`{}`]({}) |\n",
            m.display_name,
            if m.author_name.is_empty() {
                "-"
            } else {
                &m.author_name
            },
            m.current_version,
            m.name,
            url
        ));
    }

    readme.push_str("\n---\n\n");
    readme.push_str(&format!(
        "## ⚙️ Included Configurations ({})\n\n",
        configs.len()
    ));
    readme.push_str("This modpack includes curated, pre-tuned configuration files for optimal stability and gameplay:\n");
    for c in configs {
        readme.push_str(&format!("- `{}`\n", c.name));
    }
    let includes_sensitive = configs
        .iter()
        .any(|item| item.category == ConfigCategory::Sensitive);
    let includes_cache = configs
        .iter()
        .any(|item| item.category == ConfigCategory::Cache);
    readme.push_str(
        "\n> **Review before sharing:** The selected configuration files are included as listed. ",
    );
    if includes_sensitive {
        readme.push_str("Sensitive configuration files were selected. ");
    }
    if includes_cache {
        readme.push_str("Runtime cache or report files were selected. ");
    }
    readme.push_str("Check the generated config folder for private data.\n\n");

    readme.push_str("---\n\n## 🚀 Installation\n\n");
    readme.push_str("### Via r2modman / Thunderstore Mod Manager (Recommended)\n");
    readme.push_str("1. Open **r2modman** (or Thunderstore Mod Manager).\n");
    readme.push_str("2. Select **Valheim**.\n");
    readme.push_str("3. Create or select a profile.\n");
    readme.push_str(&format!(
        "4. Go to **Online**, search for **{}**, and click **Download with dependencies**.\n",
        name
    ));
    readme.push_str("5. All required mods and tailored configuration settings will be applied automatically!\n\n");
    readme.push_str("---\n*Generated with Valheim Modpack Studio.*\n");

    readme
}

pub fn build_modpack(
    opts: &ModpackOptions,
    on_progress: Option<&ProgressCallback<'_>>,
) -> Result<BuildResult, String> {
    let clean_name = opts
        .name
        .trim()
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
    if clean_name.is_empty() {
        return Err("Modpack name cannot be empty".to_string());
    }
    let version = opts.version.trim();
    Version::parse(version).map_err(|e| format!("Invalid modpack version '{}': {}", version, e))?;
    for item in &opts.selected_configs {
        if item.name.is_empty()
            || item.name == "."
            || item.name == ".."
            || item.name.chars().any(|c| matches!(c, '/' | '\\' | ':'))
            || Path::new(&item.name).components().count() != 1
        {
            return Err(format!("Invalid config item name: {}", item.name));
        }
    }

    if let Some(cb) = on_progress {
        cb(0.02, "Preparing build workspace...");
    }

    let build_folder = opts.out_dir.join(&clean_name);
    let zip_filename = format!("{}-{}.zip", clean_name, version);
    let zip_path = opts.dist_dir.join(&zip_filename);

    if build_folder.exists() {
        fs::remove_dir_all(&build_folder).map_err(|e| {
            format!(
                "Failed to clean build folder {}: {}",
                build_folder.display(),
                e
            )
        })?;
    }
    fs::create_dir_all(&build_folder)
        .map_err(|e| format!("Failed to create build folder: {}", e))?;
    fs::create_dir_all(&opts.dist_dir)
        .map_err(|e| format!("Failed to create dist folder: {}", e))?;

    // 1. Write manifest.json
    if let Some(cb) = on_progress {
        cb(0.06, "Creating manifest.json...");
    }
    let manifest = ManifestJson {
        name: clean_name.clone(),
        version_number: version.to_string(),
        website_url: opts.website_url.trim().to_string(),
        description: if opts.description.trim().is_empty() {
            "Valheim modpack with custom configurations".to_string()
        } else {
            opts.description.trim().to_string()
        },
        dependencies: opts
            .selected_mods
            .iter()
            .map(|m| m.dependency_string())
            .collect(),
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(build_folder.join("manifest.json"), manifest_bytes)
        .map_err(|e| format!("Failed to write manifest.json: {}", e))?;

    // 2. Write README.md
    if let Some(cb) = on_progress {
        cb(0.12, "Generating README.md with mod list...");
    }
    let readme_content = generate_readme(
        &clean_name,
        opts.version.trim(),
        &manifest.description,
        &opts.selected_mods,
        &opts.selected_configs,
    );
    fs::write(build_folder.join("README.md"), readme_content)
        .map_err(|e| format!("Failed to write README.md: {}", e))?;

    // 3. Write icon.png (256x256)
    if let Some(cb) = on_progress {
        cb(0.18, "Processing modpack icon...");
    }
    let icon_dest = build_folder.join("icon.png");
    if let Some(ref custom_path) = opts.custom_icon {
        if custom_path.exists() {
            fs::copy(custom_path, &icon_dest)
                .map_err(|e| format!("Failed to copy custom icon: {}", e))?;
        } else {
            fs::write(&icon_dest, EMBEDDED_ICON)
                .map_err(|e| format!("Failed to write embedded icon: {}", e))?;
        }
    } else {
        fs::write(&icon_dest, EMBEDDED_ICON)
            .map_err(|e| format!("Failed to write embedded icon: {}", e))?;
    }

    // 4. Copy config files into build_folder/config
    let config_dest = build_folder.join("config");
    std::fs::create_dir_all(&config_dest)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let total_configs = opts.selected_configs.len();
    for (idx, item) in opts.selected_configs.iter().enumerate() {
        if let Some(cb) = on_progress {
            let frac = 0.20 + 0.38 * ((idx + 1) as f32 / total_configs.max(1) as f32);
            cb(
                frac,
                &format!(
                    "Copying config ({}/{}): {}",
                    idx + 1,
                    total_configs,
                    item.name
                ),
            );
        }
        let target = config_dest.join(&item.name);
        if item.is_dir {
            copy_dir_filtered(&item.path, &target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create config directory {}: {}",
                        parent.display(),
                        e
                    )
                })?;
            }
            let metadata = fs::symlink_metadata(&item.path)
                .map_err(|e| format!("Could not inspect config {}: {}", item.path.display(), e))?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!(
                    "Config source is not a regular file: {}",
                    item.path.display()
                ));
            }
            fs::copy(&item.path, &target)
                .map_err(|e| format!("Failed to copy config {}: {}", item.path.display(), e))?;
        }
    }

    // 5. Create ZIP archive with pure Rust `zip` crate
    if let Some(cb) = on_progress {
        cb(0.60, "Creating ZIP archive...");
    }
    create_zip_archive(&build_folder, &zip_path, on_progress)?;

    if let Some(cb) = on_progress {
        cb(1.0, "Modpack built successfully!");
    }

    let zip_size_bytes = std::fs::metadata(&zip_path)
        .map_err(|e| format!("Could not inspect archive {}: {}", zip_path.display(), e))?
        .len();

    Ok(BuildResult {
        build_dir: build_folder,
        zip_path,
        mod_count: opts.selected_mods.len(),
        config_count: opts.selected_configs.len(),
        zip_size_bytes,
    })
}

fn copy_dir_filtered(src: &Path, dst: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(src).map_err(|e| {
        format!(
            "Could not inspect config directory {}: {}",
            src.display(),
            e
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "Config source is not a regular directory: {}",
            src.display()
        ));
    }
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;

    let entries = fs::read_dir(src)
        .map_err(|e| format!("Could not read directory {}: {}", src.display(), e))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("Could not read an entry in {}: {}", src.display(), e))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Filter out cache and report subdirectories/files
        if name.eq_ignore_ascii_case("MultiplayerClientCache")
            || name.eq_ignore_ascii_case("Reports")
            || name.eq_ignore_ascii_case("ClusterCache")
            || name.to_ascii_lowercase().ends_with(".records")
            || name.to_ascii_lowercase().ends_with(".log")
        {
            continue;
        }

        let dest_child = dst.join(&name);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| format!("Could not inspect {}: {}", path.display(), e))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("Refusing to package symlink: {}", path.display()));
        } else if metadata.is_dir() {
            copy_dir_filtered(&path, &dest_child)?;
        } else if metadata.is_file() {
            fs::copy(&path, &dest_child)
                .map_err(|e| format!("Failed to copy {}: {}", path.display(), e))?;
        }
    }

    // If destination is empty directory, remove it
    let mut entries = fs::read_dir(dst)
        .map_err(|e| format!("Could not inspect destination {}: {}", dst.display(), e))?;
    if entries.next().is_none() {
        fs::remove_dir(dst)
            .map_err(|e| format!("Could not remove empty directory {}: {}", dst.display(), e))?;
    }

    Ok(())
}

fn create_zip_archive(
    src_dir: &Path,
    out_zip: &Path,
    on_progress: Option<&ProgressCallback<'_>>,
) -> Result<(), String> {
    let file_name = out_zip
        .file_name()
        .ok_or_else(|| format!("Invalid output archive path: {}", out_zip.display()))?
        .to_string_lossy();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary_zip = out_zip.with_file_name(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nonce
    ));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_zip)
        .map_err(|e| format!("Failed to create zip file {}: {}", out_zip.display(), e))?;
    let result = (|| {
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        let total = WalkDir::new(src_dir)
            .into_iter()
            .try_fold(0usize, |count, entry| {
                entry.map(|entry| count + usize::from(entry.path() != src_dir))
            })
            .map_err(|e| format!("Could not scan build folder {}: {}", src_dir.display(), e))?;

        for (idx, entry) in WalkDir::new(src_dir)
            .into_iter()
            // Retain traversal errors so a changed/unreadable directory cannot
            // silently publish an incomplete archive after the counting pass.
            .filter(|entry| match entry {
                Ok(entry) => entry.path() != src_dir,
                Err(_) => true,
            })
            .enumerate()
        {
            let entry = entry
                .map_err(|e| format!("Could not scan build folder {}: {}", src_dir.display(), e))?;
            let path = entry.path();
            let relative_path = path
                .strip_prefix(src_dir)
                .map_err(|e| format!("Strip prefix failed: {}", e))?;
            let name = zip_entry_name(relative_path)?;

            if let Some(cb) = on_progress {
                let frac = 0.60 + 0.38 * ((idx + 1) as f32 / total.max(1) as f32);
                cb(frac, &format!("Compressing: {}", name));
            }

            let metadata = fs::symlink_metadata(path).map_err(|e| {
                format!("Could not inspect archive input {}: {}", path.display(), e)
            })?;
            if metadata.file_type().is_symlink() {
                return Err(format!("Refusing to package symlink: {}", path.display()));
            } else if metadata.is_dir() {
                zip.add_directory(&name, options.unix_permissions(0o755))
                    .map_err(|e| format!("Zip add_directory failed: {}", e))?;
            } else if metadata.is_file() {
                zip.start_file(&name, options)
                    .map_err(|e| format!("Zip start_file failed: {}", e))?;
                let mut source = File::open(path)
                    .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;
                io::copy(&mut source, &mut zip)
                    .map_err(|e| format!("Could not stream {} into zip: {}", path.display(), e))?;
            }
        }

        zip.finish()
            .map_err(|e| format!("Failed to finalize zip file: {}", e))?;

        let existing = match fs::symlink_metadata(out_zip) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(format!(
                    "Could not inspect existing archive {}: {}",
                    out_zip.display(),
                    error
                ));
            }
        };
        if existing
            .as_ref()
            .is_some_and(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(format!(
                "Refusing to replace non-regular archive path: {}",
                out_zip.display()
            ));
        }

        #[cfg(windows)]
        if existing.is_some() {
            let backup = out_zip.with_file_name(format!(
                ".{}.{}.{}.bak",
                file_name,
                std::process::id(),
                nonce
            ));
            fs::rename(out_zip, &backup).map_err(|e| {
                format!(
                    "Could not preserve existing archive {}: {}",
                    out_zip.display(),
                    e
                )
            })?;
            if let Err(error) = fs::rename(&temporary_zip, out_zip) {
                return match fs::rename(&backup, out_zip) {
                    Ok(()) => Err(format!(
                        "Could not publish archive {}: {}",
                        out_zip.display(),
                        error
                    )),
                    Err(restore_error) => Err(format!(
                        "Could not publish archive {}: {}; the previous archive remains at {} because restoring it failed: {}",
                        out_zip.display(),
                        error,
                        backup.display(),
                        restore_error
                    )),
                };
            }
            fs::remove_file(&backup).map_err(|e| {
                format!(
                    "Archive was published, but could not remove backup {}: {}",
                    backup.display(),
                    e
                )
            })?;
        } else {
            fs::rename(&temporary_zip, out_zip)
                .map_err(|e| format!("Could not publish archive {}: {}", out_zip.display(), e))?;
        }

        #[cfg(not(windows))]
        fs::rename(&temporary_zip, out_zip)
            .map_err(|e| format!("Could not publish archive {}: {}", out_zip.display(), e))?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_zip);
    }
    result
}

fn zip_entry_name(path: &Path) -> Result<String, String> {
    let mut components = Vec::new();
    for component in path.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(format!("Unsafe path in build folder: {}", path.display()));
        };
        let component = component.to_string_lossy();
        if component.is_empty() || component.chars().any(|c| matches!(c, '\\' | ':')) {
            return Err(format!("Unsafe path in build folder: {}", path.display()));
        }
        components.push(component.into_owned());
    }
    if components.is_empty() {
        return Err(format!("Invalid path in build folder: {}", path.display()));
    }
    Ok(components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn archive_upgrade_preserves_contents_and_existing_output_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fs::create_dir_all(source.join("config")).unwrap();
        fs::write(source.join("config/game.cfg"), b"enabled = true\n").unwrap();
        let output = dir.path().join("pack.zip");
        create_zip_archive(&source, &output, None).unwrap();
        let original = fs::read(&output).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
        let mut text = String::new();
        archive
            .by_name("config/game.cfg")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "enabled = true\n");
        drop(archive);
        assert!(create_zip_archive(&dir.path().join("missing"), &output, None).is_err());
        assert_eq!(fs::read(&output).unwrap(), original);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(source.join("config/game.cfg"), source.join("linked.cfg"))
                .unwrap();
            assert!(create_zip_archive(&source, &output, None).is_err());
            assert_eq!(fs::read(&output).unwrap(), original);
        }
    }

    #[test]
    fn rejects_archive_traversal_and_foreign_separators() {
        for name in [
            "../secret",
            "/secret",
            "config/../../secret",
            "config/a\\b",
            "config/C:secret",
        ] {
            assert!(zip_entry_name(Path::new(name)).is_err(), "{name}");
        }
        assert_eq!(
            zip_entry_name(Path::new("config/game.cfg")).unwrap(),
            "config/game.cfg"
        );
    }

    #[test]
    fn a_directory_disappearing_during_compression_preserves_the_previous_archive() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let nested = source.join("config");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("game.cfg"), b"enabled = true\n").unwrap();
        let output = dir.path().join("pack.zip");
        fs::write(&output, b"previous archive").unwrap();
        let removed = std::sync::atomic::AtomicBool::new(false);
        let callback = |_: f32, _: &str| {
            if !removed.swap(true, std::sync::atomic::Ordering::SeqCst) {
                fs::remove_dir_all(&nested).unwrap();
            }
        };
        assert!(create_zip_archive(&source, &output, Some(&callback)).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"previous archive");
    }
}

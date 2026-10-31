use semver::Version;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

type ProgressCallback<'a> = dyn Fn(f32, &str) + 'a;

use crate::package_docs::{PackageDocumentation, PackageSnapshot, ReadmeOptions, ReleaseEntry};
use crate::profile::{ConfigItem, ModItem};

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
    pub documentation: PackageDocumentation,
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
    pub release: Box<ReleaseEntry>,
}

pub fn clean_package_name(name: &str) -> String {
    name.trim()
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
}

pub(crate) fn package_description(description: &str) -> &str {
    if description.trim().is_empty() {
        "Modpack with custom configurations"
    } else {
        description.trim()
    }
}

pub(crate) fn markdown_text(text: &str) -> String {
    let mut escaped = String::new();
    for c in text.chars() {
        match c {
            '\r' | '\n' => escaped.push(' '),
            '\\' | '|' | '*' | '_' | '[' | ']' | '<' | '>' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

pub(crate) fn inline_code(text: &str) -> String {
    let text = text.replace(['\r', '\n'], " ");
    let delimiter = "`".repeat(text.split(|c| c != '`').map(str::len).max().unwrap_or(0) + 1);
    if text.contains('`') || text.starts_with(' ') || text.ends_with(' ') {
        format!("{delimiter} {text} {delimiter}")
    } else {
        format!("{delimiter}{text}{delimiter}")
    }
}

fn url_segment(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

fn thunderstore_url(item: &ModItem) -> String {
    let package = item
        .name
        .split_once('-')
        .or_else(|| {
            (!item.author_name.trim().is_empty())
                .then_some((item.author_name.trim(), item.name.as_str()))
        })
        .or_else(|| {
            let path = item
                .website_url
                .strip_prefix("https://thunderstore.io/c/valheim/p/")
                .or_else(|| {
                    item.website_url
                        .strip_prefix("https://valheim.thunderstore.io/package/")
                })?;
            let mut parts = path.split('/');
            Some((parts.next()?, parts.next()?))
        });
    match package {
        Some((namespace, name)) => format!(
            "https://thunderstore.io/c/valheim/p/{}/{}/",
            url_segment(namespace),
            url_segment(name)
        ),
        None => format!(
            "https://thunderstore.io/c/valheim/?q={}",
            url_segment(&item.name)
        ),
    }
}

pub fn generate_readme<'a>(
    name: &str,
    version: &str,
    description: &str,
    options: &ReadmeOptions,
    mods: impl Iterator<Item = &'a ModItem> + Clone,
    configs: impl Iterator<Item = &'a ConfigItem> + Clone,
) -> String {
    let mut readme = String::new();
    readme.push_str(&format!("# {}\n\n", clean_package_name(name)));
    for line in package_description(description).lines() {
        readme.push_str(&format!("> {line}\n"));
    }
    readme.push_str(&format!("\n**Version:** {}\n", inline_code(version.trim())));
    if !options.additional_markdown.trim().is_empty()
        || options.include_mods
        || options.include_configs
    {
        readme.push_str("\n---\n\n");
    }
    if !options.additional_markdown.trim().is_empty() {
        readme.push_str(options.additional_markdown.trim_matches(['\r', '\n']));
        readme.push_str("\n\n");
    }

    if options.include_mods {
        readme.push_str(&format!(
            "## 📦 Included Mods ({})\n\n",
            mods.clone().count()
        ));
        readme.push_str("| Mod | Author | Version | Package |\n");
        readme.push_str("| --- | --- | --- | --- |\n");
        for m in mods {
            readme.push_str(&format!(
                "| **{}** | {} | {} | [{}]({}) |\n",
                markdown_text(&m.display_name),
                markdown_text(if m.author_name.is_empty() {
                    "-"
                } else {
                    &m.author_name
                }),
                inline_code(m.current_version.trim()).replace('|', "\\|"),
                inline_code(&m.name).replace('|', "\\|"),
                thunderstore_url(m)
            ));
        }
    }
    if options.include_configs {
        if options.include_mods {
            readme.push_str("\n---\n\n");
        }
        readme.push_str(&format!(
            "## ⚙️ Included Configurations ({})\n\n",
            configs.clone().count()
        ));
        for c in configs {
            readme.push_str(&format!("- {}\n", inline_code(&c.name)));
        }
    }

    readme
}

pub fn build_modpack(
    opts: &ModpackOptions,
    on_progress: Option<&ProgressCallback<'_>>,
) -> Result<BuildResult, String> {
    let clean_name = clean_package_name(&opts.name);
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
        description: package_description(&opts.description).to_string(),
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
        cb(0.12, "Generating package documentation...");
    }
    let readme_content = generate_readme(
        &clean_name,
        opts.version.trim(),
        &manifest.description,
        &opts.documentation.readme,
        opts.selected_mods.iter(),
        opts.selected_configs.iter(),
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

    let snapshot = PackageSnapshot::new(
        &clean_name,
        &manifest.description,
        &opts.documentation.readme,
        opts.selected_mods.iter(),
        opts.selected_configs.iter(),
    );
    let release = opts.documentation.changelog.release(version, snapshot);
    fs::write(
        build_folder.join("CHANGELOG.md"),
        opts.documentation.changelog.document(&release),
    )
    .map_err(|error| format!("Failed to write CHANGELOG.md: {error}"))?;

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
        release: Box::new(release),
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
    use crate::profile::ConfigCategory;
    use std::io::Read;

    fn package_items() -> (Vec<ModItem>, Vec<ConfigItem>) {
        (
            vec![ModItem {
                name: "Team-Journey".into(),
                author_name: "Team".into(),
                display_name: "Journey".into(),
                description: String::new(),
                website_url: "https://example.com/mod-source".into(),
                enabled: true,
                original_version: "1.2.0".into(),
                current_version: "1.2.3".into(),
                selected: true,
            }],
            vec![ConfigItem {
                name: "journey.cfg".into(),
                path: PathBuf::new(),
                is_dir: false,
                category: ConfigCategory::Safe,
                size: 0,
                selected: true,
            }],
        )
    }

    #[test]
    fn readme_matches_the_requested_structure_and_has_no_generated_branding() {
        let (mods, configs) = package_items();
        let options = ReadmeOptions {
            additional_markdown: "## Gameplay\n\nBuild together.  \nExplore freely.\n".into(),
            ..Default::default()
        };
        let text = generate_readme(
            " North Casual ",
            "1.0.1",
            "A casual adventure",
            &options,
            mods.iter(),
            configs.iter(),
        );
        assert!(text.starts_with(
            "# North_Casual\n\n> A casual adventure\n\n**Version:** `1.0.1`\n\n---\n\n## Gameplay"
        ));
        assert!(text.contains("Build together.  \nExplore freely.\n\n## 📦 Included Mods (1)"));
        assert!(text.contains("| **Journey** | Team | `1.2.3` | [`Team-Journey`](https://thunderstore.io/c/valheim/p/Team/Journey/) |"));
        assert!(text.ends_with("## ⚙️ Included Configurations (1)\n\n- `journey.cfg`\n"));
        for forbidden in [
            "Thunderstore",
            "r2modman",
            "ValheimManager",
            "Valheim Modpack Studio",
            "Installation",
            "Generated by",
            "Powered by",
        ] {
            assert!(!text.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn readme_lists_are_independently_optional_and_escape_package_fields() {
        let (mut mods, mut configs) = package_items();
        mods[0].display_name = "A|B*[test]".into();
        configs[0].name = "a`b.cfg".into();
        for include_mods in [false, true] {
            for include_configs in [false, true] {
                let options = ReadmeOptions {
                    include_mods,
                    include_configs,
                    ..Default::default()
                };
                let text =
                    generate_readme("North", "1.0.0", "", &options, mods.iter(), configs.iter());
                assert_eq!(text.contains("Included Mods"), include_mods);
                assert_eq!(text.contains("Included Configurations"), include_configs);
                if include_mods {
                    assert!(text.contains(r"**A\|B\*\[test\]**"));
                }
                if include_configs {
                    assert!(text.contains("- `` a`b.cfg ``"));
                }
                if !include_mods && !include_configs {
                    assert!(!text.contains("---"));
                }
            }
        }
    }

    #[test]
    fn hidden_readme_lists_do_not_remove_dependencies_or_config_files_from_the_zip() {
        let directory = tempfile::tempdir().unwrap();
        let (mods, mut configs) = package_items();
        configs[0].path = directory.path().join("journey.cfg");
        fs::write(&configs[0].path, "enabled=true").unwrap();
        let options = ModpackOptions {
            name: "North".into(),
            version: "1.0.0".into(),
            description: "Explore".into(),
            website_url: String::new(),
            custom_icon: None,
            selected_mods: mods,
            selected_configs: configs,
            documentation: PackageDocumentation {
                readme: ReadmeOptions {
                    include_mods: false,
                    include_configs: false,
                    additional_markdown: "## Custom guide\n\nHave fun.".into(),
                },
                ..Default::default()
            },
            out_dir: directory.path().join("build"),
            dist_dir: directory.path().join("dist"),
        };
        let result = build_modpack(&options, None).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&result.zip_path).unwrap()).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
        assert_eq!(manifest["dependencies"][0], "Team-Journey-1.2.3");
        for (file, expected) in [
            (
                "README.md",
                generate_readme(
                    "North",
                    "1.0.0",
                    "Explore",
                    &options.documentation.readme,
                    options.selected_mods.iter(),
                    options.selected_configs.iter(),
                ),
            ),
            (
                "CHANGELOG.md",
                options.documentation.changelog.document(&result.release),
            ),
            ("config/journey.cfg", "enabled=true".into()),
        ] {
            let mut text = String::new();
            archive
                .by_name(file)
                .unwrap()
                .read_to_string(&mut text)
                .unwrap();
            assert_eq!(text, expected, "{file}");
        }
    }

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

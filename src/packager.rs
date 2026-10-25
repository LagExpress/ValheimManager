use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::Serialize;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

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
            if m.author_name.is_empty() { "-" } else { &m.author_name },
            m.current_version,
            m.name,
            url
        ));
    }

    readme.push_str("\n---\n\n");
    readme.push_str(&format!("## ⚙️ Included Configurations ({})\n\n", configs.len()));
    readme.push_str("This modpack includes curated, pre-tuned configuration files for optimal stability and gameplay:\n");
    for c in configs {
        readme.push_str(&format!("- `{}`\n", c.name));
    }
    readme.push_str("\n> **Security Notice:** Personal player IDs, runtime caches, and private server connection passwords have been excluded.\n\n");

    readme.push_str("---\n\n## 🚀 Installation\n\n");
    readme.push_str("### Via r2modman / Thunderstore Mod Manager (Recommended)\n");
    readme.push_str("1. Open **r2modman** (or Thunderstore Mod Manager).\n");
    readme.push_str("2. Select **Valheim**.\n");
    readme.push_str("3. Create or select a profile.\n");
    readme.push_str(&format!("4. Go to **Online**, search for **{}**, and click **Download with dependencies**.\n", name));
    readme.push_str("5. All required mods and tailored configuration settings will be applied automatically!\n\n");
    readme.push_str("---\n*Generated with Valheim Modpack Studio.*\n");

    readme
}

pub fn build_modpack(opts: &ModpackOptions) -> Result<BuildResult, String> {
    let clean_name = opts.name.trim().replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
    if clean_name.is_empty() {
        return Err("Modpack name cannot be empty".to_string());
    }

    let build_folder = opts.out_dir.join(&clean_name);
    let zip_filename = format!("{}-{}.zip", clean_name, opts.version.trim());
    let zip_path = opts.dist_dir.join(&zip_filename);

    if build_folder.exists() {
        let _ = std::fs::remove_dir_all(&build_folder);
    }
    std::fs::create_dir_all(&build_folder)
        .map_err(|e| format!("Failed to create build folder: {}", e))?;
    std::fs::create_dir_all(&opts.dist_dir)
        .map_err(|e| format!("Failed to create dist folder: {}", e))?;

    // 1. Write manifest.json
    let manifest = ManifestJson {
        name: clean_name.clone(),
        version_number: opts.version.trim().to_string(),
        website_url: opts.website_url.trim().to_string(),
        description: if opts.description.trim().is_empty() {
            "Valheim modpack with custom configurations".to_string()
        } else {
            opts.description.trim().to_string()
        },
        dependencies: opts.selected_mods.iter().map(|m| m.dependency_string()).collect(),
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    std::fs::write(build_folder.join("manifest.json"), manifest_bytes)
        .map_err(|e| format!("Failed to write manifest.json: {}", e))?;

    // 2. Write README.md
    let readme_content = generate_readme(
        &clean_name,
        opts.version.trim(),
        &manifest.description,
        &opts.selected_mods,
        &opts.selected_configs,
    );
    std::fs::write(build_folder.join("README.md"), readme_content)
        .map_err(|e| format!("Failed to write README.md: {}", e))?;

    // 3. Write icon.png (256x256)
    let icon_dest = build_folder.join("icon.png");
    if let Some(ref custom_path) = opts.custom_icon {
        if custom_path.exists() {
            std::fs::copy(custom_path, &icon_dest)
                .map_err(|e| format!("Failed to copy custom icon: {}", e))?;
        } else {
            std::fs::write(&icon_dest, EMBEDDED_ICON)
                .map_err(|e| format!("Failed to write embedded icon: {}", e))?;
        }
    } else {
        std::fs::write(&icon_dest, EMBEDDED_ICON)
            .map_err(|e| format!("Failed to write embedded icon: {}", e))?;
    }

    // 4. Copy config files into build_folder/config
    let config_dest = build_folder.join("config");
    std::fs::create_dir_all(&config_dest)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    for item in &opts.selected_configs {
        let target = config_dest.join(&item.name);
        if item.is_dir {
            copy_dir_filtered(&item.path, &target)?;
        } else {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(&item.path, &target);
        }
    }

    // 5. Create ZIP archive with pure Rust `zip` crate
    create_zip_archive(&build_folder, &zip_path)?;

    let zip_size_bytes = std::fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);

    Ok(BuildResult {
        build_dir: build_folder,
        zip_path,
        mod_count: opts.selected_mods.len(),
        config_count: opts.selected_configs.len(),
        zip_size_bytes,
    })
}

fn copy_dir_filtered(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst).map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;

    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Filter out cache and report subdirectories/files
            if name.eq_ignore_ascii_case("MultiplayerClientCache")
                || name.eq_ignore_ascii_case("Reports")
                || name.eq_ignore_ascii_case("ClusterCache")
                || name.ends_with(".records")
                || name.ends_with(".log")
            {
                continue;
            }

            let dest_child = dst.join(&name);
            if path.is_dir() {
                copy_dir_filtered(&path, &dest_child)?;
            } else {
                let _ = std::fs::copy(&path, &dest_child);
            }
        }
    }

    // If destination is empty directory, remove it
    if let Ok(mut entries) = std::fs::read_dir(dst) {
        if entries.next().is_none() {
            let _ = std::fs::remove_dir(dst);
        }
    }

    Ok(())
}

fn create_zip_archive(src_dir: &Path, out_zip: &Path) -> Result<(), String> {
    let file = File::create(out_zip)
        .map_err(|e| format!("Failed to create zip file {}: {}", out_zip.display(), e))?;
    let mut zip = ZipWriter::new(file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();

    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == src_dir {
            continue;
        }

        let relative_path = path
            .strip_prefix(src_dir)
            .map_err(|e| format!("Strip prefix failed: {}", e))?;

        let name = relative_path
            .to_string_lossy()
            .replace('\\', "/"); // Ensure unix path separators inside zip

        if path.is_dir() {
            zip.add_directory(&name, options)
                .map_err(|e| format!("Zip add_directory failed: {}", e))?;
        } else {
            zip.start_file(&name, options)
                .map_err(|e| format!("Zip start_file failed: {}", e))?;
            let mut f = File::open(path)
                .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;
            buffer.clear();
            f.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
            zip.write_all(&buffer)
                .map_err(|e| format!("Zip write failed: {}", e))?;
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip file: {}", e))?;

    Ok(())
}

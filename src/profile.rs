use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModManager {
    R2Modman,
    Gale,
    Custom,
}

impl ModManager {
    pub fn label(&self) -> &'static str {
        match self {
            ModManager::R2Modman => "r2modman",
            ModManager::Gale => "Gale",
            ModManager::Custom => "Custom Path",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawVersionNumber {
    #[serde(default)]
    pub major: u64,
    #[serde(default)]
    pub minor: u64,
    #[serde(default)]
    pub patch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawModEntry {
    #[serde(rename = "manifestVersion", default)]
    pub manifest_version: u32,
    pub name: String,
    #[serde(rename = "authorName", default)]
    pub author_name: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "websiteUrl", default)]
    pub website_url: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(rename = "versionNumber", default)]
    pub version_number: Option<RawVersionNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaleExportMod {
    pub name: String,
    #[serde(default)]
    pub version: Option<RawVersionNumber>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaleExportProfile {
    #[serde(rename = "profileName", default)]
    pub profile_name: String,
    #[serde(default)]
    pub mods: Vec<GaleExportMod>,
}

#[derive(Debug, Clone)]
pub struct ModItem {
    pub name: String,
    pub author_name: String,
    pub display_name: String,
    pub description: String,
    pub website_url: String,
    pub enabled: bool,
    pub original_version: String,
    pub current_version: String,
    pub selected: bool,
}

impl ModItem {
    pub fn dependency_string(&self) -> String {
        format!("{}-{}", self.name, self.current_version.trim())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCategory {
    Safe,
    Sensitive,
    Cache,
}

impl ConfigCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ConfigCategory::Safe => "SAFE",
            ConfigCategory::Sensitive => "SENSITIVE / DAT",
            ConfigCategory::Cache => "CACHE / REPORT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub category: ConfigCategory,
    pub size: u64,
    pub selected: bool,
}

pub fn get_r2modman_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // Linux standard paths
    if let Some(cfg) = dirs::config_dir() {
        roots.push(cfg.join("r2modmanPlus-local").join("Valheim").join("profiles"));
        roots.push(cfg.join("r2modman").join("Valheim").join("profiles"));
    }

    // Linux Flatpak path
    if let Some(home) = dirs::home_dir() {
        roots.push(
            home.join(".var")
                .join("app")
                .join("com.github.ebkr.r2modman")
                .join("config")
                .join("r2modmanPlus-local")
                .join("Valheim")
                .join("profiles"),
        );
    }

    // Windows standard AppData / Roaming
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("r2modmanPlus-local").join("Valheim").join("profiles"));
    }

    roots.into_iter().filter(|p| p.exists() && p.is_dir()).collect()
}

pub fn get_gale_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // Linux Gale paths
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("com.kesomannen.gale").join("valheim").join("profiles"));
        roots.push(data.join("gale").join("valheim").join("profiles"));
    }

    if let Some(home) = dirs::home_dir() {
        roots.push(
            home.join(".var")
                .join("app")
                .join("com.kesomannen.gale")
                .join("data")
                .join("com.kesomannen.gale")
                .join("valheim")
                .join("profiles"),
        );
    }

    // Windows Gale paths (AppData/Local or AppData/Roaming)
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("com.kesomannen.gale").join("valheim").join("profiles"));
    }
    if let Some(cfg) = dirs::config_dir() {
        roots.push(cfg.join("com.kesomannen.gale").join("valheim").join("profiles"));
    }

    roots.into_iter().filter(|p| p.exists() && p.is_dir()).collect()
}

#[allow(dead_code)]
pub fn get_default_profile_roots() -> Vec<PathBuf> {
    let r2 = get_r2modman_roots();
    if !r2.is_empty() {
        return r2;
    }
    let gale = get_gale_roots();
    if !gale.is_empty() {
        return gale;
    }
    Vec::new()
}

pub fn list_profiles_in_dir(dir: &Path) -> Vec<String> {
    let mut profiles = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && (path.join("mods.yml").exists() || path.join("export.r2x").exists()) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    profiles.push(name.to_string());
                }
            }
        }
    }
    profiles.sort();
    profiles
}

pub fn load_mods_from_profile(profile_dir: &Path) -> Result<Vec<ModItem>, String> {
    let mods_file = profile_dir.join("mods.yml");
    let export_file = profile_dir.join("export.r2x");

    if mods_file.exists() {
        // r2modman mods.yml format
        let content = std::fs::read_to_string(&mods_file)
            .map_err(|e| format!("Failed to read mods.yml: {}", e))?;

        let raw_entries: Vec<RawModEntry> = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse mods.yml: {}", e))?;

        let mut mods = Vec::new();
        for raw in raw_entries {
            let ver = if let Some(vn) = raw.version_number {
                format!("{}.{}.{}", vn.major, vn.minor, vn.patch)
            } else {
                "1.0.0".to_string()
            };

            let display = if raw.display_name.is_empty() {
                raw.name.clone()
            } else {
                raw.display_name
            };

            mods.push(ModItem {
                name: raw.name,
                author_name: raw.author_name,
                display_name: display,
                description: raw.description,
                website_url: raw.website_url,
                enabled: raw.enabled,
                original_version: ver.clone(),
                current_version: ver,
                selected: raw.enabled,
            });
        }
        return Ok(mods);
    }

    if export_file.exists() {
        // Gale export.r2x format
        let content = std::fs::read_to_string(&export_file)
            .map_err(|e| format!("Failed to read export.r2x: {}", e))?;

        let export_data: GaleExportProfile = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse export.r2x: {}", e))?;

        let mut mods = Vec::new();
        for m in export_data.mods {
            let ver = if let Some(vn) = m.version {
                format!("{}.{}.{}", vn.major, vn.minor, vn.patch)
            } else {
                "1.0.0".to_string()
            };

            let (author, display) = if let Some((a, d)) = m.name.split_once('-') {
                (a.to_string(), d.to_string())
            } else {
                ("-".to_string(), m.name.clone())
            };

            mods.push(ModItem {
                name: m.name.clone(),
                author_name: author,
                display_name: display,
                description: String::new(),
                website_url: format!("https://thunderstore.io/c/valheim/p/{}/", m.name),
                enabled: m.enabled,
                original_version: ver.clone(),
                current_version: ver,
                selected: m.enabled,
            });
        }
        return Ok(mods);
    }

    Err(format!(
        "No valid profile manifest (mods.yml or export.r2x) found in {}",
        profile_dir.display()
    ))
}

pub fn scan_profile_configs(profile_dir: &Path) -> Vec<ConfigItem> {
    let config_dir = profile_dir.join("BepInEx").join("config");
    let mut items = Vec::new();

    if !config_dir.exists() {
        return items;
    }

    let sensitive_re = Regex::new(r"(?i)quickconnect|player_\d+|credential|secret|token|\.dat$").unwrap();
    let cache_re = Regex::new(r"(?i)MultiplayerClientCache|Reports|ClusterCache|\.records$|\.log$").unwrap();

    if let Ok(entries) = std::fs::read_dir(&config_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_dir = path.is_dir();
            let name = entry.file_name().to_string_lossy().to_string();

            let size = if is_dir {
                0
            } else {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            };

            let category = if sensitive_re.is_match(&name) {
                ConfigCategory::Sensitive
            } else if cache_re.is_match(&name) || is_pure_cache_dir(&path) {
                ConfigCategory::Cache
            } else {
                ConfigCategory::Safe
            };

            let selected = category == ConfigCategory::Safe;

            items.push(ConfigItem {
                name,
                path,
                is_dir,
                category,
                size,
                selected,
            });
        }
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}

fn is_pure_cache_dir(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let cache_re = Regex::new(r"(?i)MultiplayerClientCache|Reports|ClusterCache|\.records$|\.log$").unwrap();
    if let Ok(children) = std::fs::read_dir(dir) {
        let list: Vec<_> = children.flatten().collect();
        if list.is_empty() {
            return false;
        }
        return list.iter().all(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            cache_re.is_match(&name)
        });
    }
    false
}

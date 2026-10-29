//! User-owned modpack presets, kept separate from server credentials.
use crate::profile::ModManager;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedModChoice {
    pub selected: bool,
    pub version_override: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModpackConfiguration {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub description: String,
    pub website: String,
    pub custom_icon: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub manager: ModManager,
    pub source_root: Option<PathBuf>,
    pub source_profile: String,
    #[serde(default)]
    pub always_increment: bool,
    #[serde(default)]
    pub mods: BTreeMap<String, SavedModChoice>,
    #[serde(default)]
    pub configs: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModpackLibrary {
    pub schema_version: u32,
    pub active_id: Option<u64>,
    pub configurations: Vec<ModpackConfiguration>,
}

impl Default for ModpackLibrary {
    fn default() -> Self {
        Self {
            schema_version: 1,
            active_id: None,
            configurations: Vec::new(),
        }
    }
}

impl ModpackLibrary {
    pub fn path() -> PathBuf {
        crate::appearance::UiPreferences::path().with_file_name("modpacks.json")
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => return Err(format!("Could not read {}: {error}", path.display())),
        };
        let library: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Could not parse {}: {error}", path.display()))?;
        if library.schema_version != 1 {
            return Err(format!(
                "Unsupported saved modpack format in {}",
                path.display()
            ));
        }
        let ids: HashSet<_> = library.configurations.iter().map(|c| c.id).collect();
        if ids.len() != library.configurations.len()
            || library.active_id.is_some_and(|id| !ids.contains(&id))
        {
            return Err(format!("Invalid saved modpack IDs in {}", path.display()));
        }
        Ok(library)
    }

    pub fn configuration(&self, id: u64) -> Option<&ModpackConfiguration> {
        self.configurations.iter().find(|c| c.id == id)
    }

    pub fn next_id(&self) -> Result<u64, String> {
        self.configurations
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| "Saved configuration ID limit reached".into())
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or("Invalid saved modpack path")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temporary = parent.join(format!(".modpacks.{}.{}.tmp", std::process::id(), nonce));
        let mut created = false;
        let result = (|| {
            let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
            let mut options = std::fs::OpenOptions::new();
            options.create_new(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
            created = true;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            std::fs::rename(&temporary, path).map_err(|e| e.to_string())
        })();
        if result.is_err() && created {
            let _ = std::fs::remove_file(&temporary);
        }
        result.map_err(|error| format!("Could not save {}: {error}", path.display()))
    }
}

pub fn increment_patch(version: &str) -> Result<String, String> {
    let mut version = semver::Version::parse(version.trim())
        .map_err(|error| format!("Invalid modpack version: {error}"))?;
    version.patch = version
        .patch
        .checked_add(1)
        .ok_or("Patch version limit reached")?;
    version.pre = semver::Prerelease::EMPTY;
    version.build = semver::BuildMetadata::EMPTY;
    Ok(version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_increment_is_explicit_and_checked() {
        assert_eq!(increment_patch("1.2.9").unwrap(), "1.2.10");
        assert_eq!(increment_patch(" 1.2.3-rc.1+build.4 ").unwrap(), "1.2.4");
        assert!(increment_patch("invalid").is_err());
        assert!(increment_patch(&format!("1.2.{}", u64::MAX)).is_err());
    }

    #[test]
    fn malformed_or_future_libraries_are_not_replaced_on_load() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("modpacks.json");
        assert!(ModpackLibrary::load(&path)
            .unwrap()
            .configurations
            .is_empty());
        for text in [
            "broken",
            r#"{"schema_version":2,"active_id":null,"configurations":[]}"#,
        ] {
            std::fs::write(&path, text).unwrap();
            assert!(ModpackLibrary::load(&path).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        }
    }
}

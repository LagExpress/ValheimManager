//! Package documentation and release history, independent of application releases.
use crate::packager::{inline_code, markdown_text};
use crate::profile::{ConfigItem, ModItem};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadmeOptions {
    pub additional_markdown: String,
    pub include_mods: bool,
    pub include_configs: bool,
}

impl Default for ReadmeOptions {
    fn default() -> Self {
        Self {
            additional_markdown: String::new(),
            include_mods: true,
            include_configs: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PackageDocumentation {
    pub readme: ReadmeOptions,
    pub changelog: PackageChangelog,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleasedMod {
    pub label: String,
    pub version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSnapshot {
    pub mods: BTreeMap<String, ReleasedMod>,
    pub configs: BTreeSet<String>,
    pub name: String,
    pub description: String,
    pub readme: ReadmeOptions,
}

impl PackageSnapshot {
    pub fn new<'a>(
        name: &str,
        description: &str,
        readme: &ReadmeOptions,
        mods: impl Iterator<Item = &'a ModItem>,
        configs: impl Iterator<Item = &'a ConfigItem>,
    ) -> Self {
        Self {
            mods: mods
                .map(|item| {
                    (
                        item.name.clone(),
                        ReleasedMod {
                            label: item.display_name.clone(),
                            version: item.current_version.trim().to_owned(),
                        },
                    )
                })
                .collect(),
            configs: configs.map(|item| item.name.clone()).collect(),
            name: name.to_owned(),
            description: description.to_owned(),
            readme: readme.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReleaseEntry {
    pub version: String,
    pub notes: String,
    pub edited: bool,
    pub snapshot: PackageSnapshot,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChangelogDraft {
    pub notes: String,
    pub edited: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PackageChangelog {
    pub entries: Vec<ReleaseEntry>,
    pub draft: Option<ChangelogDraft>,
}

impl PackageChangelog {
    pub fn release(&self, version: &str, snapshot: PackageSnapshot) -> ReleaseEntry {
        let edited_notes = match &self.draft {
            Some(draft) => draft.edited.then_some(draft.notes.as_str()),
            None => self
                .entries
                .iter()
                .find(|entry| entry.version == version && entry.edited)
                .map(|entry| entry.notes.as_str()),
        };
        let notes = edited_notes.map(str::to_owned).unwrap_or_else(|| {
            let previous = self.entries.iter().find(|entry| entry.version != version);
            automatic_notes(previous.map(|entry| &entry.snapshot), &snapshot)
        });
        ReleaseEntry {
            version: version.to_owned(),
            notes,
            edited: edited_notes.is_some(),
            snapshot,
        }
    }

    pub fn document(&self, release: &ReleaseEntry) -> String {
        let mut text = String::from("# Changelog\n\n");
        for entry in std::iter::once(release).chain(
            self.entries
                .iter()
                .filter(|entry| entry.version != release.version),
        ) {
            text.push_str(&format!(
                "## {}\n\n{}\n\n",
                entry.version,
                entry.notes.trim_matches(['\r', '\n'])
            ));
        }
        text
    }

    pub fn record(&mut self, release: ReleaseEntry) {
        self.entries
            .retain(|entry| entry.version != release.version);
        self.entries.insert(0, release);
        self.draft = None;
    }
}

fn automatic_notes(previous: Option<&PackageSnapshot>, current: &PackageSnapshot) -> String {
    let Some(previous) = previous else {
        return format!(
            "### Added\n\n- Initial release with {} mods and {} configuration entries.\n",
            current.mods.len(),
            current.configs.len()
        );
    };
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    for (name, item) in &current.mods {
        match previous.mods.get(name) {
            None => added.push(format!(
                "**{}** {}",
                markdown_text(&item.label),
                inline_code(&item.version)
            )),
            Some(old) if old.version != item.version => changed.push(format!(
                "**{}**: {} → {}",
                markdown_text(&item.label),
                inline_code(&old.version),
                inline_code(&item.version)
            )),
            _ => {}
        }
    }
    for (name, item) in &previous.mods {
        if !current.mods.contains_key(name) {
            removed.push(format!(
                "**{}** {}",
                markdown_text(&item.label),
                inline_code(&item.version)
            ));
        }
    }
    for name in current.configs.difference(&previous.configs) {
        added.push(format!("Configuration: {}", inline_code(name)));
    }
    for name in previous.configs.difference(&current.configs) {
        removed.push(format!("Configuration: {}", inline_code(name)));
    }
    if current.name != previous.name
        || current.description != previous.description
        || current.readme != previous.readme
    {
        changed.push("Updated package information".into());
    }
    let mut text = String::new();
    for (heading, entries) in [("Added", added), ("Changed", changed), ("Removed", removed)] {
        if !entries.is_empty() {
            text.push_str(&format!("### {heading}\n\n"));
            for entry in entries {
                text.push_str(&format!("- {entry}.\n"));
            }
            text.push('\n');
        }
    }
    if text.is_empty() {
        text.push_str("No mod-version or configuration-selection changes.\n");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> PackageSnapshot {
        PackageSnapshot {
            name: "North".into(),
            description: "Explore".into(),
            mods: BTreeMap::from([
                (
                    "Team-First".into(),
                    ReleasedMod {
                        label: "First".into(),
                        version: "1.0.0".into(),
                    },
                ),
                (
                    "Team-Old".into(),
                    ReleasedMod {
                        label: "Old".into(),
                        version: "2.0.0".into(),
                    },
                ),
            ]),
            configs: BTreeSet::from(["old.cfg".into()]),
            ..Default::default()
        }
    }

    #[test]
    fn changelog_tracks_changes_retains_history_and_rebuilds_without_duplicates() {
        let mut history = PackageChangelog::default();
        let first = history.release("1.0.0", snapshot());
        assert!(first
            .notes
            .contains("Initial release with 2 mods and 1 configuration"));
        history.record(first);
        let mut next = snapshot();
        next.mods.get_mut("Team-First").unwrap().version = "1.1.0".into();
        next.mods.remove("Team-Old");
        next.mods.insert(
            "Team-New".into(),
            ReleasedMod {
                label: "New".into(),
                version: "3.0.0".into(),
            },
        );
        next.configs = BTreeSet::from(["new.cfg".into()]);
        let second = history.release("1.0.1", next.clone());
        for text in [
            "### Added",
            "### Changed",
            "### Removed",
            "**First**: `1.0.0` → `1.1.0`",
            "**New**",
            "**Old**",
            "`new.cfg`",
            "`old.cfg`",
        ] {
            assert!(second.notes.contains(text), "{text}: {}", second.notes);
        }
        let document = history.document(&second);
        assert_eq!(document.matches("## 1.0.0\n").count(), 1);
        history.record(second.clone());
        let rebuild = history.release("1.0.1", next.clone());
        assert_eq!(rebuild.notes, second.notes);
        history.record(rebuild);
        assert_eq!(history.entries.len(), 2);
        let unchanged = history.release("1.0.2", next);
        assert!(unchanged.notes.starts_with("No mod-version"));
        // Preparing a preview or a failed attempt never records a release.
        assert_eq!(history.entries[0].version, "1.0.1");
    }

    #[test]
    fn manual_notes_survive_rebuilds_and_regeneration_is_explicit() {
        let mut history = PackageChangelog {
            draft: Some(ChangelogDraft {
                notes: "### Fixed\n\n- Adjusted combat.\n".into(),
                edited: true,
            }),
            ..Default::default()
        };
        let entry = history.release("1.0.1", snapshot());
        assert!(entry.edited && entry.notes.contains("Adjusted combat"));
        history.record(entry.clone());
        assert!(history.draft.is_none());
        assert_eq!(history.release("1.0.1", snapshot()).notes, entry.notes);
        assert!(!history.release("1.0.2", snapshot()).edited);
        history.draft = Some(ChangelogDraft::default());
        let regenerated = history.release("1.0.1", snapshot());
        assert!(!regenerated.edited);
        assert!(regenerated.notes.contains("Initial release"));
        // Regeneration does not alter earlier committed history until a build succeeds.
        assert_eq!(history.entries[0].notes, entry.notes);
        let json = serde_json::to_string(&history).unwrap();
        let restored: PackageChangelog = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.document(&regenerated),
            history.document(&regenerated)
        );
    }
}

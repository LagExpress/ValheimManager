use super::*;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Default)]
pub(super) struct ReadmePreview {
    pub markdown: String,
    pub changelog_markdown: String,
    pub changelog_editor: String,
    pub release: Option<crate::package_docs::ReleaseEntry>,
    pub dirty: bool,
    pub last_tab: Option<ActiveTab>,
    pub generation_key: Option<(u64, String, bool, bool, u64)>,
    pub source_view: bool,
    pub show_changelog: bool,
    pub renderer: egui_commonmark::CommonMarkCache,
    pub context: Option<egui::Context>,
    pub style_key: Option<(ThemePreset, u32)>,
}

impl ModpackApp {
    pub(super) fn refresh_readme_preview(&mut self) {
        let mut hasher = DefaultHasher::new();
        (
            &self.pack_name,
            &self.pack_description,
            &self.documentation.readme,
        )
            .hash(&mut hasher);
        let metadata = hasher.finish();
        let unchanged = self
            .readme_preview
            .generation_key
            .as_ref()
            .is_some_and(|key| {
                key.0 == metadata
                    && key.1 == self.pack_version
                    && key.2 == self.always_increment_version
                    && key.3 == self.building
                    && key.4 == self.data_revision
            });
        if unchanged && !self.readme_preview.dirty {
            return;
        }
        let version = if self.always_increment_version && !self.building {
            crate::modpack_config::increment_patch(&self.pack_version)
                .unwrap_or_else(|_| self.pack_version.trim().to_owned())
        } else {
            self.pack_version.trim().to_owned()
        };
        self.readme_preview.markdown = crate::packager::generate_readme(
            &self.pack_name,
            &version,
            &self.pack_description,
            &self.documentation.readme,
            self.mods.iter().filter(|item| item.selected),
            self.configs.iter().filter(|item| item.selected),
        );
        let snapshot = crate::package_docs::PackageSnapshot::new(
            &crate::packager::clean_package_name(&self.pack_name),
            crate::packager::package_description(&self.pack_description),
            &self.documentation.readme,
            self.mods.iter().filter(|item| item.selected),
            self.configs.iter().filter(|item| item.selected),
        );
        let release = self.documentation.changelog.release(&version, snapshot);
        self.readme_preview.changelog_editor = release.notes.clone();
        self.readme_preview.changelog_markdown = self.documentation.changelog.document(&release);
        self.readme_preview.release = Some(release);
        self.readme_preview.renderer.clear_viewers();
        self.readme_preview.generation_key = Some((
            metadata,
            self.pack_version.clone(),
            self.always_increment_version,
            self.building,
            self.data_revision,
        ));
        self.readme_preview.dirty = false;
    }
}

use super::*;

pub(super) struct BuildVersionPrompt {
    pub next_version: Result<String, String>,
    pub always_increment: bool,
}

pub(super) struct BuildAttempt {
    configuration_id: u64,
    previous_version: String,
    version: String,
}

impl ModpackApp {
    pub(super) fn restore_modpack_library(&mut self) {
        match ModpackLibrary::load(&self.modpack_library_path) {
            Ok(library) => self.modpack_library = library,
            Err(error) => self.modpack_library_error = Some(error),
        }
        let active = self
            .modpack_library
            .active_id
            .and_then(|id| self.modpack_library.configuration(id))
            .cloned();
        if let Some(configuration) = active {
            self.apply_modpack_configuration(&configuration);
        } else {
            self.refresh_profiles();
        }
        if let Some(error) = &self.modpack_library_error {
            self.status_message = Some((error.clone(), true));
        }
    }

    fn configuration_snapshot(&self, id: u64) -> ModpackConfiguration {
        ModpackConfiguration {
            id,
            name: self.pack_name.trim().to_owned(),
            version: self.pack_version.trim().to_owned(),
            description: self.pack_description.clone(),
            documentation: self.documentation.clone(),
            website: self.pack_website.clone(),
            custom_icon: self.custom_icon_path.clone(),
            output_dir: self.output_dir.clone(),
            manager: self.mod_manager,
            source_root: self.selected_root.clone(),
            source_profile: self.selected_profile.clone(),
            always_increment: self.always_increment_version,
            mods: self
                .mods
                .iter()
                .map(|item| {
                    (
                        item.name.clone(),
                        SavedModChoice {
                            selected: item.selected,
                            version_override: (item.current_version != item.original_version)
                                .then(|| item.current_version.clone()),
                        },
                    )
                })
                .collect(),
            configs: self
                .configs
                .iter()
                .map(|item| (item.name.clone(), item.selected))
                .collect(),
        }
    }

    pub(super) fn save_modpack_configuration(&mut self, as_new: bool) -> Result<u64, String> {
        if let Some(error) = &self.modpack_library_error {
            return Err(format!(
                "{error}. Fix or move this file and restart before saving configurations."
            ));
        }
        if self.pack_name.trim().is_empty() {
            return Err("Modpack name cannot be empty".into());
        }
        semver::Version::parse(self.pack_version.trim())
            .map_err(|error| format!("Invalid modpack version: {error}"))?;
        let mut library = self.modpack_library.clone();
        let id = if as_new { None } else { library.active_id }
            .map(Ok)
            .unwrap_or_else(|| library.next_id())?;
        let snapshot = self.configuration_snapshot(id);
        if let Some(configuration) = library.configurations.iter_mut().find(|c| c.id == id) {
            *configuration = snapshot;
        } else {
            library.configurations.push(snapshot);
        }
        library.active_id = Some(id);
        library.save(&self.modpack_library_path)?;
        self.modpack_library = library;
        Ok(id)
    }

    pub(super) fn load_modpack_configuration(&mut self, id: u64) -> Result<(), String> {
        if self.building {
            return Err("Wait for the current build to finish".into());
        }
        let configuration = self
            .modpack_library
            .configuration(id)
            .cloned()
            .ok_or("Saved configuration not found")?;
        let mut library = self.modpack_library.clone();
        library.active_id = Some(id);
        library.save(&self.modpack_library_path)?;
        self.modpack_library = library;
        self.apply_modpack_configuration(&configuration);
        self.last_build_result = None;
        self.build_progress = None;
        Ok(())
    }

    fn apply_modpack_configuration(&mut self, configuration: &ModpackConfiguration) {
        self.mod_manager = configuration.manager;
        self.selected_root = configuration.source_root.clone();
        self.selected_profile = configuration.source_profile.clone();
        self.profile_roots = match self.mod_manager {
            ModManager::R2Modman => get_r2modman_roots(),
            ModManager::Gale => get_gale_roots(),
            ModManager::Custom => self.selected_root.iter().cloned().collect(),
        };
        self.refresh_profiles();
        // Preserve the saved source even when its folder is currently unavailable.
        if !configuration.source_profile.is_empty()
            && !self
                .available_profiles
                .contains(&configuration.source_profile)
        {
            self.selected_profile = configuration.source_profile.clone();
            self.mods.clear();
            self.configs.clear();
            self.status_message = Some((
                "Saved source profile is unavailable; choose its folder or another profile.".into(),
                true,
            ));
        }
        self.pack_name = configuration.name.clone();
        self.pack_version = configuration.version.clone();
        self.pack_description = configuration.description.clone();
        self.documentation = configuration.documentation.clone();
        self.readme_preview.dirty = true;
        self.pack_website = configuration.website.clone();
        self.custom_icon_path = configuration.custom_icon.clone();
        self.output_dir = configuration.output_dir.clone();
        self.always_increment_version = configuration.always_increment;
    }

    pub(super) fn restore_saved_selections(&mut self) {
        let Some(configuration) = self
            .modpack_library
            .active_id
            .and_then(|id| self.modpack_library.configuration(id))
        else {
            return;
        };
        if configuration.source_root != self.selected_root
            || configuration.source_profile != self.selected_profile
            || configuration.manager != self.mod_manager
        {
            return;
        }
        for item in &mut self.mods {
            if let Some(saved) = configuration.mods.get(&item.name) {
                item.selected = saved.selected;
                if let Some(version) = &saved.version_override {
                    item.current_version = version.clone();
                }
            }
        }
        for item in &mut self.configs {
            if let Some(selected) = configuration.configs.get(&item.name) {
                item.selected = *selected;
            }
        }
    }

    pub(super) fn validate_build_request(&self) -> Result<(), String> {
        if self.pack_name.trim().is_empty() {
            return Err("Modpack name cannot be empty".into());
        }
        semver::Version::parse(self.pack_version.trim())
            .map_err(|error| format!("Invalid modpack version: {error}"))?;
        if !self.mods.iter().any(|item| item.selected) {
            return Err("No mods selected for the modpack".into());
        }
        if self.output_dir.as_os_str().is_empty() {
            return Err("Choose an output folder".into());
        }
        Ok(())
    }

    pub(super) fn start_versioned_build(
        &mut self,
        ctx: &egui::Context,
        increment: bool,
        always: bool,
    ) {
        if self.building {
            return;
        }
        self.build_version_prompt = None;
        let result = (|| {
            self.validate_build_request()?;
            let version = if increment {
                crate::modpack_config::increment_patch(&self.pack_version)?
            } else {
                self.pack_version.trim().to_owned()
            };
            let previous_preference = self.always_increment_version;
            self.always_increment_version = always;
            let id = match self.save_modpack_configuration(false) {
                Ok(id) => id,
                Err(error) => {
                    self.always_increment_version = previous_preference;
                    return Err(error);
                }
            };
            self.build_attempt = Some(BuildAttempt {
                configuration_id: id,
                previous_version: self.pack_version.clone(),
                version: version.clone(),
            });
            self.pack_version = version;
            self.launch_build(ctx);
            Ok(())
        })();
        if let Err(error) = result {
            self.status_message = Some((error, true));
        }
    }

    pub(super) fn finish_build_version(
        &mut self,
        result: Option<&BuildResult>,
    ) -> Result<(), String> {
        let Some(attempt) = self.build_attempt.take() else {
            return Ok(());
        };
        let Some(result) = result else {
            if self.pack_version == attempt.version {
                self.pack_version = attempt.previous_version;
            }
            return Ok(());
        };
        let mut library = self.modpack_library.clone();
        let configuration = library
            .configurations
            .iter_mut()
            .find(|c| c.id == attempt.configuration_id)
            .ok_or("Built configuration not found")?;
        configuration.version = attempt.version;
        configuration
            .documentation
            .changelog
            .record((*result.release).clone());
        let changelog = configuration.documentation.changelog.clone();
        library.save(&self.modpack_library_path)?;
        self.modpack_library = library;
        self.documentation.changelog = changelog;
        self.readme_preview.dirty = true;
        Ok(())
    }
}

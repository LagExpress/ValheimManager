use std::path::PathBuf;
use eframe::egui::{self, Color32, RichText, Ui};
use crate::profile::{
    get_default_profile_roots, list_profiles_in_dir, load_mods_from_profile,
    scan_profile_configs, ConfigCategory, ConfigItem, ModItem,
};
use crate::packager::{build_modpack, BuildResult, ModpackOptions};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ActiveTab {
    ProfileMetadata,
    Mods,
    Configs,
    Build,
    Help,
}

pub struct ModpackApp {
    // Directories & Profile
    #[allow(dead_code)]
    pub profile_roots: Vec<PathBuf>,
    pub selected_root: Option<PathBuf>,
    pub available_profiles: Vec<String>,
    pub selected_profile: String,

    // Data loaded from current profile
    pub mods: Vec<ModItem>,
    pub configs: Vec<ConfigItem>,

    // Search filters
    pub mod_search: String,
    pub config_search: String,

    // Metadata
    pub pack_name: String,
    pub pack_version: String,
    pub pack_description: String,
    pub pack_website: String,
    pub custom_icon_path: Option<PathBuf>,

    // UI State
    pub active_tab: ActiveTab,
    pub status_message: Option<(String, bool)>, // (message, is_error)
    pub last_build_result: Option<BuildResult>,
}

impl Default for ModpackApp {
    fn default() -> Self {
        let roots = get_default_profile_roots();
        let selected_root = roots.first().cloned();
        let mut app = Self {
            profile_roots: roots,
            selected_root,
            available_profiles: Vec::new(),
            selected_profile: String::new(),
            mods: Vec::new(),
            configs: Vec::new(),
            mod_search: String::new(),
            config_search: String::new(),
            pack_name: "Valheim_Modpack".to_string(),
            pack_version: "1.0.0".to_string(),
            pack_description: "A curated Valheim modpack with custom configurations".to_string(),
            pack_website: String::new(),
            custom_icon_path: None,
            active_tab: ActiveTab::ProfileMetadata,
            status_message: None,
            last_build_result: None,
        };

        app.refresh_profiles();
        app
    }
}

impl ModpackApp {
    pub fn refresh_profiles(&mut self) {
        if let Some(ref root) = self.selected_root {
            self.available_profiles = list_profiles_in_dir(root);
            if !self.available_profiles.is_empty() {
                if !self.available_profiles.contains(&self.selected_profile) {
                    if self.available_profiles.contains(&"Default".to_string()) {
                        self.selected_profile = "Default".to_string();
                    } else {
                        self.selected_profile = self.available_profiles[0].clone();
                    }
                }
                self.load_selected_profile_data();
            } else {
                self.mods.clear();
                self.configs.clear();
            }
        }
    }

    pub fn load_selected_profile_data(&mut self) {
        if let Some(ref root) = self.selected_root {
            let profile_path = root.join(&self.selected_profile);
            match load_mods_from_profile(&profile_path) {
                Ok(m) => {
                    self.mods = m;
                    self.status_message = None;
                }
                Err(e) => {
                    self.status_message = Some((format!("Error loading mods: {}", e), true));
                }
            }
            self.configs = scan_profile_configs(&profile_path);

            if self.pack_name.is_empty() || self.pack_name == "Valheim_Modpack" {
                self.pack_name = format!("{}_Modpack", self.selected_profile)
                    .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
            }
        }
    }

    pub fn trigger_build(&mut self) {
        let clean_name = self.pack_name.trim().replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
        if clean_name.is_empty() {
            self.status_message = Some(("Modpack name cannot be empty".to_string(), true));
            return;
        }

        let selected_mods: Vec<ModItem> = self.mods.iter().filter(|m| m.selected).cloned().collect();
        let selected_configs: Vec<ConfigItem> = self.configs.iter().filter(|c| c.selected).cloned().collect();

        if selected_mods.is_empty() {
            self.status_message = Some(("Warning: No mods selected for the modpack".to_string(), true));
            return;
        }

        let opts = ModpackOptions {
            name: clean_name,
            version: self.pack_version.clone(),
            description: self.pack_description.clone(),
            website_url: self.pack_website.clone(),
            selected_mods,
            selected_configs,
            custom_icon: self.custom_icon_path.clone(),
            out_dir: PathBuf::from("./build"),
            dist_dir: PathBuf::from("./dist"),
        };

        match build_modpack(&opts) {
            Ok(result) => {
                let size_kb = (result.zip_size_bytes as f64) / 1024.0;
                self.status_message = Some((
                    format!(
                        "Successfully built {} ({:.1} KB) with {} mods and {} configs!",
                        result.zip_path.display(),
                        size_kb,
                        result.mod_count,
                        result.config_count
                    ),
                    false,
                ));
                self.last_build_result = Some(result);
                self.active_tab = ActiveTab::Build;
            }
            Err(e) => {
                self.status_message = Some((format!("Build failed: {}", e), true));
            }
        }
    }
}

impl eframe::App for ModpackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keep active heartbeat so compositor restore/unminimize immediately repaints
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Top Navigation Bar
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("🛡️ Valheim Modpack Studio").color(Color32::from_rgb(230, 180, 50)).strong());
                ui.label(RichText::new("v1.0.0").weak());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref res) = self.last_build_result {
                        if ui.button("📂 Open Output Folder").clicked() {
                            let _ = opener::reveal(&res.zip_path);
                        }
                    }
                    if ui.button(RichText::new("✨ Build Modpack ZIP").strong().color(Color32::WHITE))
                        .on_hover_text("Build Thunderstore zip archive now")
                        .clicked()
                    {
                        self.trigger_build();
                    }
                });
            });
            ui.add_space(4.0);

            // Tab bar
            ui.horizontal(|ui| {
                let tabs = [
                    (ActiveTab::ProfileMetadata, "📂 Profile & Info"),
                    (ActiveTab::Mods, &format!("📦 Mods ({})", self.mods.iter().filter(|m| m.selected).count())),
                    (ActiveTab::Configs, &format!("⚙️ Configs ({})", self.configs.iter().filter(|c| c.selected).count())),
                    (ActiveTab::Build, "🚀 Build & Export"),
                    (ActiveTab::Help, "ℹ️ Instructions"),
                ];

                for (tab, label) in tabs {
                    let is_active = self.active_tab == tab;
                    if ui.selectable_label(is_active, RichText::new(label).strong()).clicked() {
                        self.active_tab = tab;
                    }
                }
            });
            ui.add_space(4.0);
        });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some((ref msg, is_err)) = self.status_message {
                    let color = if is_err {
                        Color32::from_rgb(255, 90, 90)
                    } else {
                        Color32::from_rgb(90, 220, 90)
                    };
                    ui.label(RichText::new(msg).color(color));
                } else {
                    ui.label(RichText::new(format!(
                        "Profile: {} | {} mods selected | {} configs selected",
                        self.selected_profile,
                        self.mods.iter().filter(|m| m.selected).count(),
                        self.configs.iter().filter(|c| c.selected).count()
                    )).weak());
                }
            });
        });

        // Central Panel
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                ActiveTab::ProfileMetadata => self.ui_profile_metadata(ui),
                ActiveTab::Mods => self.ui_mods_selection(ui),
                ActiveTab::Configs => self.ui_configs_selection(ui),
                ActiveTab::Build => self.ui_build_export(ui),
                ActiveTab::Help => self.ui_help(ui),
            }
        });
    }
}

impl ModpackApp {
    fn ui_profile_metadata(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading("📂 r2modman Profile Source");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Profiles Directory:");
                    let current_dir_display = self
                        .selected_root
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Not found".to_string());
                    ui.monospace(current_dir_display);

                    if ui.button("Browse...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.selected_root = Some(folder);
                            self.refresh_profiles();
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Selected Profile:");
                    let current_profile = self.selected_profile.clone();
                    let mut changed_profile = None;
                    egui::ComboBox::from_id_salt("profile_select")
                        .selected_text(&current_profile)
                        .show_ui(ui, |ui| {
                            for p in &self.available_profiles {
                                if ui.selectable_label(self.selected_profile == *p, p).clicked() {
                                    changed_profile = Some(p.clone());
                                }
                            }
                        });
                    if let Some(p) = changed_profile {
                        self.selected_profile = p;
                        self.load_selected_profile_data();
                    }

                    if ui.button("🔄 Reload").clicked() {
                        self.refresh_profiles();
                    }

                    ui.label(RichText::new(format!(
                        "({} mods installed, {} enabled)",
                        self.mods.len(),
                        self.mods.iter().filter(|m| m.enabled).count()
                    )).weak());
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.heading("📝 Thunderstore Modpack Metadata");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Package Name:");
                    let mut name = self.pack_name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        self.pack_name = name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                    }
                    ui.label(RichText::new("(Letters, numbers, and underscores only)").weak());
                });

                ui.horizontal(|ui| {
                    ui.label("Version Number:");
                    ui.text_edit_singleline(&mut self.pack_version);
                    ui.label(RichText::new("(SemVer X.Y.Z, e.g. 1.0.0)").weak());
                });

                ui.horizontal(|ui| {
                    ui.label("Short Description:");
                    ui.add(egui::TextEdit::singleline(&mut self.pack_description).desired_width(400.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Website / GitHub URL:");
                    ui.add(egui::TextEdit::singleline(&mut self.pack_website).desired_width(400.0));
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Icon (256x256 PNG):");
                    if let Some(ref icon) = self.custom_icon_path {
                        ui.label(RichText::new(icon.file_name().unwrap_or_default().to_string_lossy()).strong());
                        if ui.button("Reset to Default Rune Icon").clicked() {
                            self.custom_icon_path = None;
                        }
                    } else {
                        ui.label(RichText::new("Default Embedded Norse Rune Icon").color(Color32::from_rgb(230, 180, 50)));
                        if ui.button("Choose Custom PNG...").clicked() {
                            if let Some(file) = rfd::FileDialog::new()
                                .add_filter("PNG Image", &["png"])
                                .pick_file()
                            {
                                self.custom_icon_path = Some(file);
                            }
                        }
                    }
                });
            });

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Next: Select Mods ➡️").strong()).clicked() {
                    self.active_tab = ActiveTab::Mods;
                }
            });
        });
    }

    fn ui_mods_selection(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("📦 Mod Dependencies");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Deselect All").clicked() {
                    for m in &mut self.mods {
                        m.selected = false;
                    }
                }
                if ui.button("Select Enabled Only").clicked() {
                    for m in &mut self.mods {
                        m.selected = m.enabled;
                    }
                }
                if ui.button("Select All").clicked() {
                    for m in &mut self.mods {
                        m.selected = true;
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.mod_search);
            if !self.mod_search.is_empty() && ui.button("❌").clicked() {
                self.mod_search.clear();
            }
            ui.label(RichText::new(format!(
                "Selected: {} / {}",
                self.mods.iter().filter(|m| m.selected).count(),
                self.mods.len()
            )).strong());
        });
        ui.separator();

        let search = self.mod_search.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("mods_table")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Select").strong());
                    ui.label(RichText::new("Mod Name").strong());
                    ui.label(RichText::new("Author").strong());
                    ui.label(RichText::new("Version").strong());
                    ui.label(RichText::new("Thunderstore ID").strong());
                    ui.end_row();

                    for m in &mut self.mods {
                        if !search.is_empty()
                            && !m.name.to_lowercase().contains(&search)
                            && !m.display_name.to_lowercase().contains(&search)
                            && !m.author_name.to_lowercase().contains(&search)
                        {
                            continue;
                        }

                        ui.checkbox(&mut m.selected, "");
                        let name_label = ui.label(RichText::new(&m.display_name).strong());
                        if !m.description.is_empty() {
                            name_label.on_hover_text(&m.description);
                        }
                        ui.label(&m.author_name);

                        // Editable version
                        let ver_edit = ui.add(egui::TextEdit::singleline(&mut m.current_version).desired_width(70.0));
                        if m.current_version != m.original_version {
                            ver_edit.on_hover_text(format!("Original version: {}", m.original_version));
                        }

                        ui.monospace(&m.name);
                        ui.end_row();
                    }
                });
        });
    }

    fn ui_configs_selection(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("⚙️ Configuration Files");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Deselect All").clicked() {
                    for c in &mut self.configs {
                        c.selected = false;
                    }
                }
                if ui.button("Select Safe Only (Recommended)").clicked() {
                    for c in &mut self.configs {
                        c.selected = c.category == ConfigCategory::Safe;
                    }
                }
                if ui.button("Select All").clicked() {
                    for c in &mut self.configs {
                        c.selected = true;
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.config_search);
            if !self.config_search.is_empty() && ui.button("❌").clicked() {
                self.config_search.clear();
            }
            ui.label(RichText::new(format!(
                "Selected: {} / {}",
                self.configs.iter().filter(|c| c.selected).count(),
                self.configs.len()
            )).strong());
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("🟢 Safe").color(Color32::from_rgb(100, 220, 100)));
            ui.label("|");
            ui.label(RichText::new("🔴 Sensitive / Passwords (Auto-excluded)").color(Color32::from_rgb(255, 100, 100)));
            ui.label("|");
            ui.label(RichText::new("🟡 Runtime Cache / Reports (Auto-excluded)").color(Color32::from_rgb(240, 200, 80)));
        });
        ui.separator();

        let search = self.config_search.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("configs_table")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Select").strong());
                    ui.label(RichText::new("Status").strong());
                    ui.label(RichText::new("Name").strong());
                    ui.label(RichText::new("Type").strong());
                    ui.label(RichText::new("Size").strong());
                    ui.end_row();

                    for cfg in &mut self.configs {
                        if !search.is_empty() && !cfg.name.to_lowercase().contains(&search) {
                            continue;
                        }

                        ui.checkbox(&mut cfg.selected, "");

                        let badge_color = match cfg.category {
                            ConfigCategory::Safe => Color32::from_rgb(100, 220, 100),
                            ConfigCategory::Sensitive => Color32::from_rgb(255, 100, 100),
                            ConfigCategory::Cache => Color32::from_rgb(240, 200, 80),
                        };
                        ui.label(RichText::new(cfg.category.label()).color(badge_color));

                        ui.monospace(&cfg.name);
                        ui.label(if cfg.is_dir { "Directory" } else { "File" });

                        if cfg.is_dir {
                            ui.label("-");
                        } else {
                            ui.label(format!("{:.1} KB", cfg.size as f64 / 1024.0));
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn ui_build_export(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading("📦 Ready to Build Modpack");
                ui.separator();

                let selected_mods_count = self.mods.iter().filter(|m| m.selected).count();
                let selected_configs_count = self.configs.iter().filter(|c| c.selected).count();

                ui.label(RichText::new(format!("Package: {} v{}", self.pack_name, self.pack_version)).heading());
                ui.label(format!("• Mods to include: {} dependencies", selected_mods_count));
                ui.label(format!("• Configurations to include: {} files/folders", selected_configs_count));
                ui.label(format!("• Target ZIP: ./dist/{}-{}.zip", self.pack_name, self.pack_version));

                ui.add_space(8.0);
                if ui.button(RichText::new("✨ GENERATE MODPACK ZIP NOW ✨").heading().color(Color32::WHITE)).clicked() {
                    self.trigger_build();
                }
            });

            if let Some(ref res) = self.last_build_result {
                ui.add_space(16.0);
                ui.group(|ui| {
                    ui.heading(RichText::new("🎉 Build Successful!").color(Color32::from_rgb(90, 220, 90)));
                    ui.separator();
                    ui.label(format!("ZIP Archive: {}", res.zip_path.display()));
                    ui.label(format!("Size: {:.1} KB", (res.zip_size_bytes as f64) / 1024.0));
                    ui.label(format!("Unpacked Directory: {}", res.build_dir.display()));

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("📂 Open Output Folder").clicked() {
                            let _ = opener::reveal(&res.zip_path);
                        }
                    });
                });
            }
        });
    }

    fn ui_help(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading("📖 How to Distribute Your Modpack");
            ui.separator();

            ui.label(RichText::new("Option 1: Upload to Thunderstore (Public or Unlisted)").strong());
            ui.label("1. Create an account on https://thunderstore.io/c/valheim/");
            ui.label("2. Click 'Upload' in the top-right header.");
            ui.label("3. Select your community team, attach the generated .zip from dist/, and tag 'Modpacks'.");
            ui.label("4. When players install it through r2modman, all mods and configs are downloaded automatically!");

            ui.add_space(12.0);
            ui.label(RichText::new("Option 2: Direct Share with Friends").strong());
            ui.label("You can send the .zip to friends or extract it into their Valheim BepInEx folder.");
        });
    }
}

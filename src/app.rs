use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Instant;

use crate::i18n::Language;
use crate::packager::{build_modpack, BuildResult, ModpackOptions};
use crate::profile::{
    get_gale_roots, get_r2modman_roots, list_profiles_in_dir, load_mods_from_profile,
    scan_profile_configs, ConfigCategory, ConfigItem, ModItem, ModManager,
};
use crate::sftp::WrapperStatus;
use crate::sync::{
    self, LogLevel, PreviewResult, Progress, SyncConfig, SyncEvent, SyncLog, SyncMode, SyncRequest,
    SyncSummary, DEFAULT_IGNORED_PATTERNS, IGNORE_FILE_NAME,
};
use eframe::egui::{self, Color32, RichText, Ui};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ActiveTab {
    ProfileMetadata,
    Mods,
    Configs,
    Build,
    ServerSync,
    Help,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PreviewTab {
    All,
    Uploads,
    Deletions,
}

pub struct SyncUiState {
    pub config: SyncConfig,
    pub ignore_text: String,
    pub show_password: bool,
    pub running: bool,
    pub cancel: Option<Arc<AtomicBool>>,
    pub receiver: Option<Receiver<SyncEvent>>,
    pub logs: Vec<SyncLog>,
    pub progress: Option<Progress>,
    pub preview: Option<PreviewResult>,
    pub summary: Option<SyncSummary>,
    pub active_mode: Option<SyncMode>,
    pub warning: Option<String>,

    // UI/UX improvements
    pub preview_tab: PreviewTab,
    pub preview_search: String,
    pub log_filter: Option<LogLevel>,
    pub log_search: String,
    pub log_autoscroll: bool,
    pub copied_feedback: Option<Instant>,
}

impl SyncUiState {
    fn new() -> Self {
        let (config, warning) = match SyncConfig::load() {
            Ok(config) => (config, None),
            Err(e) => (SyncConfig::default(), Some(e)),
        };
        let staging = sync::expand_tilde(&config.staging_dir);
        let ignore_text = if staging.as_os_str().is_empty() {
            String::new()
        } else {
            std::fs::read_to_string(staging.join(IGNORE_FILE_NAME)).unwrap_or_default()
        };

        Self {
            config,
            ignore_text,
            show_password: false,
            running: false,
            cancel: None,
            receiver: None,
            logs: Vec::new(),
            progress: None,
            preview: None,
            summary: None,
            active_mode: None,
            warning,
            preview_tab: PreviewTab::All,
            preview_search: String::new(),
            log_filter: None,
            log_search: String::new(),
            log_autoscroll: true,
            copied_feedback: None,
        }
    }

    fn persist(&self) -> Result<(), String> {
        let staging = sync::expand_tilde(&self.config.staging_dir);
        if staging.as_os_str().is_empty() {
            return Err("Staging folder cannot be empty.".to_string());
        }
        self.config.save()?;
        std::fs::create_dir_all(&staging)
            .map_err(|e| format!("Could not create {}: {}", staging.display(), e))?;
        std::fs::write(staging.join(IGNORE_FILE_NAME), &self.ignore_text)
            .map_err(|e| format!("Could not write the ignore file: {}", e))
    }

    fn reload_ignore(&mut self) {
        let staging = sync::expand_tilde(&self.config.staging_dir);
        if staging.as_os_str().is_empty() {
            self.ignore_text.clear();
            return;
        }
        let path = staging.join(IGNORE_FILE_NAME);
        self.ignore_text = std::fs::read_to_string(path).unwrap_or_default();
    }
}

pub struct ModpackApp {
    // Language
    pub language: Language,

    // Mod Manager & Profiles
    pub mod_manager: ModManager,
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

    // Metadata & Output
    pub pack_name: String,
    pub pack_version: String,
    pub pack_description: String,
    pub pack_website: String,
    pub custom_icon_path: Option<PathBuf>,
    pub output_dir: PathBuf,

    // UI State
    pub active_tab: ActiveTab,
    pub status_message: Option<(String, bool)>, // (message, is_error)
    pub last_build_result: Option<BuildResult>,

    // Server Sync
    pub sync: SyncUiState,
}

impl Default for ModpackApp {
    fn default() -> Self {
        let r2_roots = get_r2modman_roots();
        let gale_roots = get_gale_roots();

        let (mod_manager, roots) = if !r2_roots.is_empty() {
            (ModManager::R2Modman, r2_roots)
        } else if !gale_roots.is_empty() {
            (ModManager::Gale, gale_roots)
        } else {
            (ModManager::Custom, Vec::new())
        };

        let selected_root = roots.first().cloned();
        let default_output_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("dist");

        let sync = SyncUiState::new();
        let language = sync.config.language.unwrap_or(Language::English);

        let mut app = Self {
            language,
            mod_manager,
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
            output_dir: default_output_dir,
            active_tab: ActiveTab::ProfileMetadata,
            status_message: None,
            last_build_result: None,
            sync,
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
        let clean_name = self
            .pack_name
            .trim()
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
        if clean_name.is_empty() {
            self.status_message = Some(("Modpack name cannot be empty".to_string(), true));
            return;
        }

        let selected_mods: Vec<ModItem> =
            self.mods.iter().filter(|m| m.selected).cloned().collect();
        let selected_configs: Vec<ConfigItem> = self
            .configs
            .iter()
            .filter(|c| c.selected)
            .cloned()
            .collect();

        if selected_mods.is_empty() {
            self.status_message = Some((
                "Warning: No mods selected for the modpack".to_string(),
                true,
            ));
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
            out_dir: self.output_dir.join("build"),
            dist_dir: self.output_dir.clone(),
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

    fn sync_source_bepinex(&self) -> Option<PathBuf> {
        let root = self.selected_root.as_ref()?;
        if self.selected_profile.is_empty() {
            Some(root.join("BepInEx"))
        } else {
            Some(root.join(&self.selected_profile).join("BepInEx"))
        }
    }

    fn sync_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.sync.logs.push(SyncLog {
            level: LogLevel::Error,
            message: message.clone(),
        });
        self.status_message = Some((message, true));
    }

    fn poll_sync_events(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;

        if let Some(receiver) = &self.sync.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        for event in events {
            self.handle_sync_event(event);
        }

        if disconnected {
            if self.sync.running {
                self.sync.logs.push(SyncLog {
                    level: LogLevel::Error,
                    message: "Sync worker stopped unexpectedly.".to_string(),
                });
            }
            self.sync.running = false;
            self.sync.cancel = None;
            self.sync.receiver = None;
            self.sync.progress = None;
        }
    }

    fn handle_sync_event(&mut self, event: SyncEvent) {
        match event {
            SyncEvent::Log(log) => {
                self.sync.logs.push(log);
                if self.sync.logs.len() > 2000 {
                    self.sync.logs.remove(0);
                }
            }
            SyncEvent::Progress(progress) => {
                self.sync.progress = Some(progress);
            }
            SyncEvent::Preview(preview) => {
                self.sync.preview = Some(preview);
            }
            SyncEvent::Finished(result) => {
                self.sync.running = false;
                self.sync.cancel = None;
                self.sync.progress = None;

                match result {
                    Ok(summary) => {
                        let failures = summary.failed_uploads + summary.failed_deletes;
                        let mode = self.sync.active_mode;
                        let message = if summary.cancelled {
                            "Sync cancelled.".to_string()
                        } else if failures > 0 {
                            format!("Sync finished with {} failure(s).", failures)
                        } else {
                            match mode {
                                Some(SyncMode::Preview) => format!(
                                    "Preview: {} upload(s), {} deletion(s).",
                                    summary.uploaded, summary.deleted
                                ),
                                Some(SyncMode::TestConnection) => {
                                    "Connection successful.".to_string()
                                }
                                Some(SyncMode::RefreshStaging) => format!(
                                    "Staging refreshed: {} copied, {} pruned.",
                                    summary.staged_copied, summary.staged_pruned
                                ),
                                Some(SyncMode::CheckWrapper) => {
                                    "Wrapper check complete.".to_string()
                                }
                                _ => format!(
                                    "Sync finished: {} uploaded, {} deleted.",
                                    summary.uploaded, summary.deleted
                                ),
                            }
                        };
                        self.status_message = Some((message, failures > 0));
                        self.sync.summary = Some(summary);
                    }
                    Err(e) => {
                        self.sync.logs.push(SyncLog {
                            level: LogLevel::Error,
                            message: e.clone(),
                        });
                        self.status_message = Some((format!("Sync failed: {}", e), true));
                    }
                }
            }
        }
    }

    fn start_sync(&mut self, mode: SyncMode, ctx: &egui::Context) {
        if self.sync.running {
            return;
        }

        let needs_connection = mode != SyncMode::RefreshStaging;
        if needs_connection {
            if self.sync.config.host.trim().is_empty() {
                self.sync_error("Server host cannot be empty.");
                return;
            }
            if self.sync.config.username.trim().is_empty() {
                self.sync_error("Server username cannot be empty.");
                return;
            }
            if self.sync.config.password.is_empty() {
                self.sync_error("Server password cannot be empty.");
                return;
            }
        }

        let refresh_staging = mode == SyncMode::RefreshStaging
            || (self.sync.config.auto_refresh_staging
                && matches!(
                    mode,
                    SyncMode::Preview
                        | SyncMode::SyncChanges
                        | SyncMode::ForceAll
                        | SyncMode::UploadOnly
                ));

        let mut source_bepinex = None;
        if refresh_staging {
            match self.sync_source_bepinex() {
                Some(source) if source.is_dir() => source_bepinex = Some(source),
                Some(source) => {
                    self.sync_error(format!(
                        "Source BepInEx folder not found: {}",
                        source.display()
                    ));
                    return;
                }
                None => {
                    self.sync_error("No profile selected. Choose one on the Profile & Info tab.");
                    return;
                }
            }
        }

        let staging = sync::expand_tilde(&self.sync.config.staging_dir);
        if staging.as_os_str().is_empty() {
            self.sync_error("Staging folder cannot be empty.");
            return;
        }

        if refresh_staging {
            if let Some(source) = source_bepinex.as_ref() {
                if sync::paths_overlap(source, &staging) {
                    self.sync_error("Staging folder must not overlap the source BepInEx folder.");
                    return;
                }
            }
        }

        let needs_staging = matches!(
            mode,
            SyncMode::Preview | SyncMode::SyncChanges | SyncMode::ForceAll | SyncMode::UploadOnly
        );
        if needs_staging && !refresh_staging && !staging.exists() {
            self.sync_error(format!(
                "Staging folder does not exist: {}",
                staging.display()
            ));
            return;
        }

        self.sync.logs.clear();
        self.sync.preview = None;
        self.sync.summary = None;
        self.sync.progress = None;

        if let Err(e) = self.sync.persist() {
            self.sync.logs.push(SyncLog {
                level: LogLevel::Warning,
                message: format!("Could not save sync settings: {}", e),
            });
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let request = SyncRequest {
            mode,
            config: self.sync.config.clone(),
            source_bepinex,
            ignore_patterns: sync::parse_ignore_text(&self.sync.ignore_text),
            refresh_staging,
            cancel: cancel.clone(),
        };

        let (sender, receiver) = mpsc::channel();
        let repaint_ctx = ctx.clone();
        let sink: sync::EventSink = Box::new(move |event| {
            let _ = sender.send(event);
            repaint_ctx.request_repaint();
        });

        self.sync.active_mode = Some(mode);
        self.sync.running = true;
        self.sync.cancel = Some(cancel);
        self.sync.receiver = Some(receiver);
        self.sync.logs.push(SyncLog {
            level: LogLevel::Info,
            message: format!("Starting {}...", mode.label()),
        });

        sync::spawn(request, sink);
        ctx.request_repaint();
    }
}

impl eframe::App for ModpackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keep active heartbeat so compositor restore/unminimize immediately repaints
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        self.poll_sync_events();

        // Top Navigation Bar
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new(self.language.app_title())
                        .color(Color32::from_rgb(230, 180, 50))
                        .strong(),
                );
                ui.label(RichText::new("v1.0.0").weak());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Language Switcher: [🌐 EN] / [🌐 ES]
                    ui.horizontal(|ui| {
                        let is_es = self.language == Language::Spanish;
                        let is_en = self.language == Language::English;

                        if ui.selectable_label(is_es, "🇪🇸 ES").clicked() && !is_es {
                            self.language = Language::Spanish;
                            self.sync.config.language = Some(Language::Spanish);
                            let _ = self.sync.config.save();
                        }
                        if ui.selectable_label(is_en, "🇺🇸 EN").clicked() && !is_en {
                            self.language = Language::English;
                            self.sync.config.language = Some(Language::English);
                            let _ = self.sync.config.save();
                        }
                    });

                    ui.separator();

                    if let Some(ref res) = self.last_build_result {
                        if ui.button(self.language.btn_open_output()).clicked() {
                            let _ = opener::reveal(&res.zip_path);
                        }
                    }
                    if ui
                        .button(
                            RichText::new(self.language.btn_build_quick())
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .on_hover_text("Build Thunderstore zip archive now")
                        .clicked()
                    {
                        self.trigger_build();
                    }
                });
            });
            ui.add_space(4.0);

            // Tab bar
            ui.horizontal_wrapped(|ui| {
                let tabs = [
                    (ActiveTab::ProfileMetadata, self.language.tab_profile().to_string()),
                    (
                        ActiveTab::Mods,
                        format!(
                            "{} ({})",
                            self.language.tab_mods(),
                            self.mods.iter().filter(|m| m.selected).count()
                        ),
                    ),
                    (
                        ActiveTab::Configs,
                        format!(
                            "{} ({})",
                            self.language.tab_configs(),
                            self.configs.iter().filter(|c| c.selected).count()
                        ),
                    ),
                    (ActiveTab::Build, self.language.tab_build().to_string()),
                    (ActiveTab::ServerSync, self.language.tab_server_sync().to_string()),
                    (ActiveTab::Help, self.language.tab_help().to_string()),
                ];

                for (tab, label) in tabs {
                    let is_active = self.active_tab == tab;
                    if ui
                        .selectable_label(is_active, RichText::new(label).strong())
                        .clicked()
                    {
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
                    ui.label(
                        RichText::new(self.language.status_summary(
                            self.mod_manager.label(),
                            &self.selected_profile,
                            self.mods.iter().filter(|m| m.selected).count(),
                            self.configs.iter().filter(|c| c.selected).count(),
                        ))
                        .weak(),
                    );
                }
            });
        });

        // Central Panel
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            ActiveTab::ProfileMetadata => self.ui_profile_metadata(ui),
            ActiveTab::Mods => self.ui_mods_selection(ui),
            ActiveTab::Configs => self.ui_configs_selection(ui),
            ActiveTab::Build => self.ui_build_export(ui),
            ActiveTab::ServerSync => self.ui_server_sync(ui),
            ActiveTab::Help => self.ui_help(ui),
        });
    }
}

impl ModpackApp {
    fn ui_profile_metadata(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading(self.language.heading_mod_manager());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.language.label_mod_manager()).strong());

                    if ui
                        .selectable_label(self.mod_manager == ModManager::R2Modman, "r2modman")
                        .clicked()
                    {
                        self.mod_manager = ModManager::R2Modman;
                        let roots = get_r2modman_roots();
                        self.selected_root = roots.first().cloned();
                        self.refresh_profiles();
                    }

                    if ui
                        .selectable_label(self.mod_manager == ModManager::Gale, "Gale")
                        .clicked()
                    {
                        self.mod_manager = ModManager::Gale;
                        let roots = get_gale_roots();
                        self.selected_root = roots.first().cloned();
                        self.refresh_profiles();
                    }

                    if ui
                        .selectable_label(self.mod_manager == ModManager::Custom, self.language.label_custom_path())
                        .clicked()
                    {
                        self.mod_manager = ModManager::Custom;
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(self.language.label_profiles_dir());
                    let current_dir_display = self
                        .selected_root
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| self.language.not_found().to_string());
                    ui.monospace(current_dir_display);

                    if ui.button(self.language.btn_browse()).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.selected_root = Some(folder);
                            self.mod_manager = ModManager::Custom;
                            self.refresh_profiles();
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(self.language.label_selected_profile());
                    let current_profile = self.selected_profile.clone();
                    let mut changed_profile = None;
                    egui::ComboBox::from_id_salt("profile_select")
                        .selected_text(&current_profile)
                        .show_ui(ui, |ui| {
                            for p in &self.available_profiles {
                                if ui
                                    .selectable_label(self.selected_profile == *p, p)
                                    .clicked()
                                {
                                    changed_profile = Some(p.clone());
                                }
                            }
                        });
                    if let Some(p) = changed_profile {
                        self.selected_profile = p;
                        self.load_selected_profile_data();
                    }

                    if ui.button(self.language.btn_reload()).clicked() {
                        self.refresh_profiles();
                    }

                    let enabled_count = self.mods.iter().filter(|m| m.enabled).count();
                    ui.label(
                        RichText::new(self.language.label_profile_stats(self.mods.len(), enabled_count))
                            .weak(),
                    );
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.heading(self.language.heading_metadata());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.language.heading_output_dir()).strong());
                    let out_display = self.output_dir.display().to_string();
                    ui.monospace(out_display);
                    if ui.button(self.language.btn_browse()).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.output_dir = folder;
                        }
                    }
                });
                ui.label(RichText::new(self.language.label_output_dir_desc()).weak());

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(self.language.label_package_name());
                    let mut name = self.pack_name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        self.pack_name =
                            name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                    }
                    ui.label(RichText::new("(Letters, numbers, underscores)").weak());
                });

                ui.horizontal(|ui| {
                    ui.label(self.language.label_version());
                    ui.text_edit_singleline(&mut self.pack_version);
                    ui.label(RichText::new("(SemVer X.Y.Z, e.g. 1.0.0)").weak());
                });

                ui.horizontal(|ui| {
                    ui.label(self.language.label_description());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.pack_description).desired_width(400.0),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(self.language.label_website());
                    ui.add(egui::TextEdit::singleline(&mut self.pack_website).desired_width(400.0));
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(self.language.label_icon());
                    if let Some(ref icon) = self.custom_icon_path {
                        ui.label(
                            RichText::new(icon.file_name().unwrap_or_default().to_string_lossy())
                                .strong(),
                        );
                        if ui.button(self.language.btn_remove()).clicked() {
                            self.custom_icon_path = None;
                        }
                    } else {
                        ui.label(
                            RichText::new(self.language.label_default_icon())
                                .color(Color32::from_rgb(230, 180, 50)),
                        );
                        if ui.button(self.language.btn_browse()).clicked() {
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
                if ui
                    .button(RichText::new(self.language.btn_proceed_to_mods()).strong())
                    .clicked()
                {
                    self.active_tab = ActiveTab::Mods;
                }
            });
        });
    }

    fn ui_mods_selection(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.language.heading_mods());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.language.btn_deselect_all()).clicked() {
                    for m in &mut self.mods {
                        m.selected = false;
                    }
                }
                if ui.button(self.language.btn_select_enabled()).clicked() {
                    for m in &mut self.mods {
                        m.selected = m.enabled;
                    }
                }
                if ui.button(self.language.btn_select_all()).clicked() {
                    for m in &mut self.mods {
                        m.selected = true;
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.mod_search)
                    .hint_text(self.language.hint_search_mods())
                    .desired_width(260.0),
            );
            if !self.mod_search.is_empty() && ui.button("❌").clicked() {
                self.mod_search.clear();
            }
            let selected_count = self.mods.iter().filter(|m| m.selected).count();
            ui.label(
                RichText::new(self.language.label_mods_selected(selected_count, self.mods.len()))
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new(self.language.btn_proceed_to_configs()).strong()).clicked() {
                    self.active_tab = ActiveTab::Configs;
                }
            });
        });
        ui.separator();

        let search = self.mod_search.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("mods_table")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("✓").strong());
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
                        let ver_edit = ui.add(
                            egui::TextEdit::singleline(&mut m.current_version).desired_width(70.0),
                        );
                        if m.current_version != m.original_version {
                            ver_edit
                                .on_hover_text(format!("Original version: {}", m.original_version));
                        }

                        ui.monospace(&m.name);
                        ui.end_row();
                    }
                });
        });
    }

    fn ui_configs_selection(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.language.heading_configs());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.language.btn_deselect_all()).clicked() {
                    for c in &mut self.configs {
                        c.selected = false;
                    }
                }
                if ui.button("Select Safe Only (Recommended)").clicked() {
                    for c in &mut self.configs {
                        c.selected = c.category == ConfigCategory::Safe;
                    }
                }
                if ui.button(self.language.btn_select_all()).clicked() {
                    for c in &mut self.configs {
                        c.selected = true;
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.config_search)
                    .hint_text(self.language.hint_search_configs())
                    .desired_width(260.0),
            );
            if !self.config_search.is_empty() && ui.button("❌").clicked() {
                self.config_search.clear();
            }
            let selected_count = self.configs.iter().filter(|c| c.selected).count();
            ui.label(
                RichText::new(self.language.label_configs_selected(selected_count, self.configs.len()))
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new(self.language.btn_proceed_to_build()).strong()).clicked() {
                    self.active_tab = ActiveTab::Build;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("🟢 Safe").color(Color32::from_rgb(100, 220, 100)));
            ui.label("|");
            ui.label(
                RichText::new("🔴 Sensitive / Passwords (Auto-excluded)")
                    .color(Color32::from_rgb(255, 100, 100)),
            );
            ui.label("|");
            ui.label(
                RichText::new("🟡 Runtime Cache / Reports (Auto-excluded)")
                    .color(Color32::from_rgb(240, 200, 80)),
            );
        });
        ui.separator();

        let search = self.config_search.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("configs_table")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("✓").strong());
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
                ui.heading(self.language.heading_build());
                ui.separator();

                let selected_mods_count = self.mods.iter().filter(|m| m.selected).count();
                let selected_configs_count = self.configs.iter().filter(|c| c.selected).count();

                ui.label(
                    RichText::new(format!(
                        "Package: {} v{}",
                        self.pack_name, self.pack_version
                    ))
                    .heading(),
                );
                ui.label(format!(
                    "• Source: {} ({})",
                    self.mod_manager.label(),
                    self.selected_profile
                ));
                ui.label(format!(
                    "• Mods to include: {} dependencies",
                    selected_mods_count
                ));
                ui.label(format!(
                    "• Configurations to include: {} files/folders",
                    selected_configs_count
                ));

                ui.horizontal(|ui| {
                    ui.label(format!("• Output Directory: {}", self.output_dir.display()));
                    if ui.button(self.language.btn_browse()).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.output_dir = folder;
                        }
                    }
                });
                ui.label(format!(
                    "• Target ZIP: {}/{}-{}.zip",
                    self.output_dir.display(),
                    self.pack_name,
                    self.pack_version
                ));

                ui.add_space(8.0);
                if ui
                    .button(
                        RichText::new(self.language.btn_build_main())
                            .heading()
                            .color(Color32::WHITE),
                    )
                    .clicked()
                {
                    self.trigger_build();
                }
            });

            if let Some(ref res) = self.last_build_result {
                ui.add_space(16.0);
                ui.group(|ui| {
                    ui.heading(
                        RichText::new("🎉 Build Successful!").color(Color32::from_rgb(90, 220, 90)),
                    );
                    ui.separator();
                    ui.label(format!("{}: {}", self.language.label_zip_archive(), res.zip_path.display()));
                    ui.label(format!(
                        "Size: {:.1} KB",
                        (res.zip_size_bytes as f64) / 1024.0
                    ));
                    ui.label(format!("{}: {}", self.language.label_unpacked_folder(), res.build_dir.display()));

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(self.language.btn_open_folder()).clicked() {
                            let _ = opener::reveal(&res.zip_path);
                        }
                    });
                });
            }
        });
    }

    fn ui_server_sync(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading(self.language.sync_title());
            ui.label(self.language.sync_desc());

            if let Some(warning) = self.sync.warning.clone() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(230, 180, 50), warning);
                    if ui.small_button(self.language.btn_dismiss()).clicked() {
                        self.sync.warning = None;
                    }
                });
            }

            let source = self.sync_source_bepinex();
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading(self.language.heading_source_profile());
                ui.separator();
                match source {
                    Some(ref path) => {
                        ui.label(format!("Profile BepInEx: {}", path.display()));
                        if path.is_dir() {
                            ui.colored_label(
                                Color32::from_rgb(90, 220, 90),
                                self.language.source_folder_found(),
                            );
                        } else {
                            ui.colored_label(
                                Color32::from_rgb(220, 90, 90),
                                self.language.source_folder_missing(),
                            );
                        }
                    }
                    None => {
                        ui.colored_label(
                            Color32::from_rgb(220, 90, 90),
                            self.language.no_profile_selected(),
                        );
                    }
                }
            });

            ui.add_space(12.0);
            ui.group(|ui| {
                ui.heading(self.language.heading_connection());
                ui.separator();
                egui::Grid::new("sync_connection_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(self.language.label_host());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sync.config.host)
                                .desired_width(320.0)
                                .hint_text("example.com or 192.168.1.10"),
                        );
                        ui.end_row();

                        ui.label(self.language.label_port());
                        ui.add(
                            egui::DragValue::new(&mut self.sync.config.port)
                                .range(1..=65535),
                        );
                        ui.end_row();

                        ui.label(self.language.label_username());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sync.config.username)
                                .desired_width(320.0),
                        );
                        ui.end_row();

                        ui.label(self.language.label_password());
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.sync.config.password)
                                    .password(!self.sync.show_password)
                                    .desired_width(320.0),
                            );
                            ui.checkbox(&mut self.sync.show_password, self.language.label_show_password());
                        });
                        ui.end_row();

                        ui.label(self.language.label_staging_dir());
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.sync.config.staging_dir)
                                    .desired_width(320.0),
                            );
                            if ui.button(self.language.btn_browse()).clicked() {
                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                    self.sync.config.staging_dir = folder.display().to_string();
                                    self.sync.reload_ignore();
                                }
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(4.0);
                ui.checkbox(
                    &mut self.sync.config.auto_refresh_staging,
                    self.language.chk_auto_refresh(),
                );
                ui.label(
                    RichText::new(self.language.desc_auto_refresh())
                        .weak(),
                );

                ui.add_space(6.0);
                if ui.button(self.language.btn_save_settings()).clicked() {
                    match self.sync.persist() {
                        Ok(()) => {
                            self.status_message =
                                Some(("Server sync settings saved.".to_string(), false));
                        }
                        Err(e) => self.sync_error(format!("Could not save settings: {}", e)),
                    }
                }
            });

            ui.add_space(12.0);
            ui.group(|ui| {
                ui.heading(self.language.heading_ignore());
                ui.label(self.language.desc_ignore());
                ui.add(
                    egui::TextEdit::multiline(&mut self.sync.ignore_text)
                        .desired_rows(7)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
                ui.horizontal(|ui| {
                    if ui.button(self.language.btn_reset_defaults()).clicked() {
                        self.sync.ignore_text = format!(
                            "{}\n",
                            DEFAULT_IGNORED_PATTERNS.join("\n")
                        );
                    }
                    if ui.button(self.language.btn_reload_staging()).clicked() {
                        self.sync.reload_ignore();
                    }
                    if ui.button(self.language.btn_save_ignore()).clicked() {
                        match self.sync.persist() {
                            Ok(()) => {
                                self.status_message =
                                    Some(("Ignore file saved.".to_string(), false));
                            }
                            Err(e) => self.sync_error(format!("Could not save ignore file: {}", e)),
                        }
                    }
                });
            });

            ui.add_space(12.0);
            ui.group(|ui| {
                ui.heading(self.language.heading_actions());
                ui.separator();

                let running = self.sync.running;
                ui.add_enabled_ui(!running, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(self.language.btn_action_preview())
                            .on_hover_text(self.language.tip_action_preview())
                            .clicked()
                        {
                            self.start_sync(SyncMode::Preview, ui.ctx());
                        }
                        if ui
                            .button(self.language.btn_action_sync())
                            .on_hover_text(self.language.tip_action_sync())
                            .clicked()
                        {
                            self.start_sync(SyncMode::SyncChanges, ui.ctx());
                        }
                        if ui
                            .button(self.language.btn_action_force())
                            .on_hover_text(self.language.tip_action_force())
                            .clicked()
                        {
                            self.start_sync(SyncMode::ForceAll, ui.ctx());
                        }
                        if ui
                            .button(self.language.btn_action_upload_only())
                            .on_hover_text(self.language.tip_action_upload_only())
                            .clicked()
                        {
                            self.start_sync(SyncMode::UploadOnly, ui.ctx());
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(self.language.btn_action_wrapper())
                            .on_hover_text(self.language.tip_action_wrapper())
                            .clicked()
                        {
                            self.start_sync(SyncMode::CheckWrapper, ui.ctx());
                        }
                        if ui
                            .button(self.language.btn_action_refresh())
                            .on_hover_text(self.language.tip_action_refresh())
                            .clicked()
                        {
                            self.start_sync(SyncMode::RefreshStaging, ui.ctx());
                        }
                        if ui
                            .button(self.language.btn_action_test())
                            .on_hover_text(self.language.tip_action_test())
                            .clicked()
                        {
                            self.start_sync(SyncMode::TestConnection, ui.ctx());
                        }
                    });
                });

                if running {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Sync running...");
                        let cancellable = matches!(
                            self.sync.active_mode,
                            Some(SyncMode::SyncChanges | SyncMode::ForceAll | SyncMode::UploadOnly)
                        );
                        if cancellable {
                            if let Some(cancel) = &self.sync.cancel {
                                if ui.button("⏹ Cancel").clicked() {
                                    cancel.store(true, Ordering::Relaxed);
                                }
                            }
                        }
                    });
                }

                if let Some(progress) = &self.sync.progress {
                    let fraction = if progress.total == 0 {
                        0.0
                    } else {
                        progress.current as f32 / progress.total as f32
                    };
                    ui.add(
                        egui::ProgressBar::new(fraction).text(format!(
                            "{} ({}/{})",
                            progress.label, progress.current, progress.total
                        )),
                    );
                }
            });

            // Modern Preview Section (UI/UX Redesign)
            self.ui_sync_preview_dashboard(ui);

            // Modern Execution Summary (UI/UX Redesign)
            self.ui_sync_execution_summary(ui);

            // Modern Terminal Logs Console (UI/UX Redesign)
            self.ui_sync_logs_console(ui);
        });
    }

    // Modern Preview Dashboard: Stat Tiles & Interactive Changes Browser
    fn ui_sync_preview_dashboard(&mut self, ui: &mut Ui) {
        let preview = match &self.sync.preview {
            Some(p) => p.clone(),
            None => return,
        };

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(self.language.heading_preview());
            ui.separator();

            // 1. Stat Cards Row
            ui.horizontal_wrapped(|ui| {
                // Uploads Badge Card
                egui::Frame::group(ui.style())
                    .rounding(egui::Rounding::same(6.0))
                    .fill(Color32::from_rgb(20, 36, 26))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(45, 140, 75)))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⬆").size(24.0).color(Color32::from_rgb(90, 220, 110)));
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(format!("{}", preview.uploads.len()))
                                        .heading()
                                        .strong()
                                        .color(Color32::from_rgb(90, 220, 110)),
                                );
                                ui.label(RichText::new(self.language.badge_uploads()).small().weak());
                            });
                        });
                    });

                // Deletions Badge Card
                egui::Frame::group(ui.style())
                    .rounding(egui::Rounding::same(6.0))
                    .fill(Color32::from_rgb(38, 22, 22))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(180, 50, 50)))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🗑").size(24.0).color(Color32::from_rgb(240, 90, 90)));
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(format!("{}", preview.deletions.len()))
                                        .heading()
                                        .strong()
                                        .color(Color32::from_rgb(240, 90, 90)),
                                );
                                ui.label(RichText::new(self.language.badge_deletions()).small().weak());
                            });
                        });
                    });

                // File Counts Card
                egui::Frame::group(ui.style())
                    .rounding(egui::Rounding::same(6.0))
                    .fill(Color32::from_rgb(24, 26, 32))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("📁").size(24.0));
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(format!("{}/{}", preview.local_count, preview.remote_count))
                                        .heading()
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} / {}",
                                        self.language.badge_local(),
                                        self.language.badge_remote()
                                    ))
                                    .small()
                                    .weak(),
                                );
                            });
                        });
                    });

                // Server Wrapper Card
                egui::Frame::group(ui.style())
                    .rounding(egui::Rounding::same(6.0))
                    .fill(Color32::from_rgb(24, 26, 32))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🛡").size(24.0));
                            ui.vertical(|ui| {
                                let (status_text, status_color) = match preview.wrapper.status {
                                    WrapperStatus::Intact => (
                                        self.language.wrapper_intact(),
                                        Color32::from_rgb(90, 220, 90),
                                    ),
                                    WrapperStatus::ElfNeedsRename => (
                                        self.language.wrapper_rename_needed(),
                                        Color32::from_rgb(240, 180, 50),
                                    ),
                                    _ => (
                                        self.language.wrapper_missing(),
                                        Color32::from_rgb(240, 80, 80),
                                    ),
                                };
                                ui.label(RichText::new(status_text).strong().color(status_color));
                                let sym_text = if preview.wrapper.symlink_present {
                                    self.language.symlink_ok()
                                } else {
                                    self.language.symlink_missing()
                                };
                                ui.label(RichText::new(sym_text).small().weak());
                            });
                        });
                    });
            });

            // Warnings from wrapper (if any)
            for warning in &preview.wrapper.warnings {
                ui.add_space(2.0);
                ui.colored_label(Color32::from_rgb(230, 180, 50), format!("⚠️ {}", warning));
            }

            ui.add_space(8.0);

            // 2. Interactive Changes Browser
            let all_count = preview.uploads.len() + preview.deletions.len();
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        self.sync.preview_tab == PreviewTab::All,
                        self.language.tab_all_changes(all_count),
                    )
                    .clicked()
                {
                    self.sync.preview_tab = PreviewTab::All;
                }
                if ui
                    .selectable_label(
                        self.sync.preview_tab == PreviewTab::Uploads,
                        self.language.tab_uploads(preview.uploads.len()),
                    )
                    .clicked()
                {
                    self.sync.preview_tab = PreviewTab::Uploads;
                }
                if ui
                    .selectable_label(
                        self.sync.preview_tab == PreviewTab::Deletions,
                        self.language.tab_deletions(preview.deletions.len()),
                    )
                    .clicked()
                {
                    self.sync.preview_tab = PreviewTab::Deletions;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.sync.preview_search.is_empty() && ui.small_button("✕").clicked() {
                        self.sync.preview_search.clear();
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.sync.preview_search)
                            .hint_text(self.language.hint_search_changes())
                            .desired_width(220.0),
                    );
                    ui.label("🔍");
                });
            });

            ui.add_space(4.0);

            if all_count == 0 {
                egui::Frame::none()
                    .fill(Color32::from_rgb(18, 30, 22))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(self.language.msg_no_changes())
                                .color(Color32::from_rgb(90, 220, 110))
                                .strong(),
                        );
                    });
            } else {
                let search = self.sync.preview_search.to_lowercase();
                let items_to_show: Vec<(bool, &String)> = match self.sync.preview_tab {
                    PreviewTab::All => {
                        let mut list = Vec::new();
                        for u in &preview.uploads {
                            list.push((true, u));
                        }
                        for d in &preview.deletions {
                            list.push((false, d));
                        }
                        list
                    }
                    PreviewTab::Uploads => preview.uploads.iter().map(|u| (true, u)).collect(),
                    PreviewTab::Deletions => preview.deletions.iter().map(|d| (false, d)).collect(),
                };

                let filtered: Vec<_> = items_to_show
                    .into_iter()
                    .filter(|(_, path)| search.is_empty() || path.to_lowercase().contains(&search))
                    .collect();

                if filtered.is_empty() {
                    ui.label(RichText::new(self.language.msg_no_changes_filter()).weak());
                } else {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(16, 18, 24))
                        .rounding(egui::Rounding::same(6.0))
                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(32, 36, 46)))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(220.0)
                                .show(ui, |ui| {
                                    for (is_upload, path) in filtered {
                                        ui.horizontal(|ui| {
                                            if is_upload {
                                                ui.label(
                                                    RichText::new(" + ADD ")
                                                        .size(10.0)
                                                        .color(Color32::BLACK)
                                                        .background_color(Color32::from_rgb(90, 220, 110))
                                                        .strong(),
                                                );
                                            } else {
                                                ui.label(
                                                    RichText::new(" - DEL ")
                                                        .size(10.0)
                                                        .color(Color32::WHITE)
                                                        .background_color(Color32::from_rgb(220, 60, 60))
                                                        .strong(),
                                                );
                                            }

                                            // Path with dimmed directory and bold filename
                                            if let Some((dir, filename)) = path.rsplit_once('/') {
                                                ui.label(RichText::new(format!("{}/", dir)).weak().monospace());
                                                ui.label(RichText::new(filename).monospace().strong());
                                            } else {
                                                ui.label(RichText::new(path.as_str()).monospace().strong());
                                            }

                                            // Category tag on the far right
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let ext = path
                                                        .split('.')
                                                        .last()
                                                        .unwrap_or("")
                                                        .to_lowercase();
                                                    let (tag, color) = match ext.as_str() {
                                                        "dll" => ("PLUGIN", Color32::from_rgb(80, 160, 240)),
                                                        "cfg" | "json" | "yaml" | "yml" => (
                                                            "CONFIG",
                                                            Color32::from_rgb(230, 180, 70),
                                                        ),
                                                        "png" | "jpg" | "asset" => (
                                                            "ASSET",
                                                            Color32::from_rgb(160, 120, 230),
                                                        ),
                                                        _ => ("FILE", Color32::from_rgb(140, 140, 140)),
                                                    };
                                                    ui.label(
                                                        RichText::new(tag)
                                                            .size(9.0)
                                                            .color(color)
                                                            .monospace(),
                                                    );
                                                },
                                            );
                                        });
                                    }
                                });
                        });
                }
            }
        });
    }

    // Modern Execution Summary Banner
    fn ui_sync_execution_summary(&mut self, ui: &mut Ui) {
        if self.sync.active_mode == Some(SyncMode::Preview) {
            return;
        }

        if let Some(summary) = &self.sync.summary {
            ui.add_space(10.0);
            let (bg_color, stroke_color, title) = if summary.cancelled {
                (
                    Color32::from_rgb(34, 30, 20),
                    Color32::from_rgb(200, 160, 40),
                    self.language.sync_cancelled_title().to_string(),
                )
            } else if summary.failed_uploads > 0 || summary.failed_deletes > 0 {
                (
                    Color32::from_rgb(36, 22, 22),
                    Color32::from_rgb(210, 60, 60),
                    self.language.sync_failed_title().to_string(),
                )
            } else {
                (
                    Color32::from_rgb(20, 34, 24),
                    Color32::from_rgb(50, 160, 80),
                    self.language.sync_completed_title(summary.elapsed_secs),
                )
            };

            egui::Frame::group(ui.style())
                .rounding(egui::Rounding::same(6.0))
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0_f32, stroke_color))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(RichText::new(title).heading().strong());
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("• Uploaded: {}", summary.uploaded));
                        ui.label(format!("• Deleted: {}", summary.deleted));
                        if summary.failed_uploads > 0 {
                            ui.colored_label(
                                Color32::from_rgb(255, 90, 90),
                                format!("• Failed uploads: {}", summary.failed_uploads),
                            );
                        }
                        if summary.failed_deletes > 0 {
                            ui.colored_label(
                                Color32::from_rgb(255, 90, 90),
                                format!("• Failed deletes: {}", summary.failed_deletes),
                            );
                        }
                        ui.label(format!("• Staged copies: {}", summary.staged_copied));
                        ui.label(format!("• Staged prunes: {}", summary.staged_pruned));
                        ui.label(format!("• Elapsed: {:.1}s", summary.elapsed_secs));
                    });
                });
        }
    }

    // Modern Terminal Logs Console
    fn ui_sync_logs_console(&mut self, ui: &mut Ui) {
        ui.add_space(12.0);
        ui.group(|ui| {
            let total_logs = self.sync.logs.len();
            let err_count = self
                .sync
                .logs
                .iter()
                .filter(|l| l.level == LogLevel::Error)
                .count();
            let warn_count = self
                .sync
                .logs
                .iter()
                .filter(|l| l.level == LogLevel::Warning)
                .count();
            let succ_count = self
                .sync
                .logs
                .iter()
                .filter(|l| l.level == LogLevel::Success)
                .count();
            let info_count = self
                .sync
                .logs
                .iter()
                .filter(|l| l.level == LogLevel::Info)
                .count();

            // Toolbar
            ui.horizontal(|ui| {
                ui.heading(self.language.heading_logs());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(self.language.btn_clear_logs()).clicked() {
                        self.sync.logs.clear();
                    }

                    // Copy logs button
                    if ui.small_button(self.language.btn_copy_logs()).clicked() {
                        let text: String = self
                            .sync
                            .logs
                            .iter()
                            .map(|l| {
                                let tag = match l.level {
                                    LogLevel::Info => "[INFO]",
                                    LogLevel::Success => "[OK]",
                                    LogLevel::Warning => "[WARN]",
                                    LogLevel::Error => "[ERROR]",
                                };
                                format!("{} {}\n", tag, l.message)
                            })
                            .collect();
                        ui.ctx().copy_text(text);
                        self.sync.copied_feedback = Some(Instant::now());
                    }

                    // Copy feedback text
                    if let Some(t) = self.sync.copied_feedback {
                        if t.elapsed().as_secs_f32() < 2.0 {
                            ui.colored_label(
                                Color32::from_rgb(90, 220, 110),
                                self.language.msg_logs_copied(),
                            );
                        } else {
                            self.sync.copied_feedback = None;
                        }
                    }

                    ui.checkbox(&mut self.sync.log_autoscroll, self.language.chk_autoscroll());
                });
            });

            // Filter chips & search row
            ui.horizontal(|ui| {
                let is_all = self.sync.log_filter.is_none();
                if ui
                    .selectable_label(is_all, self.language.log_filter_all(total_logs))
                    .clicked()
                {
                    self.sync.log_filter = None;
                }

                if err_count > 0 || self.sync.log_filter == Some(LogLevel::Error) {
                    let is_err = self.sync.log_filter == Some(LogLevel::Error);
                    let err_chip = RichText::new(self.language.log_filter_errors(err_count))
                        .color(if err_count > 0 {
                            Color32::from_rgb(255, 90, 90)
                        } else {
                            Color32::GRAY
                        });
                    if ui.selectable_label(is_err, err_chip).clicked() {
                        self.sync.log_filter = if is_err { None } else { Some(LogLevel::Error) };
                    }
                }

                if warn_count > 0 || self.sync.log_filter == Some(LogLevel::Warning) {
                    let is_warn = self.sync.log_filter == Some(LogLevel::Warning);
                    let warn_chip = RichText::new(self.language.log_filter_warnings(warn_count))
                        .color(if warn_count > 0 {
                            Color32::from_rgb(240, 180, 50)
                        } else {
                            Color32::GRAY
                        });
                    if ui.selectable_label(is_warn, warn_chip).clicked() {
                        self.sync.log_filter = if is_warn {
                            None
                        } else {
                            Some(LogLevel::Warning)
                        };
                    }
                }

                let is_succ = self.sync.log_filter == Some(LogLevel::Success);
                if ui
                    .selectable_label(is_succ, self.language.log_filter_success(succ_count))
                    .clicked()
                {
                    self.sync.log_filter = if is_succ {
                        None
                    } else {
                        Some(LogLevel::Success)
                    };
                }

                let is_info = self.sync.log_filter == Some(LogLevel::Info);
                if ui
                    .selectable_label(is_info, self.language.log_filter_info(info_count))
                    .clicked()
                {
                    self.sync.log_filter = if is_info { None } else { Some(LogLevel::Info) };
                }

                // Search
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.sync.log_search.is_empty() && ui.small_button("✕").clicked() {
                        self.sync.log_search.clear();
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.sync.log_search)
                            .hint_text(self.language.hint_search_logs())
                            .desired_width(180.0),
                    );
                    ui.label("🔍");
                });
            });

            ui.separator();

            if self.sync.logs.is_empty() {
                ui.label(RichText::new(self.language.no_logs_yet()).weak());
            } else {
                let search = self.sync.log_search.to_lowercase();
                let active_level = self.sync.log_filter;

                let filtered_logs: Vec<_> = self
                    .sync
                    .logs
                    .iter()
                    .filter(|l| {
                        if let Some(lvl) = active_level {
                            if l.level != lvl {
                                return false;
                            }
                        }
                        if !search.is_empty() && !l.message.to_lowercase().contains(&search) {
                            return false;
                        }
                        true
                    })
                    .collect();

                if filtered_logs.is_empty() {
                    ui.label(RichText::new(self.language.no_logs_matching()).weak());
                } else {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(14, 16, 22))
                        .rounding(egui::Rounding::same(6.0))
                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(30, 34, 44)))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            let mut scroll = egui::ScrollArea::vertical().max_height(240.0);
                            if self.sync.log_autoscroll {
                                scroll = scroll.stick_to_bottom(true);
                            }
                            scroll.show(ui, |ui| {
                                for log in filtered_logs {
                                    ui.horizontal(|ui| {
                                        let (tag, tag_bg, tag_fg) = match log.level {
                                            LogLevel::Info => (
                                                "INFO",
                                                Color32::from_rgb(40, 50, 70),
                                                Color32::from_rgb(170, 190, 220),
                                            ),
                                            LogLevel::Success => (
                                                " OK ",
                                                Color32::from_rgb(20, 60, 30),
                                                Color32::from_rgb(100, 230, 130),
                                            ),
                                            LogLevel::Warning => (
                                                "WARN",
                                                Color32::from_rgb(70, 50, 15),
                                                Color32::from_rgb(250, 200, 80),
                                            ),
                                            LogLevel::Error => (
                                                "ERR!",
                                                Color32::from_rgb(75, 20, 20),
                                                Color32::from_rgb(255, 110, 110),
                                            ),
                                        };

                                        ui.label(
                                            RichText::new(tag)
                                                .size(10.0)
                                                .color(tag_fg)
                                                .background_color(tag_bg)
                                                .monospace()
                                                .strong(),
                                        );

                                        let msg_color = match log.level {
                                            LogLevel::Info => Color32::from_rgb(195, 200, 210),
                                            LogLevel::Success => Color32::from_rgb(130, 230, 150),
                                            LogLevel::Warning => Color32::from_rgb(240, 200, 100),
                                            LogLevel::Error => Color32::from_rgb(255, 120, 120),
                                        };

                                        ui.label(
                                            RichText::new(&log.message)
                                                .size(12.0)
                                                .color(msg_color)
                                                .monospace(),
                                        );
                                    });
                                }
                            });
                        });
                }
            }
        });
    }

    fn ui_help(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading(self.language.help_title());
            ui.separator();

            for (section_title, points) in self.language.help_content() {
                ui.add_space(8.0);
                ui.label(RichText::new(*section_title).strong());
                for point in *points {
                    ui.label(*point);
                }
            }
        });
    }
}

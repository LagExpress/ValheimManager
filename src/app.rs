use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

use crate::appearance::{Palette, ThemePreset, UiPreferences};
use crate::i18n::Language;
use crate::modpack_config::{ModpackConfiguration, ModpackLibrary, SavedModChoice};
use crate::packager::{build_modpack, BuildResult, ModpackOptions};
use crate::platform_effects::PlatformEffects;
use crate::profile::{
    get_gale_roots, get_r2modman_roots, list_profiles_in_dir, load_mods_from_profile,
    scan_profile_configs, ConfigCategory, ConfigItem, ModItem, ModManager,
};
use crate::sftp::WrapperStatus;
use crate::sync::{
    self, LogLevel, PreviewResult, Progress, SyncConfig, SyncEvent, SyncLog, SyncMode, SyncRequest,
    SyncSummary, DEFAULT_IGNORED_PATTERNS, IGNORE_FILE_NAME,
};
use crate::ui_components::{self as components, Icon};
use eframe::egui::{self, Color32, RichText, Ui};

mod saved;
use saved::{BuildAttempt, BuildVersionPrompt};
mod readme;
use readme::ReadmePreview;

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
    pub logs: VecDeque<SyncLog>,
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
    pub preview_revision: u64,
    pub logs_revision: u64,
}

impl SyncUiState {
    fn push_log(&mut self, log: SyncLog) {
        self.logs.push_back(log);
        if self.logs.len() > 2000 {
            self.logs.pop_front();
        }
        self.logs_revision = self.logs_revision.wrapping_add(1);
    }

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
            logs: VecDeque::new(),
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
            preview_revision: 0,
            logs_revision: 0,
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
    modpack_library: ModpackLibrary,
    modpack_library_path: PathBuf,
    modpack_library_error: Option<String>,
    always_increment_version: bool,
    build_version_prompt: Option<BuildVersionPrompt>,
    build_attempt: Option<BuildAttempt>,
    preferences: UiPreferences,
    theme: ThemePreset,
    effects: PlatformEffects,
    applied_theme: Option<(ThemePreset, bool)>,
    initial_size_set: bool,
    open_receiver: Option<Receiver<Result<(), String>>>,
    data_revision: u64,
    mod_filter: FilterCache,
    config_filter: FilterCache,
    preview_filter: FilterCache,
    logs_filter: FilterCache,
    log_layout: LogLayoutCache,
    readme_preview: ReadmePreview,
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
    pub documentation: crate::package_docs::PackageDocumentation,
    pub pack_website: String,
    pub custom_icon_path: Option<PathBuf>,
    pub output_dir: PathBuf,

    // UI State
    pub active_tab: ActiveTab,
    pub status_message: Option<(String, bool)>, // (message, is_error)
    pub last_build_result: Option<BuildResult>,

    // Compiling / Build Progress State
    pub building: bool,
    pub build_progress: Option<(f32, String)>,
    pub build_receiver: Option<Receiver<BuildEvent>>,

    // Server Sync
    pub sync: SyncUiState,
}

pub enum BuildEvent {
    Progress(f32, String),
    Finished(Result<BuildResult, String>),
}

#[derive(Default)]
struct FilterCache {
    key: Option<(String, u64)>,
    indices: Vec<usize>,
}

type LogLayoutKey = (String, u64, u32, u32, egui::FontId);

#[derive(Default)]
struct LogLayoutCache {
    context: Option<egui::Context>,
    key: Option<LogLayoutKey>,
    galleys: Vec<Arc<egui::Galley>>,
    offsets: Vec<f32>,
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
            modpack_library: ModpackLibrary::default(),
            modpack_library_path: ModpackLibrary::path(),
            modpack_library_error: None,
            always_increment_version: false,
            build_version_prompt: None,
            build_attempt: None,
            preferences: UiPreferences::default(),
            theme: ThemePreset::Dark,
            effects: PlatformEffects::default(),
            applied_theme: None,
            initial_size_set: false,
            open_receiver: None,
            data_revision: 0,
            mod_filter: FilterCache::default(),
            config_filter: FilterCache::default(),
            preview_filter: FilterCache::default(),
            logs_filter: FilterCache::default(),
            log_layout: LogLayoutCache::default(),
            readme_preview: ReadmePreview::default(),
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
            documentation: crate::package_docs::PackageDocumentation::default(),
            pack_website: String::new(),
            custom_icon_path: None,
            output_dir: default_output_dir,
            active_tab: ActiveTab::ProfileMetadata,
            status_message: None,
            last_build_result: None,
            building: false,
            build_progress: None,
            build_receiver: None,
            sync,
        };

        app.restore_modpack_library();
        app
    }
}

impl ModpackApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::appearance::install_fonts(&cc.egui_ctx);
        let preferences = UiPreferences::load(&UiPreferences::path());
        let theme = preferences.resolve(&cc.egui_ctx);
        let app = Self {
            preferences,
            theme,
            effects: PlatformEffects::new(cc),
            ..Self::default()
        };
        Palette::new(theme, false).apply(&cc.egui_ctx, theme);
        app
    }

    pub fn refresh_profiles(&mut self) {
        self.data_revision = self.data_revision.wrapping_add(1);
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
                self.selected_profile.clear();
                self.mods.clear();
                self.configs.clear();
            }
        } else {
            self.available_profiles.clear();
            self.selected_profile.clear();
            self.mods.clear();
            self.configs.clear();
        }
    }

    pub fn load_selected_profile_data(&mut self) {
        self.data_revision = self.data_revision.wrapping_add(1);
        if let Some(ref root) = self.selected_root {
            let profile_path = root.join(&self.selected_profile);
            match load_mods_from_profile(&profile_path) {
                Ok(m) => {
                    self.mods = m;
                    self.status_message = None;
                }
                Err(e) => {
                    self.mods.clear();
                    self.status_message = Some((format!("Error loading mods: {}", e), true));
                }
            }
            self.configs = scan_profile_configs(&profile_path);
            self.restore_saved_selections();

            if self.pack_name.is_empty() || self.pack_name == "Valheim_Modpack" {
                self.pack_name = format!("{}_Modpack", self.selected_profile)
                    .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
            }
        }
    }

    pub fn trigger_build(&mut self, ctx: &egui::Context) {
        if self.building || self.build_version_prompt.is_some() {
            return;
        }
        if let Err(error) = self.validate_build_request() {
            self.status_message = Some((error, true));
            return;
        }
        if self.always_increment_version {
            self.start_versioned_build(ctx, true, true);
        } else {
            self.build_version_prompt = Some(BuildVersionPrompt {
                next_version: crate::modpack_config::increment_patch(&self.pack_version),
                always_increment: false,
            });
        }
    }

    fn launch_build(&mut self, ctx: &egui::Context) {
        let clean_name = crate::packager::clean_package_name(&self.pack_name);
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
            documentation: self.documentation.clone(),
            website_url: self.pack_website.clone(),
            selected_mods,
            selected_configs,
            custom_icon: self.custom_icon_path.clone(),
            out_dir: self.output_dir.join("build"),
            dist_dir: self.output_dir.clone(),
        };

        self.building = true;
        self.build_progress = Some((0.02, "Starting build...".to_string()));
        self.active_tab = ActiveTab::Build;

        let (tx, rx) = mpsc::channel();
        self.build_receiver = Some(rx);
        let repaint_ctx = ctx.clone();

        std::thread::spawn(move || {
            let res = build_modpack(
                &opts,
                Some(&|fraction, message| {
                    let _ = tx.send(BuildEvent::Progress(fraction, message.to_string()));
                    repaint_ctx.request_repaint();
                }),
            );
            let _ = tx.send(BuildEvent::Finished(res));
            repaint_ctx.request_repaint();
        });
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
        self.sync.push_log(SyncLog {
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
                self.sync.push_log(SyncLog {
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

    fn poll_build_events(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;

        if let Some(receiver) = &self.build_receiver {
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
            match event {
                BuildEvent::Progress(fraction, msg) => {
                    self.build_progress = Some((fraction, msg));
                }
                BuildEvent::Finished(result) => {
                    self.building = false;
                    self.build_receiver = None;
                    let version_save_error = self.finish_build_version(result.as_ref().ok()).err();
                    match result {
                        Ok(res) => {
                            self.build_progress = Some((1.0, "Complete!".to_string()));
                            let size_kb = (res.zip_size_bytes as f64) / 1024.0;
                            self.status_message = Some((
                                format!(
                                    "Successfully built {} ({:.1} KB) with {} mods and {} configs!",
                                    res.zip_path.display(),
                                    size_kb,
                                    res.mod_count,
                                    res.config_count
                                ),
                                false,
                            ));
                            self.last_build_result = Some(res);
                            if let Some(error) = version_save_error {
                                self.status_message =
                                    Some((format!("ZIP built, but {error}"), true));
                            }
                        }
                        Err(e) => {
                            self.build_progress = None;
                            self.status_message = Some((format!("Build failed: {}", e), true));
                        }
                    }
                }
            }
        }

        if disconnected {
            if self.build_attempt.is_some() {
                let _ = self.finish_build_version(None);
                self.build_progress = None;
                self.status_message =
                    Some(("Build worker disconnected before completion".into(), true));
            }
            self.building = false;
            self.build_receiver = None;
        }
    }

    fn handle_sync_event(&mut self, event: SyncEvent) {
        match event {
            SyncEvent::Log(log) => {
                self.sync.push_log(log);
            }
            SyncEvent::Progress(progress) => {
                self.sync.progress = Some(progress);
            }
            SyncEvent::Preview(preview) => {
                self.sync.preview_revision = self.sync.preview_revision.wrapping_add(1);
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
                        self.sync.push_log(SyncLog {
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
        if mode == SyncMode::CheckWrapper && !self.sync.config.manage_bepinex_launcher {
            self.sync_error(match self.language {
                Language::English => "Custom BepInEx launcher checks and repair are disabled. Enable the hosting compatibility setting first.",
                Language::Spanish => "La revisión y reparación del iniciador BepInEx están desactivadas. Activa primero la opción de compatibilidad del alojamiento.",
            });
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
        self.sync.logs_revision = self.sync.logs_revision.wrapping_add(1);
        self.sync.preview_revision = self.sync.preview_revision.wrapping_add(1);
        self.sync.summary = None;
        self.sync.progress = None;

        if let Err(e) = self.sync.persist() {
            self.sync.push_log(SyncLog {
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
        self.sync.push_log(SyncLog {
            level: LogLevel::Info,
            message: format!("Starting {}...", mode.label()),
        });

        sync::spawn(request, sink);
        ctx.request_repaint();
    }
}

mod ui;

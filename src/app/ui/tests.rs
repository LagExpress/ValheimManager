use super::*;
use crate::sftp::WrapperReport;
use eframe::{egui_wgpu, wgpu};

const TABS: [ActiveTab; 6] = [
    ActiveTab::ProfileMetadata,
    ActiveTab::Mods,
    ActiveTab::Configs,
    ActiveTab::Build,
    ActiveTab::ServerSync,
    ActiveTab::Help,
];
const SIZES: [(u32, u32); 5] = [
    (640, 480),
    (800, 600),
    (980, 656),
    (1280, 720),
    (1920, 1080),
];

fn fixture() -> ModpackApp {
    let mut app = ModpackApp {
        language: Language::English,
        ..Default::default()
    };
    static FIXTURE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = FIXTURE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    app.modpack_library = ModpackLibrary::default();
    app.modpack_library_error = None;
    app.modpack_library_path = std::env::temp_dir().join(format!(
        "valheim-ui-fixture/{}/modpacks-{id}.json",
        std::process::id()
    ));
    app.always_increment_version = false;
    app.pack_version = "1.0.0".into();
    app.custom_icon_path = None;
    app.sync.config = SyncConfig::default();
    app.sync.config.manage_bepinex_launcher = true;
    app.sync.config.host = "valheim.example.net".into();
    app.sync.config.username = "viking".into();
    app.sync.config.staging_dir = "/home/viking/Desktop/Valheim/Northern_Realms/staging".into();
    app.sync.warning = None;
    app.sync.ignore_text = DEFAULT_IGNORED_PATTERNS.join("\n");
    let profiles = std::env::temp_dir().join("valheim-ui-fixture/profiles");
    std::fs::create_dir_all(profiles.join("Northern Realms/BepInEx")).unwrap();
    app.selected_root = Some(profiles);
    app.selected_profile = "Northern Realms".into();
    app.available_profiles = vec![app.selected_profile.clone(), "Vanilla".into()];
    app.pack_name = "Northern_Realms".into();
    app.pack_description="A carefully tuned journey through the northern realms, with building, exploration, and quality-of-life improvements.".into();
    app.output_dir = PathBuf::from("/home/viking/Documents/Valheim/modpacks/dist");
    let names = [
        "BepInEx Pack",
        "Valheim Plus",
        "Epic Loot",
        "Plant Everything",
        "Better Archery",
        "Equipment and Quick Slots",
        "Build Camera",
        "Craft from Containers",
    ];
    app.mods = (0..1000)
        .map(|i| ModItem {
            name: format!("Viking-{}-{i}", names[i % names.len()].replace(' ', "")),
            author_name: format!("Viking Collective {}", i % 8 + 1),
            display_name: format!("{} {}", names[i % names.len()], i + 1),
            description: "A configurable improvement for your Valheim adventure.".into(),
            website_url: String::new(),
            enabled: i % 7 != 0,
            original_version: "1.2.3".into(),
            current_version: "1.2.3".into(),
            selected: i % 7 != 0,
        })
        .collect();
    app.configs = (0..1000)
        .map(|i| ConfigItem {
            name: format!("com.northern-realms.configuration-{i}.cfg"),
            path: PathBuf::from(format!(
                "/profiles/Northern Realms/BepInEx/config/setting-{i}.cfg"
            )),
            is_dir: i % 8 == 0,
            category: match i % 7 {
                0 => ConfigCategory::Sensitive,
                1 => ConfigCategory::Cache,
                _ => ConfigCategory::Safe,
            },
            size: 4096,
            selected: i % 7 > 1,
        })
        .collect();
    app.sync.preview = Some(PreviewResult {
        uploads: (0..1000)
            .map(|i| format!("plugins/NorthernRealms/Plugin-{i}.dll"))
            .collect(),
        deletions: vec!["config/obsolete-setting.cfg".into()],
        local_count: 1250,
        remote_count: 1245,
        wrapper: WrapperReport {
            status: WrapperStatus::Intact,
            symlink_present: true,
            ..Default::default()
        },
    });
    app.sync.logs = (0..2000)
        .map(|i| SyncLog {
            level: match i % 4 {
                0 => LogLevel::Info,
                1 => LogLevel::Success,
                2 => LogLevel::Warning,
                _ => LogLevel::Error,
            },
            message: format!("Deployment event {i}: plugins/NorthernRealms/Plugin-{i}.dll"),
        })
        .collect();
    app.sync.summary = Some(SyncSummary {
        uploaded: 24,
        deleted: 3,
        staged_copied: 28,
        elapsed_secs: 2.4,
        ..Default::default()
    });
    app.last_build_result = Some(BuildResult {
        build_dir: app.output_dir.join("build/Northern_Realms"),
        zip_path: app.output_dir.join("Northern_Realms-1.0.0.zip"),
        mod_count: 23,
        config_count: 20,
        zip_size_bytes: 1_500_000,
    });
    app.data_revision += 1;
    app.sync.preview_revision += 1;
    app.sync.logs_revision += 1;
    app
}

fn render(app: &mut ModpackApp, size: (u32, u32), scale: f32) -> (egui::Context, egui::FullOutput) {
    let ctx = egui::Context::default();
    crate::appearance::install_fonts(&ctx);
    let output = render_in(app, &ctx, size, scale);
    (ctx, output)
}

fn render_in(
    app: &mut ModpackApp,
    ctx: &egui::Context,
    size: (u32, u32),
    scale: f32,
) -> egui::FullOutput {
    Palette::new(app.theme, false).apply(ctx, app.theme);
    let base_time = ctx.input(|i| i.time);
    let hover = std::env::var("VALHEIM_PREVIEW_HOVER")
        .ok()
        .and_then(|value| {
            let (x, y) = value.split_once(',')?;
            Some(egui::pos2(x.parse().ok()?, y.parse().ok()?))
        });
    let mut combined = egui::FullOutput::default();
    for frame in 0..3 {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size.0 as f32, size.1 as f32),
            )),
            time: Some(
                base_time + (frame + 1) as f64 * if hover.is_some() { 0.1 } else { 1.0 / 60.0 },
            ),
            ..Default::default()
        };
        if let Some(position) = hover {
            input.events.push(egui::Event::PointerMoved(position));
        }
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(scale);
        combined.append(ctx.run_ui(input, |ui| app.render(ui)));
    }
    combined
}

#[test]
fn responsive_matrix_preserves_state_and_cached_views() {
    let mut app = fixture();
    for theme in ThemePreset::ALL {
        for language in [Language::English, Language::Spanish] {
            for scale in [1.0, 1.5, 2.0] {
                let ctx = egui::Context::default();
                crate::appearance::install_fonts(&ctx);
                for size in SIZES {
                    for tab in TABS {
                        app.theme = theme;
                        app.language = language;
                        app.active_tab = tab;
                        let mut output = render_in(&mut app, &ctx, size, scale);
                        assert!(!output.shapes.is_empty(), "{theme:?} {size:?}");
                        assert!(
                            output.shapes.len() < 2500,
                            "Unbounded list rendering: {}",
                            output.shapes.len()
                        );
                        assert_eq!(app.mods.len(), 1000);
                        assert_eq!(app.sync.logs.len(), 2000);
                        assert_eq!(app.sync.config.host, "valheim.example.net");
                        assert!(!app.building && !app.sync.running);
                        output.textures_delta.clear();
                    }
                }
            }
        }
    }
    app.active_tab = ActiveTab::Mods;
    render(&mut app, (980, 656), 1.0).1.textures_delta.clear();
    let allocation = app.mod_filter.indices.as_ptr();
    app.building = true;
    app.sync.running = true;
    for theme in ThemePreset::ALL {
        app.theme = theme;
        render(&mut app, (640, 480), 1.0).1.textures_delta.clear();
        assert_eq!(app.mod_filter.indices.as_ptr(), allocation);
        assert!(app.building && app.sync.running);
    }
    app.mod_search = "Epic Loot".into();
    app.update_mod_filter();
    assert!(app
        .mod_filter
        .indices
        .iter()
        .all(|i| app.mods[*i].display_name.contains("Epic Loot")));
}

#[test]
fn sync_toolbar_stays_visible_and_precedes_the_connection_panels() {
    fn visible_text(output: &egui::FullOutput, label: &str) -> egui::Rect {
        let shape = output.shapes.iter().rev().find(|shape| {
            matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.job.text == label)
        }).unwrap_or_else(|| panic!("Missing label: {label}"));
        let rect = shape.shape.visual_bounding_rect();
        assert!(
            shape.clip_rect.expand(1.0).contains_rect(rect),
            "Clipped label: {label}, bounds {rect:?}, clip {:?}",
            shape.clip_rect
        );
        rect
    }

    for theme in ThemePreset::ALL {
        for language in [Language::English, Language::Spanish] {
            for size in [(640, 480), (1920, 1080)] {
                let mut app = fixture();
                app.active_tab = ActiveTab::ServerSync;
                app.sync.logs.truncate(4);
                app.theme = theme;
                app.language = language;
                let ignore_text = app.sync.ignore_text.clone();
                let (ctx, mut output) = render(&mut app, size, 1.0);
                output.textures_delta.clear();
                let actions = visible_text(&output, language.heading_actions());
                let ignore = visible_text(&output, language.heading_ignore());
                let mut buttons = Vec::new();
                for label in [
                    language.btn_action_preview(),
                    language.btn_action_sync(),
                    language.btn_action_upload_only(),
                    language.btn_action_force(),
                    language.btn_action_wrapper(),
                    language.btn_action_refresh(),
                    language.btn_action_test(),
                ] {
                    let rect = visible_text(&output, label);
                    assert!(rect.top() > actions.bottom() && rect.bottom() < ignore.top());
                    buttons.push(rect);
                }
                if size.0 >= 1100 {
                    let source = visible_text(&output, language.heading_source_profile());
                    let preview = visible_text(&output, language.heading_preview());
                    assert!(source.top() > ignore.bottom());
                    assert!(source.left() < preview.left());
                    assert!((source.top() - preview.top()).abs() < 2.0);
                    assert!(buttons
                        .iter()
                        .all(|rect| (rect.top() - buttons[0].top()).abs() < 2.0));
                }
                app.sync.running = true;
                app.sync.active_mode = Some(SyncMode::SyncChanges);
                let mut running = render_in(&mut app, &ctx, size, 1.0);
                running.textures_delta.clear();
                visible_text(&running, tr(language, "Cancel", "Cancelar"));
                assert_eq!(app.sync.ignore_text, ignore_text);
                assert!(app.sync.running);
            }
        }
    }
}

#[test]
fn validation_errors_invalidate_log_views_and_keep_the_buffer_bounded() {
    let mut app = fixture();
    app.sync.logs.clear();
    app.sync.logs_revision += 1;
    app.active_tab = ActiveTab::ServerSync;
    render(&mut app, (1280, 720), 1.0).1.textures_delta.clear();
    assert!(app.logs_filter.indices.is_empty());
    for i in 0..2100 {
        app.sync_error(format!("Invalid connection {i}"));
    }
    render(&mut app, (1280, 720), 1.0).1.textures_delta.clear();
    assert_eq!(app.sync.logs.len(), 2000);
    assert_eq!(app.logs_filter.indices.len(), 2000);
    assert_eq!(
        app.sync.logs.front().unwrap().message,
        "Invalid connection 100"
    );
}

#[test]
fn idle_glass_pages_do_not_schedule_continuous_repaints() {
    let mut app = fixture();
    for theme in [ThemePreset::GlassLight, ThemePreset::GlassDark] {
        let ctx = egui::Context::default();
        crate::appearance::install_fonts(&ctx);
        app.theme = theme;
        for tab in TABS {
            app.active_tab = tab;
            for _ in 0..10 {
                render_in(&mut app, &ctx, (1280, 720), 1.0)
                    .textures_delta
                    .clear();
            }
            let mut output = render_in(&mut app, &ctx, (1280, 720), 1.0);
            let delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
            output.textures_delta.clear();
            assert!(
                delay >= std::time::Duration::from_secs(1),
                "{theme:?} {}: {delay:?}",
                tab_name(tab)
            );
        }
    }
}

fn build_fixture(directory: &std::path::Path) -> ModpackApp {
    let mut app = fixture();
    app.modpack_library_path = directory.join("settings/modpacks.json");
    app.output_dir = directory.join("dist");
    app.mods.truncate(2);
    app.mods[0].selected = true;
    app.configs.clear();
    app
}

fn finish_worker(app: &mut ModpackApp) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.building {
        assert!(std::time::Instant::now() < deadline, "Build did not finish");
        app.poll_build_events();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn archive_version(app: &ModpackApp) -> String {
    let path = &app
        .last_build_result
        .as_ref()
        .expect("Expected successful build")
        .zip_path;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    manifest["version_number"].as_str().unwrap().to_owned()
}

#[test]
fn saved_configurations_restore_metadata_source_selections_and_installed_updates() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = build_fixture(directory.path());
    let profiles = directory.path().join("profiles");
    let profile = profiles.join("Northern Realms");
    std::fs::create_dir_all(profile.join("BepInEx/config")).unwrap();
    std::fs::write(profile.join("mods.yml"), "- name: Viking-BepInExPack-0\n  versionNumber: {major: 2, minor: 4, patch: 0}\n  enabled: true\n- name: Viking-ValheimPlus-1\n  versionNumber: {major: 2, minor: 4, patch: 0}\n  enabled: true\n").unwrap();
    std::fs::write(profile.join("BepInEx/config/server.cfg"), "enabled=true").unwrap();
    app.mod_manager = ModManager::Custom;
    app.selected_root = Some(profiles.clone());
    app.pack_name = "Saved_North".into();
    app.pack_version = "1.2.3".into();
    app.pack_description = "Remember this description".into();
    app.pack_website = "https://example.com".into();
    app.custom_icon_path = Some(directory.path().join("icon.png"));
    app.mods[0].current_version = "9.8.7".into();
    app.mods[1].selected = false;
    app.configs.push(ConfigItem {
        name: "server.cfg".into(),
        path: profile.join("BepInEx/config/server.cfg"),
        is_dir: false,
        category: ConfigCategory::Safe,
        size: 12,
        selected: false,
    });
    let first = app.save_modpack_configuration(false).unwrap();
    app.pack_name = "Another_Pack".into();
    app.always_increment_version = true;
    let second = app.save_modpack_configuration(true).unwrap();
    assert_ne!(first, second);
    app.load_modpack_configuration(first).unwrap();
    assert_eq!(app.pack_name, "Saved_North");
    assert_eq!(app.pack_description, "Remember this description");
    assert_eq!(app.pack_version, "1.2.3");
    assert_eq!(app.pack_website, "https://example.com");
    assert_eq!(app.selected_root, Some(profiles));
    assert!(!app.always_increment_version);
    assert_eq!(app.mods[0].current_version, "9.8.7");
    assert_eq!(app.mods[1].current_version, "2.4.0");
    assert!(!app.mods[1].selected);
    assert!(!app.configs[0].selected);
    let mut restarted = build_fixture(directory.path());
    restarted.restore_modpack_library();
    assert_eq!(restarted.pack_name, app.pack_name);
    assert_eq!(restarted.pack_description, app.pack_description);
    assert_eq!(restarted.pack_version, app.pack_version);
    assert_eq!(restarted.output_dir, app.output_dir);
    assert_eq!(restarted.custom_icon_path, app.custom_icon_path);
    assert_eq!(restarted.modpack_library.configurations.len(), 2);
}

#[test]
fn builds_prompt_keep_increment_and_remember_only_successful_versions() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = build_fixture(directory.path());
    let ctx = egui::Context::default();
    app.pack_version = "1.2.3".into();
    app.trigger_build(&ctx);
    assert!(app.build_version_prompt.is_some());
    assert!(!app.building);
    assert!(!app.modpack_library_path.exists());
    app.build_version_prompt = None; // Cancel leaves the declaration untouched.
    assert_eq!(app.pack_version, "1.2.3");
    app.start_versioned_build(&ctx, false, false);
    finish_worker(&mut app);
    assert_eq!(archive_version(&app), "1.2.3");
    app.start_versioned_build(&ctx, true, true);
    finish_worker(&mut app);
    assert_eq!(archive_version(&app), "1.2.4");
    app.trigger_build(&ctx); // Remembered policy skips the dialog.
    assert!(app.build_version_prompt.is_none());
    finish_worker(&mut app);
    assert_eq!(archive_version(&app), "1.2.5");
    let saved = ModpackLibrary::load(&app.modpack_library_path).unwrap();
    assert_eq!(saved.configurations[0].version, "1.2.5");
    assert!(saved.configurations[0].always_increment);
    let blocked_output = directory.path().join("not-a-folder");
    std::fs::write(&blocked_output, "block output creation").unwrap();
    app.output_dir = blocked_output;
    app.trigger_build(&ctx);
    finish_worker(&mut app);
    assert_eq!(app.pack_version, "1.2.5");
    assert!(app.status_message.as_ref().unwrap().1);
    assert_eq!(
        ModpackLibrary::load(&app.modpack_library_path)
            .unwrap()
            .configurations[0]
            .version,
        "1.2.5"
    );
}

#[test]
fn invalid_inputs_and_unwritable_settings_do_not_start_or_increment_builds() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = build_fixture(directory.path());
    app.always_increment_version = true;
    app.mods.clear();
    app.trigger_build(&egui::Context::default());
    assert!(!app.building && app.build_version_prompt.is_none());
    assert_eq!(app.pack_version, "1.0.0");
    let mut app = build_fixture(directory.path());
    app.modpack_library_path = directory.path().to_owned();
    app.start_versioned_build(&egui::Context::default(), true, true);
    assert!(!app.building && !app.always_increment_version);
    assert_eq!(app.pack_version, "1.0.0");
    assert!(app.modpack_library.configurations.is_empty());
}

#[test]
fn cancelling_the_version_dialog_with_escape_does_not_save_or_increment() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = build_fixture(directory.path());
    let ctx = egui::Context::default();
    crate::appearance::install_fonts(&ctx);
    app.trigger_build(&ctx);
    render_in(&mut app, &ctx, (640, 480), 1.0)
        .textures_delta
        .clear();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 480.0),
            )),
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| app.render(ui),
    );
    output.textures_delta.clear();
    assert!(app.build_version_prompt.is_none());
    assert!(!app.building);
    assert_eq!(app.pack_version, "1.0.0");
    assert!(!app.modpack_library_path.exists());
}

#[test]
fn disabled_launcher_action_is_blocked_before_connection_or_settings_writes() {
    let mut app = fixture();
    app.sync.config.manage_bepinex_launcher = false;
    app.sync.config.host.clear();
    app.sync.config.staging_dir.clear();
    app.start_sync(SyncMode::CheckWrapper, &egui::Context::default());
    assert!(!app.sync.running);
    assert!(app.sync.receiver.is_none());
    assert!(app.status_message.as_ref().unwrap().0.contains("disabled"));
}

#[test]
fn wrapped_log_rows_fit_their_width_and_reuse_layouts() {
    let mut app = fixture();
    app.active_tab = ActiveTab::ServerSync;
    app.sync.logs.clear();
    for message in [
        "A long error message with many words ".repeat(20),
        "a".repeat(512),
        "first line\nsecond line".into(),
    ] {
        app.sync.push_log(SyncLog {
            level: LogLevel::Error,
            message,
        });
    }
    let ctx = egui::Context::default();
    crate::appearance::install_fonts(&ctx);
    render_in(&mut app, &ctx, (640, 480), 1.0)
        .textures_delta
        .clear();
    for galley in &app.log_layout.galleys {
        assert!(galley.rows.len() > 1);
        assert!(!galley.elided);
        assert!(galley.size().x <= galley.job.wrap.max_width + 1.0);
    }
    let first = app.log_layout.galleys[0].clone();
    render_in(&mut app, &ctx, (640, 480), 1.0)
        .textures_delta
        .clear();
    assert!(Arc::ptr_eq(&first, &app.log_layout.galleys[0]));
    render_in(&mut app, &ctx, (1280, 720), 1.0)
        .textures_delta
        .clear();
    assert!(!Arc::ptr_eq(&first, &app.log_layout.galleys[0]));
    app.sync.log_search = "second line".into();
    render_in(&mut app, &ctx, (1280, 720), 1.0)
        .textures_delta
        .clear();
    assert_eq!(app.log_layout.galleys.len(), 1);
}

struct Capture {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: egui_wgpu::Renderer,
}

impl Capture {
    fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("Offscreen graphics adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let renderer = egui_wgpu::Renderer::new(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        Self {
            device,
            queue,
            renderer,
        }
    }

    fn save(
        &mut self,
        ctx: &egui::Context,
        mut output: egui::FullOutput,
        size: (u32, u32),
        path: &std::path::Path,
    ) {
        let scale = output.pixels_per_point;
        let (width, height) = (
            (size.0 as f32 * scale).round() as u32,
            (size.1 as f32 * scale).round() as u32,
        );
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        let jobs = ctx.tessellate(output.shapes, scale);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI capture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut commands =
            self.renderer
                .update_buffers(&self.device, &self.queue, &mut encoder, &jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("UI preview"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &jobs, &screen);
        }
        let pitch = (width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(pitch) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        commands.push(encoder.finish());
        self.queue.submit(commands);
        let (tx, rx) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = buffer.slice(..).get_mapped_range().unwrap();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in mapped.chunks(pitch as usize) {
            pixels.extend_from_slice(&row[..(width * 4) as usize]);
        }
        image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8).unwrap();
        drop(mapped);
        buffer.unmap();
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        output.textures_delta.clear();
    }
}

#[test]
#[ignore = "Exports fixture screenshots; run explicitly for visual review"]
fn export_visual_previews() {
    let root = PathBuf::from("target/ui-previews");
    std::fs::create_dir_all(&root).unwrap();
    let mut capture = Capture::new();
    let mut app = fixture();
    let filter = std::env::var("VALHEIM_PREVIEW_FILTER").unwrap_or_default();
    for theme in ThemePreset::ALL {
        for language in [Language::English, Language::Spanish] {
            let ctx = egui::Context::default();
            crate::appearance::install_fonts(&ctx);
            for size in SIZES {
                for tab in TABS {
                    // All pages at minimum and screenshot widths; Sync also at all wider sizes.
                    if !matches!(size.0, 640 | 980) && tab != ActiveTab::ServerSync {
                        continue;
                    }
                    if language == Language::Spanish && size.0 != 640 {
                        continue;
                    }
                    app.theme = theme;
                    app.language = language;
                    app.active_tab = tab;
                    let name = format!(
                        "{theme:?}-{}-{}x{}-{}.png",
                        language.code(),
                        size.0,
                        size.1,
                        tab_name(tab)
                    );
                    if !name.contains(&filter) {
                        continue;
                    }
                    let output = render_in(&mut app, &ctx, size, 1.0);
                    capture.save(&ctx, output, size, &root.join(name));
                }
            }
            // Capture bottom-of-page controls after all top-of-page captures,
            // so wheel events cannot carry scroll offsets into wider previews.
            for tab in [
                ActiveTab::ServerSync,
                ActiveTab::ProfileMetadata,
                ActiveTab::Help,
                ActiveTab::Build,
            ] {
                let name = format!(
                    "{theme:?}-{}-640x480-{}-scrolled.png",
                    language.code(),
                    tab_name(tab)
                );
                if name.contains(&filter) {
                    app.active_tab = tab;
                    let size = (640, 480);
                    let mut combined = render_in(&mut app, &ctx, size, 1.0);
                    for frame in 0..12 {
                        let input = egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(640.0, 480.0),
                            )),
                            time: Some(ctx.input(|i| i.time) + 0.1),
                            events: if frame == 0 {
                                vec![
                                    egui::Event::PointerMoved(egui::pos2(18.0, 250.0)),
                                    egui::Event::MouseWheel {
                                        unit: egui::MouseWheelUnit::Point,
                                        delta: egui::vec2(0.0, -10000.0),
                                        phase: egui::TouchPhase::Move,
                                        source: egui::MouseWheelSource::Unknown,
                                        modifiers: egui::Modifiers::NONE,
                                    },
                                ]
                            } else {
                                vec![]
                            },
                            ..Default::default()
                        };
                        combined.append(ctx.run_ui(input, |ui| app.render(ui)));
                    }
                    capture.save(&ctx, combined, size, &root.join(name));
                }
            }
        }
    }
    for theme in ThemePreset::ALL {
        for scale in [1.5, 2.0] {
            let ctx = egui::Context::default();
            crate::appearance::install_fonts(&ctx);
            for tab in [
                ActiveTab::ProfileMetadata,
                ActiveTab::Mods,
                ActiveTab::ServerSync,
            ] {
                app.theme = theme;
                app.language = Language::English;
                app.active_tab = tab;
                let size = (1280, 720);
                let name = format!("{theme:?}-{}-scale{scale}.png", tab_name(tab));
                if !name.contains(&filter) {
                    continue;
                }
                let output = render_in(&mut app, &ctx, size, scale);
                capture.save(&ctx, output, size, &root.join(name));
            }
        }
    }
}

#[test]
#[ignore = "Exports saved configuration, version prompt, and wrapping previews"]
fn export_configuration_previews() {
    let root = PathBuf::from("target/ui-previews").join(env!("CARGO_PKG_VERSION"));
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut capture = Capture::new();
    for theme in ThemePreset::ALL {
        for language in [Language::English, Language::Spanish] {
            let mut app = build_fixture(directory.path());
            app.mods = fixture().mods;
            app.theme = theme;
            app.language = language;
            app.pack_version = "1.2.3".into();
            app.save_modpack_configuration(false).unwrap();
            let ctx = egui::Context::default();
            crate::appearance::install_fonts(&ctx);
            let mut save = |app: &mut ModpackApp, name: &str| {
                let mut output = render_in(app, &ctx, (640, 480), 1.0);
                // Let popup fade-in finish before inspecting contrast and clipping.
                for _ in 0..5 {
                    output.append(render_in(app, &ctx, (640, 480), 1.0));
                }
                capture.save(
                    &ctx,
                    output,
                    (640, 480),
                    &root.join(format!("{theme:?}-{}-{name}.png", language.code())),
                );
            };
            save(&mut app, "saved-configuration");
            app.trigger_build(&ctx);
            save(&mut app, "version-dialog");
            app.build_version_prompt = None;
            app.sync.logs.clear();
            for message in [format!("Host key SHA256: {}", "b8f218ae9336540f205fa422faabfe1ddcc17e3914918e8dd58dfca891ba3189"),
                format!("Sync skipped a file under config/{}: the server copy is protected by the current ignore patterns.", "NorthernRealms/LongConfigurationName/".repeat(5)),
                "The SFTP connection closed before the upload finished. Check the connection settings and retry the operation; the previous server state has been preserved.".into()] {
                app.sync.push_log(SyncLog {level:LogLevel::Warning,message});
            }
            let output = Palette::new(theme, false);
            output.apply(&ctx, theme);
            let mut combined = egui::FullOutput::default();
            for _ in 0..3 {
                combined.append(ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(640.0, 480.0),
                        )),
                        ..Default::default()
                    },
                    |ui| app.sync_logs(ui),
                ));
            }
            capture.save(
                &ctx,
                combined,
                (640, 480),
                &root.join(format!("{theme:?}-{}-wrapped-logs.png", language.code())),
            );
            for enabled in [false, true] {
                app.sync.config.manage_bepinex_launcher = enabled;
                app.sync.preview.as_mut().unwrap().wrapper.status = if enabled {
                    WrapperStatus::Intact
                } else {
                    WrapperStatus::Skipped
                };
                let mut output = egui::FullOutput::default();
                for _ in 0..6 {
                    output.append(ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(640.0, 720.0),
                            )),
                            time: Some(ctx.input(|i| i.time) + 0.1),
                            ..Default::default()
                        },
                        |ui| {
                            let palette = Palette::new(theme, false);
                            components::backdrop(
                                ui,
                                palette.background,
                                palette.accent,
                                theme.glass(),
                            );
                            card(ui, app.language.heading_connection(), |ui| {
                                app.launcher_compatibility(ui)
                            });
                            app.sync_preview(ui);
                        },
                    ));
                }
                capture.save(
                    &ctx,
                    output,
                    (640, 720),
                    &root.join(format!(
                        "{theme:?}-{}-launcher-{enabled}.png",
                        language.code()
                    )),
                );
            }
        }
    }
}

fn tab_name(tab: ActiveTab) -> &'static str {
    match tab {
        ActiveTab::ProfileMetadata => "profile",
        ActiveTab::Mods => "mods",
        ActiveTab::Configs => "configs",
        ActiveTab::Build => "build",
        ActiveTab::ServerSync => "sync",
        ActiveTab::Help => "help",
    }
}

#[test]
#[ignore = "Opens a temporary fixture window to verify the native compositor integration"]
fn native_glass_probe() {
    use std::sync::Mutex;
    struct Probe {
        app: ModpackApp,
        frame: usize,
        samples: Arc<Mutex<Vec<(String, bool)>>>,
    }
    impl eframe::App for Probe {
        fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
            [0.0; 4]
        }
        fn on_exit(&mut self) {
            eprintln!("Native probe: releasing effects");
            self.app.effects.shutdown();
        }
        fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
            eprintln!("Native probe: frame {}", self.frame);
            self.app.theme = match self.frame / 3 {
                0 | 4 => ThemePreset::GlassDark,
                1 => ThemePreset::Light,
                2 => ThemePreset::GlassLight,
                _ => ThemePreset::Dark,
            };
            self.app.ui(ui, frame);
            if self.frame % 3 == 2 {
                self.samples
                    .lock()
                    .unwrap()
                    .push((format!("{:?}", self.app.theme), self.app.effects.active()));
            }
            if self.frame == 4 {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(640.0, 480.0)));
            }
            if self.frame == 10 {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(980.0, 656.0)));
            }
            assert_eq!(self.app.sync.config.host, "valheim.example.net");
            assert_eq!(self.app.mods.len(), 1000);
            self.frame += 1;
            if self.frame >= 15 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.ctx().request_repaint();
            }
        }
    }
    let samples = Arc::new(Mutex::new(Vec::new()));
    let recorded = samples.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Valheim UI validation — fixture data")
            .with_inner_size([980.0, 656.0])
            .with_transparent(true),
        event_loop_builder: Some(Box::new(|builder| {
            #[cfg(target_os = "linux")]
            {
                use winit::platform::wayland::EventLoopBuilderExtWayland;
                builder.with_any_thread(true);
            }
            #[cfg(target_os = "windows")]
            {
                use winit::platform::windows::EventLoopBuilderExtWindows;
                builder.with_any_thread(true);
            }
        })),
        ..Default::default()
    };
    eframe::run_native(
        "Valheim UI validation",
        options,
        Box::new(move |cc| {
            eprintln!("Native probe: window created, binding effects");
            let mut app = fixture();
            app.effects = PlatformEffects::new(cc);
            eprintln!("Native probe: effects bound");
            app.initial_size_set = true;
            app.active_tab = ActiveTab::ServerSync;
            crate::appearance::install_fonts(&cc.egui_ctx);
            Ok(Box::new(Probe {
                app,
                frame: 0,
                samples: recorded,
            }))
        }),
    )
    .unwrap();
    let samples = samples.lock().unwrap();
    std::fs::create_dir_all("target/ui-previews").unwrap();
    std::fs::write(
        "target/ui-previews/native-glass-probe.json",
        serde_json::to_vec_pretty(&*samples).unwrap(),
    )
    .unwrap();
    assert_eq!(samples.len(), 5);
    for (theme, active) in samples.iter() {
        if theme.starts_with("Glass") {
            if std::env::var_os("VALHEIM_REQUIRE_NATIVE_BLUR").is_some() {
                assert!(*active, "Native blur unavailable: {theme}");
            }
        } else {
            assert!(!active, "Plain theme should not request a blur region");
        }
    }
}

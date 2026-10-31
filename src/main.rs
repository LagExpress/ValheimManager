mod app;
mod appearance;
mod i18n;
mod modpack_config;
mod package_docs;
mod packager;
mod platform_effects;
mod profile;
mod sftp;
mod sync;
mod ui_components;

use app::ModpackApp;
use eframe::egui;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

fn ensure_desktop_integration() {
    #[cfg(target_os = "linux")]
    {
        if let Some(data_dir) = dirs::data_dir() {
            let apps_dir = data_dir.join("applications");
            let icons_dir = data_dir
                .join("icons")
                .join("hicolor")
                .join("256x256")
                .join("apps");
            let _ = std::fs::create_dir_all(&apps_dir);
            let _ = std::fs::create_dir_all(&icons_dir);

            // Install icon so desktop panels can find it
            let icon_file = icons_dir.join("valheim-modpack-studio.png");
            let _ = std::fs::write(&icon_file, packager::EMBEDDED_ICON);

            // Determine app executable (AppImage path or native binary)
            let exe_path = std::env::var("APPIMAGE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::current_exe().unwrap_or_default());

            let desktop_file = apps_dir.join("valheim-modpack-studio.desktop");
            let desktop_content = format!(
                "[Desktop Entry]\nType=Application\nName=Valheim Modpack Studio\nComment=Create Thunderstore-ready modpacks from r2modman profiles\nExec=\"{}\"\nIcon=valheim-modpack-studio\nCategories=Game;Utility;\nTerminal=false\nStartupWMClass=valheim-modpack-studio\nKeywords=valheim;mods;modpack;thunderstore;\n",
                exe_path.display()
            );
            let _ = std::fs::write(desktop_file, desktop_content);
        }
    }
}

fn main() -> eframe::Result<()> {
    ensure_desktop_integration();

    let icon = eframe::icon_data::from_png_bytes(packager::EMBEDDED_ICON).ok();

    // Window configuration with explicit app_id and decoration buttons for Wayland / COSMIC
    let mut viewport = egui::ViewportBuilder::default()
        .with_app_id("valheim-modpack-studio")
        .with_inner_size([1100.0, 720.0])
        .with_min_inner_size([640.0, 480.0])
        .with_transparent(true)
        .with_title("Valheim Modpack Studio")
        .with_resizable(true)
        .with_maximize_button(true)
        .with_minimize_button(true)
        .with_close_button(true);

    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Valheim Modpack Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(ModpackApp::new(cc)))),
    )
}

mod app;
mod packager;
mod profile;

use app::ModpackApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Window configuration
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 720.0])
            .with_min_inner_size([800.0, 520.0])
            .with_title("Valheim Modpack Studio"),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            supported_backends: eframe::wgpu::Backends::all(),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "Valheim Modpack Studio",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ModpackApp::default()))
        }),
    )
}

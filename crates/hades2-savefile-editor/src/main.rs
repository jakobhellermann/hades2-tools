#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;

const TITLE: &str = "Hades II Savefile Editor";

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([400.0, 300.0])
            .with_inner_size([800.0, 450.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png").as_slice())
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        TITLE,
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

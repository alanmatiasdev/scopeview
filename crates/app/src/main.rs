mod app;
#[cfg(test)]
mod i18n_keys;
mod open;
mod paint;
mod selection;
mod shortcuts;
mod store;
mod theme;
mod tools;
mod view;

/// The window icon, embedded so the binary needs no files next to it.
fn window_icon() -> Option<eframe::egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon/scopeview-256.png")).ok()
}

fn main() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("ScopeView")
        .with_app_id("io.github.alanmatiasdev.ScopeView")
        .with_inner_size([1100.0, 720.0])
        .with_min_inner_size([640.0, 420.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "ScopeView",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(&cc.egui_ctx)))),
    )
}

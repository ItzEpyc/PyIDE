#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "src/app.rs"]
mod app;
#[path = "src/editor.rs"]
mod editor;

fn main() -> eframe::Result<()> {
    app::run()
}

#![windows_subsystem = "windows"]

mod clicker;
mod gui;

use eframe::egui;
use gui::AutoClickerApp;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([260.0, 350.0])
            .with_resizable(false)
            .with_decorations(true),
        centered: true,
        ..Default::default()
    };
    
    eframe::run_native(
        "Auto Clicker",
        options,
        Box::new(|_cc| Box::new(AutoClickerApp::default())),
    );
}

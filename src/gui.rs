use eframe::egui;
use crate::clicker::{AutoClicker, ClickMode};

fn custom_slider(ui: &mut egui::Ui, value: &mut u32, range: std::ops::RangeInclusive<u32>) -> bool {
    let desired_width = ui.available_width();
    let height = 20.0;
    let (response, painter) = ui.allocate_painter(
        egui::vec2(desired_width, height),
        egui::Sense::click_and_drag(),
    );

    let old_value = *value;
    let mut changed = false;
    
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.hover_pos() {
            let rect = response.rect;
            let normalized = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let range_size = range.end() - range.start();
            *value = range.start() + (normalized * range_size as f32) as u32;
            changed = old_value != *value;
        }
    }

    // Draw the track
    let track_rect = egui::Rect::from_min_size(
        response.rect.left_top(),
        egui::vec2(response.rect.width(), height),
    );
    painter.rect_filled(
        track_rect,
        egui::Rounding::same(8.0),
        egui::Color32::from_rgb(60, 60, 60),
    );

    // Draw the fill
    let range_size = (range.end() - range.start()) as f32;
    let fill_width = if range_size == 0.0 {
        response.rect.width()
    } else {
        response.rect.width() * (*value - range.start()) as f32 / range_size
    };
    let fill_rect = egui::Rect::from_min_size(
        response.rect.left_top(),
        egui::vec2(fill_width, height),
    );
    painter.rect_filled(
        fill_rect,
        egui::Rounding::same(8.0),
        egui::Color32::from_rgb(0, 120, 212),
    );

    changed
}

fn custom_radio_button(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let desired_size = egui::vec2(60.0, 25.0);
    let (response, painter) = ui.allocate_painter(
        desired_size,
        egui::Sense::click(),
    );

    // Draw the button background
    let bg_color = if selected {
        egui::Color32::from_rgb(0, 120, 212)
    } else {
        egui::Color32::from_rgb(60, 60, 60)
    };
    painter.rect_filled(
        response.rect,
        egui::Rounding::same(6.0),
        bg_color,
    );

    // Draw the text
    let text_color = if selected {
        egui::Color32::from_rgb(255, 255, 255)
    } else {
        egui::Color32::from_rgb(200, 200, 200)
    };
    painter.text(
        response.rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        text_color,
    );

    response
}

pub struct AutoClickerApp {
    clicker: AutoClicker,
    min_cps: u32,
    max_cps: u32,
    click_mode: ClickMode,
    static_min_cps: u32,
    static_max_cps: u32,
}

impl Default for AutoClickerApp {
    fn default() -> Self {
        let app = Self {
            clicker: AutoClicker::new(),
            min_cps: 5,
            max_cps: 25,
            click_mode: ClickMode::Left,
            static_min_cps: 5,
            static_max_cps: 25,
        };
        
        // Initialize the clicker with our default values
        app.clicker.set_min_cps(app.min_cps);
        app.clicker.set_max_cps(app.max_cps);
        
        app
    }
}

impl eframe::App for AutoClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repainting to update status
        ctx.request_repaint();

        // Set custom visual style
        let mut style = (*ctx.style()).clone();
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(40, 40, 40);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(60, 60, 60);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(80, 80, 80);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(100, 100, 100);
        style.visuals.widgets.open.bg_fill = egui::Color32::from_rgb(120, 120, 120);
        style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
        style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(220, 220, 220);
        style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(240, 240, 240);
        style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255);
        style.visuals.widgets.open.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.open.rounding = egui::Rounding::same(8.0);
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Auto Clicker");
            
            ui.add_space(10.0);
            
            // CPS Settings
            ui.vertical(|ui| {
                ui.label("Min CPS:");
                if custom_slider(ui, &mut self.min_cps, self.static_min_cps..=self.max_cps-1) {
                    self.clicker.set_min_cps(self.min_cps);
                }
                
                ui.add_space(5.0);
                
                ui.label("Max CPS:");
                if custom_slider(ui, &mut self.max_cps, self.min_cps+1..=self.static_max_cps) {
                    self.clicker.set_max_cps(self.max_cps);
                }
            });
            
            ui.add_space(10.0);
            
            // Click Mode Selection
            ui.vertical(|ui| {
                ui.label("Click Mode:");
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if custom_radio_button(ui, self.click_mode == ClickMode::Left, "Left").clicked() {
                        self.click_mode = ClickMode::Left;
                        self.clicker.set_click_mode(ClickMode::Left);
                    }
                    ui.add_space(5.0);
                    if custom_radio_button(ui, self.click_mode == ClickMode::Right, "Right").clicked() {
                        self.click_mode = ClickMode::Right;
                        self.clicker.set_click_mode(ClickMode::Right);
                    }
                    ui.add_space(5.0);
                    if custom_radio_button(ui, self.click_mode == ClickMode::Both, "Both").clicked() {
                        self.click_mode = ClickMode::Both;
                        self.clicker.set_click_mode(ClickMode::Both);
                    }
                });
            });
            
            ui.add_space(10.0);
            
            // Toggle Button
            let is_running = self.clicker.is_running();
            if ui.add(egui::Button::new(if is_running { "Stop" } else { "Start" })
                .fill(if is_running { egui::Color32::from_rgb(200, 0, 0) } else { egui::Color32::from_rgb(0, 120, 212) }))
                .clicked() {
                self.clicker.toggle_running();
            }
            
            ui.add_space(10.0);
            
            // Status
            ui.label(format!("Status: {}", if is_running { "Running" } else { "Stopped" }));
            ui.label(format!("Current Mode: {:?}", self.click_mode));
            ui.label(format!("CPS Range: {}-{}", self.min_cps, self.max_cps));
            
            ui.add_space(10.0);
            
            // Instructions
            ui.label("Press F6 to toggle the clicker on/off");
            
            ui.add_space(10.0);
            
            // Credit
            ui.label("Made by BuPyC12");
        });
    }
} 
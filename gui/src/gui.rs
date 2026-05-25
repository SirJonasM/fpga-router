use egui::{CentralPanel, SidePanel};

use crate::{
    App, LoadStatus,
    constants::{MAX_ZOOM, MIN_ZOOM},
    input::InputHandlerState,
    render::render_placeholder_vello,
};

pub fn draw_ui(app: &mut App) -> egui::Rect {
    SidePanel::left("left_panel")
        .resizable(true)
        .default_width(200.0)
        .show(&app.egui_ctx, |ui| {
            ui.heading("egui Controls");

            ui.label("Hello from egui!");

            ui.separator();

            if ui.button("Button 1").clicked() {
                println!("Button 1 clicked");
            }
        });

    if app.input_handler.state == InputHandlerState::Command {
        render_command_palette(app);
    }
    #[cfg(feature = "diagnostics")]
    egui::Window::new("Perfromance Diagnostics")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .resizable(false)
        .collapsible(false)
        .show(&app.egui_ctx, |ui| {
            let fps = 1.0 / ui.ctx().input(|i| i.stable_dt);
            ui.label(format!("FPS: {:.1}", fps));
            ui.label(format!("Frame Time: {:.2} ms", ui.ctx().input(|i| i.predicted_dt) * 1000.0));
        });

    let mut viewport = egui::Rect::NOTHING;

    let response = CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
        .show(&app.egui_ctx, |ui| {
            viewport = ui.min_rect();

            ui.painter()
                .rect_stroke(viewport, 0.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE));
        })
        .response
        .interact(egui::Sense::drag());
    // 1. Handle Zooming (Scroll)
    let scroll_delta = app.egui_ctx.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0
        && let Some(mouse_pos) = app.egui_ctx.input(|i| i.pointer.hover_pos())
    {
        app.view_transform.zoom_at_point(scroll_delta, mouse_pos, viewport);
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        app.view_transform.pan.x += delta.x as f64;
        app.view_transform.pan.y += delta.y as f64;
    }

    viewport
}

pub fn render_command_palette(app: &mut App) {
    egui::Window::new("Command Palette")
        .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .fixed_size([500.0, 40.0])
        .show(&app.egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(": {}", app.input_handler.buffer))
                        .strong()
                        .size(20.0)
                        .color(egui::Color32::LIGHT_BLUE),
                );
            });
        });
}
pub fn render_vello(app: &mut App, viewport: egui::Rect) {
    let scene = &mut app.scene;
    scene.reset();

    render_placeholder_vello(scene, &viewport);
}

pub fn render_loading(app: &App) {
    if let LoadStatus::Loading(message) = &app.load_status {
        egui::Window::new("Loading").show(&app.egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(message);
            });
        });
    }
}

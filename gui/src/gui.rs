use egui::{CentralPanel, SidePanel};

use crate::{input::InputHandlerState, render::render_placeholder_vello, App, LoadStatus};

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
    if scroll_delta != 0.0 {
        if let Some(mouse_pos) = app.egui_ctx.input(|i| i.pointer.hover_pos()) {
            let zoom_factor = (scroll_delta as f64 * 0.001).exp();

            let mouse_vec = vello::kurbo::Vec2::new(
                mouse_pos.x as f64 - viewport.min.x as f64 - app.view_transform.pan.x,
                mouse_pos.y as f64 - viewport.min.y as f64 - app.view_transform.pan.y,
            );

            let new_scale = app.view_transform.scale * zoom_factor;
            app.view_transform.pan -= mouse_vec * (zoom_factor - 1.0);
            app.view_transform.scale = new_scale;
        }
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

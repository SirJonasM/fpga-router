use egui::{CentralPanel, SidePanel};
use vello::Scene;

use crate::{App, LoadStatus, input::InputHandlerState, render::render_placeholder_vello};

pub fn draw_ui(app: &mut App, ppp: f32) -> egui::Rect {
    SidePanel::left("left_panel")
        .resizable(true)
        .default_width(200.0)
        .show(&app.egui_ctx, |ui| {
            ui.heading("egui Controls");

            ui.label("Hello from egui!");

            ui.separator();

            if let Some((start, end)) = app.selected_edge
                && let Some(graph) = &app.router.current_graph
            {
                let node_start = graph.get_node(start).id();
                let node_end = graph.get_node(end).id();
                ui.label(format!("Selected Edge: {node_start}->{node_end}",));
            }
            if let Some(node) = app.selected_node
                && let Some(graph) = &app.router.current_graph
            {
                let node_id = graph.get_node(node).id();
                ui.label(format!("Selected Node: {node_id}",));
                ui.separator();
                ui.label("Connected to: ");
                graph.get_neighbours(node).iter().enumerate().for_each(|(i, a)| {
                    let id = graph.get_node(*a).id();
                    ui.label(format!("{i}, {id}"));
                });
            }
        });

    if app.input_handler.state == InputHandlerState::Command {
        render_command_palette(app);
    }
    let mut viewport = egui::Rect::NOTHING;

    let response = CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
        .show(&app.egui_ctx, |ui| {
            viewport = ui.clip_rect();

            ui.painter()
                .rect_stroke(viewport, 0.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE));
        })
        .response
        .interact(egui::Sense::drag());

    #[cfg(feature = "diagnostics")]
    egui::Window::new("Diagnostics")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .resizable(false)
        .collapsible(false)
        .show(&app.egui_ctx, |ui| {
            // 1. Performance metrics
            let fps = 1.0 / ui.ctx().input(|i| i.stable_dt);
            ui.label(format!("FPS: {:.1}", fps));
            ui.label(format!("Frame Time: {:.2} ms", ui.ctx().input(|i| i.predicted_dt) * 1000.0));

            ui.separator(); // Nice visual line to split sections

            // 2. Camera Zoom metric (multiplied by 100 for a clean percentage view)
            ui.label(format!("Zoom: {:.0}%", app.view_transform.scale * 100.0));

            // 3. Mouse Position metrics
            if let Some(mouse_pos) = ui.ctx().input(|i| i.pointer.latest_pos()) {
                use crate::app::screen_to_world;
                ui.label(format!("Mouse Screen: X: {:.1}, Y: {:.1}", mouse_pos.x, mouse_pos.y));

                let world_pos = screen_to_world(mouse_pos, viewport, &app.view_transform, ppp);

                ui.label(format!("Mouse World:  X: {:.1}, Y: {:.1}", world_pos.x, world_pos.y));
            } else {
                ui.label("Mouse: Off-screen");
            }
        });
    let scroll_delta = app.egui_ctx.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0
        && let Some(mouse_pos) = app.egui_ctx.input(|i| i.pointer.hover_pos())
    {
        app.view_transform.zoom_at_point(scroll_delta, mouse_pos, viewport);
    }

    let is_dragged = response.dragged_by(egui::PointerButton::Primary);
    if is_dragged {
        let delta = response.drag_delta();
        app.view_transform.pan.x += delta.x as f64;
        app.view_transform.pan.y += delta.y as f64;
    }
    app.is_moving = is_dragged || scroll_delta != 0.0;

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

pub fn render_vello(viewport: egui::Rect) -> Scene {
    let mut scene = vello::Scene::new();
    render_placeholder_vello(&mut scene, &viewport);
    scene
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

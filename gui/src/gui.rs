use egui::{CentralPanel, Id, Response, SidePanel, TopBottomPanel, panel::TopBottomSide};
use router::NodeId;
use vello::Scene;

use crate::{App, LoadStatus, render::render_placeholder_vello};

pub fn draw_ui(app: &mut App) -> Response {
    CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
        .show(&app.egui_ctx, |ui| {
            ui.painter()
                .rect_stroke(egui::Rect::NOTHING, 0.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE));
        })
        .response
        .interact(egui::Sense::drag())
}

pub fn draw_status_line(app: &App) -> Response {
    TopBottomPanel::new(TopBottomSide::Bottom, Id::new("Status Line"))
        .resizable(false)
        .show(&app.egui_ctx, |ui| {
            let zoom_text = format!("{:.0}%", app.view_transform.scale * 100.0);

            if let Some(position) = &app.position {
                let x = position.world_position.x;
                let y = position.world_position.y;
                let location = &position.location;
                ui.label(format!("({x:.1}, {y:.1}) | {zoom_text} | {location}"));
            } else {
                ui.label(format!("Mouse: Off-screen | {zoom_text}"));
            }
        })
        .response
}
pub fn draw_diagnostics(app: &App) {
    egui::Window::new("Diagnostics")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .resizable(false)
        .collapsible(false)
        .show(&app.egui_ctx, |ui| {
            let fps = 1.0 / ui.ctx().input(|i| i.stable_dt);
            ui.label(format!("FPS: {:.1}", fps));
            ui.label(format!("Frame Time: {:.2} ms", ui.ctx().input(|i| i.predicted_dt) * 1000.0));

            ui.separator();

            ui.label(format!("Zoom: {:.0}%", app.view_transform.scale * 100.0));

            if let Some(position) = &app.position {
                ui.label(format!(
                    "Mouse Screen: X: {:.1}, Y: {:.1}",
                    position.mouse_position.x, position.mouse_position.y
                ));
                ui.label(format!(
                    "Mouse World:  X: {:.1}, Y: {:.1}",
                    position.world_position.x, position.world_position.y
                ));
            }
        });
}

pub fn draw_sidepanel(app: &App) -> Option<NodeId> {
    let x = SidePanel::left("left_panel")
        .resizable(true)
        .default_width(200.0)
        .show(&app.egui_ctx, |ui| {
            ui.heading("FPGA Router");
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
                ui.label("Previous:");
                let mut responses = vec![];
                graph.get_previous(node).iter().for_each(|a| {
                    let id = graph.get_node(*a).id();
                    let response = ui.label(id).interact(egui::Sense::click());
                    responses.push((*a, response))
                });
                ui.label("Next");
                graph.get_next(node).iter().for_each(|a| {
                    let id = graph.get_node(*a).id();
                    let response = ui.label(id).interact(egui::Sense::click());
                    responses.push((*a, response))
                });
                return responses.iter().find_map(|a| if a.1.clicked() { Some(a.0) } else { None });
            }
            None
        });
    x.inner
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

use egui::Response;
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use router::{FabricGraph, Node, NodeId, TileId, TileManager};
use std::{
    sync::{
        Arc,
        mpsc::{Receiver, channel},
    },
    time::Instant,
};
use vello::{Renderer, RendererOptions, Scene, peniko::Color};
use wgpu::{
    CommandEncoderDescriptor, DeviceDescriptor, Features, Instance, Limits, LoadOp, Operations, PowerPreference, PresentMode,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, SurfaceConfiguration, TextureUsages,
    TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

#[cfg(feature = "diagnostics")]
use crate::gui::draw_diagnostics;
use crate::{
    GRAPH_SIZE,
    gui::{draw_sidepanel, draw_status_line, render_command_palette},
    input::InputHandlerState,
    layout::Position,
    render::SpatialFabricGrid,
};
use crate::{
    LoadStatus,
    constants::*,
    gui::{draw_ui, render_loading, render_vello},
    input::{Command, Goto, InputHandler},
    layout::{LayoutBuilder, find_location_at_world_pos},
    render::{build_fabric_scene, calculate_visible_tiles},
};

pub struct App {
    pub window: Arc<Window>,

    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,

    pub egui_state: EguiWinitState,
    pub egui_ctx: egui::Context,
    pub egui_renderer: EguiRenderer,

    pub central_rect: egui::Rect,
    pub view_transform: ViewTransform,
    pub vello_renderer: Renderer,
    pub scene: Scene,
    pub spatial_grid: Option<SpatialFabricGrid>,

    pub input_handler: InputHandler,

    pub router: Router,

    pub queues: Messages,
    pub load_status: LoadStatus,
    pub is_moving: bool,
    pub selected_node: Option<NodeId>,
    pub selected_edge: Option<(NodeId, NodeId)>,
    pub focus_point: Option<(vello::kurbo::Point, f64)>,
    pub(crate) position: Option<Position>,
}

#[derive(Debug)]
pub struct ViewTransform {
    pub pan: vello::kurbo::Vec2,
    pub scale: f64,
}
impl ViewTransform {
    pub fn zoom_at_point(&mut self, scroll_delta: f32, mouse_pos: egui::Pos2, viewport: egui::Rect) {
        let raw_zoom_factor = (scroll_delta as f64 * 0.001).exp();

        let target_scale = self.scale * raw_zoom_factor;
        let new_scale = target_scale.clamp(MIN_ZOOM, MAX_ZOOM);

        let effective_zoom_factor = new_scale / self.scale;

        let mouse_vec = vello::kurbo::Vec2::new(
            mouse_pos.x as f64 - viewport.min.x as f64 - self.pan.x,
            mouse_pos.y as f64 - viewport.min.y as f64 - self.pan.y,
        );

        self.pan -= mouse_vec * (effective_zoom_factor - 1.0);
        self.scale = new_scale;
    }
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            pan: vello::kurbo::Vec2::new(0.0, 0.0),
            scale: 1.0,
        }
    }
}

// Add this to your main app struct:
// pub view_transform: ViewTransform,
#[derive(Default)]
pub struct Router {
    pub current_graph: Option<Arc<FabricGraph>>,
    pub current_tile_manager: Option<Arc<TileManager>>,
}

#[derive(Default)]
pub struct Messages {
    rx_graph: Option<Receiver<router::FabricGraph>>,
    rx_tile_manager: Option<Receiver<router::TileManager>>,
}

impl App {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // WGPU
        let instance = Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let format = surface_caps.formats[0];

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // egui
        let egui_ctx = egui::Context::default();

        let egui_state = EguiWinitState::new(egui_ctx.clone(), egui::ViewportId::ROOT, &window, None, None);

        let egui_renderer = EguiRenderer::new(&device, format, None, 1);

        // vello
        let vello_renderer = Renderer::new(
            &device,
            RendererOptions {
                surface_format: Some(format),
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
            },
        )
        .unwrap();

        let scene = Scene::new();

        let graph = Arc::new(FabricGraph::from_file(&format!("tests/data/pips_{GRAPH_SIZE}x{GRAPH_SIZE}.txt"), None).unwrap());
        let tile_manager = Arc::new(TileManager::from_file(&format!("tests/data/bel_{GRAPH_SIZE}x{GRAPH_SIZE}.txt")).unwrap());

        // 1. Build your new spatial acceleration grid here once on load!
        let spatial_grid = Some(SpatialFabricGrid::build_from_graph(&graph, &tile_manager));
        let router = Router {
            current_graph: Some(graph),
            current_tile_manager: Some(tile_manager),
        };

        Self {
            window,
            surface,
            device,
            queue,
            config,
            egui_state,
            egui_ctx,
            egui_renderer,
            vello_renderer,
            scene,
            spatial_grid,
            router,
            is_moving: false,
            view_transform: Default::default(),
            load_status: Default::default(),
            input_handler: Default::default(),
            queues: Default::default(),
            selected_node: Default::default(),
            selected_edge: Default::default(),
            focus_point: Default::default(),
            central_rect: egui::Rect::NOTHING,
            position: Default::default(),
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;

            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) {
        if let Some(command) = self.input_handler.pop_command() {
            self.process_command(command);
        }

        self.check_background_tasks();

        render_loading(self);

        let output = self.surface.get_current_texture().unwrap();

        let view = output.texture.create_view(&TextureViewDescriptor::default());

        // --- egui begin frame ---
        let raw_input = self.egui_state.take_egui_input(&self.window);

        self.egui_ctx.begin_frame(raw_input);

        let ppp = self.egui_ctx.pixels_per_point();
        self.set_position(ppp);

        let _ = draw_status_line(self);
        draw_sidepanel(self);
        let response_central = draw_ui(self);
        self.central_rect = response_central.rect;

        #[cfg(feature = "diagnostics")]
        draw_diagnostics(self);

        if self.input_handler.state == InputHandlerState::Command {
            render_command_palette(self);
        }

        let viewport_vello = response_central.rect;
        self.handle_pan_movement(&response_central);
        self.find_entities();

        self.scene.reset();

        let full_output = self.egui_ctx.end_frame();

        let paint_jobs = self.egui_ctx.tessellate(full_output.shapes, ppp);

        // Upload egui textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }
        // --- render vello ---
        let (scene, transform) = self.render_vello(ppp);
        self.scene.append(&scene, transform);

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("main encoder"),
        });
        self.focus_point();

        self.vello_renderer
            .render_to_surface(
                &self.device,
                &self.queue,
                &self.scene,
                &output,
                &vello::RenderParams {
                    base_color: Color::BLACK,
                    width: self.config.width,
                    height: self.config.height,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .unwrap();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.egui_ctx.pixels_per_point(),
        };

        self.egui_renderer
            .update_buffers(&self.device, &self.queue, &mut encoder, &paint_jobs, &screen_descriptor);

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer.render(&mut rpass, &paint_jobs, &screen_descriptor);
        }

        self.queue.submit(Some(encoder.finish()));

        output.present();

        // Cleanup egui textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
    }
    fn focus_point(&mut self){ 
        if let Some(focus) = self.focus_point.take() {
            let target_world_pos = focus.0;
            let scale = focus.1;

            self.view_transform.scale = scale;

            let viewport_center_x = self.central_rect.min.x as f64 + (self.central_rect.width() as f64 / 2.0);
            let viewport_center_y = self.central_rect.min.y as f64 + (self.central_rect.height() as f64 / 2.0);

            let pan_x = viewport_center_x - (target_world_pos.x * scale);
            let pan_y = viewport_center_y - (target_world_pos.y * scale);

            self.view_transform.pan = vello::kurbo::Vec2::new(pan_x, pan_y);
        }
    }
    pub fn process_command(&mut self, command: Command) {
        match command {
            Command::LoadBel(filename) => {
                println!("Starting background load for: {}", filename);
                self.load_status = LoadStatus::Loading("Waiting for parsing Bel file.".to_string());

                // Create the channel
                let (tx, rx) = channel();
                self.queues.rx_tile_manager = Some(rx);

                // Spawn the worker thread
                std::thread::spawn(move || {
                    // This happens in the background
                    let result = router::TileManager::from_file(&filename);

                    match result {
                        Ok(tile_manager) => {
                            let _ = tx.send(tile_manager); // Send back to main thread
                        }
                        Err(e) => eprintln!("Failed to load graph: {}", e),
                    }
                });
            }
            Command::LoadPips(filename) => {
                println!("Starting background load for: {}", filename);
                self.load_status = LoadStatus::Loading("Waiting for parsing PIPS file.".to_string());

                // Create the channel
                let (tx, rx) = channel();
                self.queues.rx_graph = Some(rx);

                // Spawn the worker thread
                std::thread::spawn(move || {
                    // This happens in the background
                    let result = router::FabricGraph::from_file(&filename, None);

                    match result {
                        Ok(graph) => {
                            let _ = tx.send(graph); // Send back to main thread
                        }
                        Err(e) => eprintln!("Failed to load graph: {}", e),
                    }
                });
            }
            Command::Goto(sub_command) => match sub_command {
                Goto::Tile { x, y } => {
                    let cord = LayoutBuilder::new()
                        .tile(&TileId(x as u8, y as u8))
                        .tile_middle_point()
                        .build();
                    const SCALE: f64 = 6.0;
                    self.focus_point = Some((cord, SCALE));
                }
                Goto::Lut { x, y, bel } => {
                    let time = Instant::now();
                    let cord = LayoutBuilder::new()
                        .tile(&TileId(x as u8, y as u8))
                        .tile_inner()
                        .lut(bel)
                        .lut_middle_point()
                        .build();
                    println!("Calculated lut positon in {:?}", time.elapsed());
                    let time = Instant::now();
                    if let Some((_, cord)) = &self.spatial_grid.as_ref().and_then(|grid| {
                        grid.buckets
                            .get(&TileId(x as u8, y as u8))
                            .and_then(|bucket| bucket.lut_data.iter().find(|a| a.0.eq_ignore_ascii_case(&bel)))
                    }) {
                        println!("Found lut positon in {:?}", time.elapsed());
                        const SCALE: f64 = 10.0;
                        self.focus_point = Some((*cord, SCALE));
                    }

                    const SCALE: f64 = 10.0;
                    self.focus_point = Some((cord, SCALE));
                }
                _ => todo!(),
            },
            _ => {}
        }
    }

    fn render_vello(&self, ppp: f32) -> (Scene, Option<vello::kurbo::Affine>) {
        if let (Some(tile_manager), Some(spatial_grid)) = (&self.router.current_tile_manager, &self.spatial_grid) {
            let world_min = screen_to_world(self.central_rect.min, &self.central_rect, &self.view_transform, ppp);
            let world_max = screen_to_world(self.central_rect.max, &self.central_rect, &self.view_transform, ppp);
            let visible_range = calculate_visible_tiles(world_min, world_max);
            let current_scene_fabric = build_fabric_scene(
                tile_manager,
                spatial_grid,
                &visible_range,
                self.view_transform.scale,
                self.is_moving,
                self.selected_node,
                self.selected_edge,
            );
            let transform = vello::kurbo::Affine::translate((
                (self.central_rect.min.x * ppp) as f64 + self.view_transform.pan.x,
                (self.central_rect.min.y * ppp) as f64 + self.view_transform.pan.y,
            )) * vello::kurbo::Affine::scale(self.view_transform.scale);
            (current_scene_fabric, Some(transform))
        } else {
            (render_vello(self.central_rect), None)
        }
    }
    fn find_entities(&mut self) {
        self.egui_ctx.input(|i| {
            if i.pointer.any_released()
                && !self.is_moving
                && let Some(position) = &self.position
                && let Some(ref spatial_grid) = self.spatial_grid
            {
                if let Some(node) = find_node_at_pos(position, spatial_grid) {
                    self.selected_node = Some(node);
                    self.selected_edge = None;
                } else if let Some(edge) = find_edge_at_pos(position, spatial_grid) {
                    self.selected_edge = Some(edge);
                    self.selected_node = None;
                } else {
                    self.selected_node = None;
                    self.selected_edge = None;
                }
            }
        });
    }
    fn set_position(&mut self, ppp: f32) {
        let mouse_position = self.egui_ctx.input(|i| i.pointer.latest_pos());
        self.position = if let Some(mouse_position) = mouse_position
            && let Some(spatial_grid) = &self.spatial_grid
        {
            let world_position = screen_to_world(mouse_position, &self.central_rect, &self.view_transform, ppp);
            let p = find_location_at_world_pos(world_position.x, world_position.y, spatial_grid);
            Some(Position {
                mouse_position,
                world_position,
                location: p,
            })
        } else {
            None
        };
    }

    pub fn check_background_tasks(&mut self) {
        // 1. Handle Tile Manager Background Task
        if let Some(ref rx) = self.queues.rx_tile_manager {
            match rx.try_recv() {
                Ok(new_tile_manager) => {
                    println!("Successfully loaded Tile Manager!");
                    self.router.current_tile_manager = Some(Arc::new(new_tile_manager));

                    self.load_status = LoadStatus::Idle;
                    self.queues.rx_tile_manager = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    println!("Error: Background thread for Tile Manager crashed.");
                    self.queues.rx_tile_manager = None;
                    self.load_status = LoadStatus::Idle;
                }
                _ => {}
            }
        }

        // 2. Handle Graph Background Task
        if let Some(ref rx) = self.queues.rx_graph {
            match rx.try_recv() {
                Ok(new_graph) => {
                    println!("Successfully loaded FPGA Fabric Graph!");
                    self.router.current_graph = Some(Arc::new(new_graph));

                    self.load_status = LoadStatus::Idle;
                    self.queues.rx_graph = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    println!("Error: Background thread for Pips crashed.");
                    self.queues.rx_graph = None;
                    self.load_status = LoadStatus::Idle;
                }
                _ => {}
            }
        }
    }
    fn handle_pan_movement(&mut self, response: &Response) {
        let rect = response.rect;
        let scroll_delta = self.egui_ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0
            && let Some(mouse_pos) = self.egui_ctx.input(|i| i.pointer.hover_pos())
        {
            self.view_transform.zoom_at_point(scroll_delta, mouse_pos, rect);
        }

        let is_dragged = response.dragged_by(egui::PointerButton::Primary);
        if is_dragged {
            let delta = response.drag_delta();
            self.view_transform.pan.x += delta.x as f64;
            self.view_transform.pan.y += delta.y as f64;
        }
        self.is_moving = is_dragged || scroll_delta != 0.0;
    }
}

/// Translates a screen-space pixel position (e.g., from egui mouse inputs)
/// into the underlying world-space coordinates of your FPGA fabric.
pub fn screen_to_world(
    screen_pos: egui::Pos2,
    viewport: &egui::Rect,
    view_transform: &ViewTransform,
    pixels_per_point: f32,
) -> vello::kurbo::Point {
    let sx = screen_pos.x as f64 * pixels_per_point as f64;
    let sy = screen_pos.y as f64 * pixels_per_point as f64;

    let vx = viewport.min.x as f64 * pixels_per_point as f64;
    let vy = viewport.min.y as f64 * pixels_per_point as f64;

    vello::kurbo::Point::new(
        (sx - vx - view_transform.pan.x) / view_transform.scale,
        (sy - vy - view_transform.pan.y) / view_transform.scale,
    )
}

/// Translates a world-space coordinate back into an absolute screen-space
/// pixel position relative to the window.
pub fn world_to_screen(
    world_pos: vello::kurbo::Point,
    viewport: egui::Rect,
    view_transform: &ViewTransform,
    pixels_per_point: f32,
) -> egui::Pos2 {
    let screen_x = (world_pos.x * view_transform.scale) + viewport.min.x as f64 + view_transform.pan.x;
    let screen_y = (world_pos.y * view_transform.scale) + viewport.min.y as f64 + view_transform.pan.y;

    egui::Pos2::new(screen_x as f32, screen_y as f32)
}

pub fn find_edge_at_pos(position: &Position, spatial_grid: &SpatialFabricGrid) -> Option<(NodeId, NodeId)> {
    let world_x = position.world_position.x;
    let world_y = position.world_position.y;
    let p = find_location_at_world_pos(world_x, world_y, spatial_grid);
    let target_tile_id = match p {
        crate::layout::TargetLocation::Outer(tile_id) => tile_id,
        crate::layout::TargetLocation::Inner(tile_id, _) => tile_id,
        crate::layout::TargetLocation::InnerEmpty(tile_id) => tile_id,
        crate::layout::TargetLocation::None => return None,
    };
    let bucket = spatial_grid.buckets.get(&target_tile_id)?;

    const CLICK_TOLERANCE: f64 = WIRE_LINE_WIDTH * 10.0;
    const TOLERANCE_SQ: f64 = CLICK_TOLERANCE * CLICK_TOLERANCE;

    let mut closest_edge = None;
    let mut min_distance_sq = TOLERANCE_SQ;

    for edge in &bucket.edge_data {
        let dist_sq = distance_to_segment(position.world_position, edge.start_pos, edge.end_pos);

        if dist_sq < min_distance_sq {
            min_distance_sq = dist_sq;
            closest_edge = Some((edge.source_node, edge.target_node));
        }
    }

    closest_edge
}

fn find_node_at_pos(position: &Position, spatial_grid: &SpatialFabricGrid) -> Option<NodeId> {
    let world_x = position.world_position.x;
    let world_y = position.world_position.y;

    let target_tile_id = match position.location {
        crate::layout::TargetLocation::Outer(tile_id) => tile_id,
        crate::layout::TargetLocation::Inner(tile_id, _) => tile_id,
        crate::layout::TargetLocation::InnerEmpty(tile_id) => tile_id,
        crate::layout::TargetLocation::None => return None,
    };

    let bucket = spatial_grid.buckets.get(&target_tile_id)?;

    const RADIUS: f64 = WIRE_NODE_RADIUS * 1.2;
    const RADIUS_SQ: f64 = (WIRE_NODE_RADIUS * 1.2) * (WIRE_NODE_RADIUS * 1.2);
    let min_x = world_x - RADIUS;
    let max_x = world_x + RADIUS;

    let start_idx = match bucket
        .node_data
        .binary_search_by(|(_, pos)| pos.x.partial_cmp(&min_x).unwrap())
    {
        Ok(idx) => idx,
        Err(idx) => idx,
    };
    for &(node_id, node_pos) in &bucket.node_data[start_idx..] {
        if node_pos.x > max_x {
            break;
        }
        let dx = world_x - node_pos.x;
        let dy = world_y - node_pos.y;
        let distance = dx * dx + dy * dy;

        if distance <= RADIUS_SQ {
            return Some(node_id);
        }
    }

    None
}
/// Calculates the shortest distance squared from point `p` to line segment `a_to_b`.
fn distance_to_segment(point: vello::kurbo::Point, segment_a: vello::kurbo::Point, segment_b: vello::kurbo::Point) -> f64 {
    let ab = vello::kurbo::Vec2::new(segment_b.x - segment_a.x, segment_b.y - segment_a.y);
    let ap = vello::kurbo::Vec2::new(point.x - segment_a.x, point.y - segment_a.y);

    let ab_len_sq = ab.dot(ab);
    if ab_len_sq == 0.0 {
        return ap.dot(ap);
    }

    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);

    let closest_point = vello::kurbo::Point::new(segment_a.x + t * ab.x, segment_a.y + t * ab.y);

    let dx = point.x - closest_point.x;
    let dy = point.y - closest_point.y;
    dx * dx + dy * dy
}

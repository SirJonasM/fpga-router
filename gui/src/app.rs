use egui::Response;
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use router::{FabricGraph, TileId, TileManager};
use std::sync::{
    Arc,
    mpsc::{Receiver, channel},
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
    core::{Entity, Lut, Position, SpatialFabricGrid, find_entities_at_position},
    gui::{draw_sidepanel, draw_status_line, render_command_palette},
    input::InputHandlerState,
    utils::{SelectionManager, screen_to_world},
};
use crate::{
    LoadStatus,
    constants::*,
    core::VisibleTileRange,
    gui::{draw_ui, render_loading, render_vello},
    input::{Command, Goto, InputHandler},
    render::fabric_scene,
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
    pub selected_entity: SelectionManager<Entity>,
    pub position: Option<Position>,
    pub ppp: f64,
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
            selected_entity: SelectionManager::new(),
            central_rect: egui::Rect::NOTHING,
            position: Default::default(),
            ppp: 1.0,
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

        self.ppp = self.egui_ctx.pixels_per_point() as f64;

        let _ = draw_status_line(self);
        let entity = draw_sidepanel(self);

        if let Some(entity) = entity {
            self.focus_entity(entity);
            self.selected_entity.select(entity);
        }

        let response_central = draw_ui(self);
        self.central_rect = response_central.rect;

        #[cfg(feature = "diagnostics")]
        draw_diagnostics(self);

        if self.input_handler.state == InputHandlerState::Command {
            render_command_palette(self);
        }

        self.scene.reset();

        let full_output = self.egui_ctx.end_frame();

        let paint_jobs = self.egui_ctx.tessellate(full_output.shapes, self.ppp as f32);

        // Upload egui textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }
        // --- render vello ---
        let (scene, transform) = self.render_vello(self.ppp);
        self.scene.append(&scene, transform);

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

        self.handle_pan_movement(&response_central);

        self.position = if let Some(mouse_position) = response_central.hover_pos()
            && let Some(spatial_grid) = &self.spatial_grid
        {
            let position = self.get_position(mouse_position, spatial_grid);
            if response_central.clicked() {
                let found_entity = find_entities_at_position(&position, spatial_grid);
                self.selected_entity.select_from_spatial_query(found_entity);
            }
            Some(position)
        } else {
            None
        };

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.egui_ctx.pixels_per_point(),
        };

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("main encoder"),
        });
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

    pub fn focus_entity(&mut self, entity: Entity) {
        let (point, scale) = match entity {
            Entity::Tile(tile) => (tile.mid_point(), TILE_FOCUS_SCALE),
            Entity::Lut(lut) => (lut.mid_point(), LUT_FOCUS_SCALE),
            Entity::Node(node) => (node.position, NODE_FOCUS_SCALE),
            Entity::Edge(edge) => (edge.mid_point(), edge.focus()),
        };
        self.focus_point(point, scale);
    }
    pub fn focus_point(&mut self, point: vello::kurbo::Point, scale: f64) {
        self.view_transform.scale = scale;
        let min_x_pixels = self.central_rect.min.x as f64 * self.ppp;
        let min_y_pixels = self.central_rect.min.y as f64 * self.ppp;
        let width_pixels = self.central_rect.width() as f64 * self.ppp;
        let height_pixels = self.central_rect.height() as f64 * self.ppp;

        let viewport_center_x = min_x_pixels + (width_pixels / 2.0);
        let viewport_center_y = min_y_pixels + (height_pixels / 2.0);
        let pan_x = viewport_center_x - (point.x * scale) - min_x_pixels;
        let pan_y = viewport_center_y - (point.y * scale) - min_y_pixels;

        self.view_transform.pan = vello::kurbo::Vec2::new(pan_x, pan_y);
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
            Command::Goto(sub_command) => {
                let entity = match sub_command {
                    Goto::Tile { x, y } => {
                        let Some(spatial_grid) = &self.spatial_grid else {
                            return;
                        };
                        let tile_id = TileId(x as u8, y as u8);
                        spatial_grid.buckets.get(&tile_id).map(|tile| Entity::Tile(tile.tile_data))
                    }
                    Goto::Lut { x, y, bel } => {
                        let Some(spatial_grid) = &self.spatial_grid else {
                            return;
                        };
                        let tile_id = TileId(x as u8, y as u8);
                        spatial_grid
                            .buckets
                            .get(&tile_id)
                            .and_then(|bucket| {
                                bucket
                                    .lut_data
                                    .iter()
                                    .find(|Lut { bel_index, .. }| bel.eq_ignore_ascii_case(bel_index))
                            })
                            .copied()
                            .map(Entity::Lut)
                    }
                    Goto::Edge {
                        x1,
                        y1,
                        label1,
                        x2: _,
                        y2: _,
                        label2,
                    } => {
                        let Some(graph) = &self.router.current_graph else {
                            return;
                        };
                        let Some(spatial_grid) = &self.spatial_grid else {
                            return;
                        };
                        let tile_id1 = TileId(x1 as u8, y1 as u8);
                        spatial_grid
                            .buckets
                            .get(&tile_id1)
                            .and_then(|bucket| {
                                bucket.edge_data.iter().find(|edge_data| {
                                    graph.get_node(edge_data.source_node).id == label1
                                        && graph.get_node(edge_data.target_node).id == label2
                                        || graph.get_node(edge_data.source_node).id == label2
                                            && graph.get_node(edge_data.target_node).id == label1
                                })
                            })
                            .copied()
                            .map(Entity::Edge)
                    }
                    Goto::Node { x, y, label } => {
                        let Some(graph) = &self.router.current_graph else {
                            return;
                        };
                        let Some(spatial_grid) = &self.spatial_grid else {
                            return;
                        };
                        let tile_id = TileId(x as u8, y as u8);
                        spatial_grid
                            .buckets
                            .get(&tile_id)
                            .and_then(|bucket| {
                                bucket
                                    .node_data
                                    .iter()
                                    .find(|crate::core::Node { id, .. }| graph.get_node(*id).id == label)
                            })
                            .copied()
                            .map(Entity::Node)
                    }
                };

                if let Some(entity) = entity {
                    self.focus_entity(entity);
                    self.selected_entity.select(entity);
                }
            }
            _ => {}
        }
    }

    fn render_vello(&self, ppp: f64) -> (Scene, Option<vello::kurbo::Affine>) {
        if let Some(spatial_grid) = &self.spatial_grid {
            let world_min = screen_to_world(self.central_rect.min, &self.central_rect, &self.view_transform, ppp);
            let world_max = screen_to_world(self.central_rect.max, &self.central_rect, &self.view_transform, ppp);
            let visible_range = VisibleTileRange::new(world_min, world_max);
            let current_scene_fabric = fabric_scene(
                spatial_grid,
                &visible_range,
                self.view_transform.scale,
                self.is_moving,
                self.selected_entity.current,
            );
            let transform = vello::kurbo::Affine::translate((
                (self.central_rect.min.x * ppp as f32) as f64 + self.view_transform.pan.x,
                (self.central_rect.min.y * ppp as f32) as f64 + self.view_transform.pan.y,
            )) * vello::kurbo::Affine::scale(self.view_transform.scale);
            (current_scene_fabric, Some(transform))
        } else {
            (render_vello(self.central_rect), None)
        }
    }

    fn get_position(&self, screen_pos: egui::Pos2, spatial_grid: &SpatialFabricGrid) -> Position {
        let world_position = screen_to_world(screen_pos, &self.central_rect, &self.view_transform, self.ppp);
        let p = spatial_grid.find_location_at_world_pos(world_position.x, world_position.y);
        Position {
            mouse_position: screen_pos,
            world_position,
            location: p,
        }
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

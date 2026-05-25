use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use router::{FabricGraph, TileManager};
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

use crate::{
    LoadStatus, constants::{MAX_ZOOM, MIN_ZOOM}, gui::{draw_ui, render_loading, render_vello}, input::{Command, InputHandler}, render::{build_fabric_scene, calculate_visible_tiles}
};
use crate::{XXXXXX, render::SpatialFabricGrid};

pub struct App {
    pub window: Arc<Window>,

    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,

    pub egui_state: EguiWinitState,
    pub egui_ctx: egui::Context,
    pub egui_renderer: EguiRenderer,

    pub view_transform: ViewTransform,
    pub vello_renderer: Renderer,
    pub scene: Scene,
    pub spatial_grid: Option<SpatialFabricGrid>,
    pub _routing_scene: Option<Scene>,

    pub input_handler: InputHandler,

    pub router: Router,

    pub queues: Messages,
    pub load_status: LoadStatus, 
}

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
    current_graph: Option<Arc<FabricGraph>>,
    current_tile_manager: Option<Arc<TileManager>>,
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
        let routing_scene = None;

        let graph = Arc::new(FabricGraph::from_file(&format!("tests/data/pips_{XXXXXX}x{XXXXXX}.txt"), None).unwrap());
        let tile_manager = Arc::new(TileManager::from_file(&format!("tests/data/bel_{XXXXXX}x{XXXXXX}.txt")).unwrap());

        // 1. Build your new spatial acceleration grid here once on load!
        let spatial_grid = Some(SpatialFabricGrid::build_from_graph(&graph));
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
            _routing_scene: routing_scene,
            spatial_grid,
            input_handler: Default::default(),
            router,
            queues: Default::default(),
            load_status: LoadStatus::Idle,
            view_transform: ViewTransform::default(),
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

        let viewport = draw_ui(self);

        let full_output = self.egui_ctx.end_frame();

        let paint_jobs = self.egui_ctx.tessellate(full_output.shapes, self.egui_ctx.pixels_per_point());

        // Upload egui textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // --- render vello ---
        let view_port_rect = self.egui_ctx.screen_rect();

        self.scene.reset();

        let transform = vello::kurbo::Affine::translate((
            viewport.min.x as f64 + self.view_transform.pan.x,
            viewport.min.y as f64 + self.view_transform.pan.y,
        )) * vello::kurbo::Affine::scale(self.view_transform.scale);

        if let (Some(graph), Some(tile_manager), Some(spatial_grid)) = (
            &self.router.current_graph,
            &self.router.current_tile_manager,
            &self.spatial_grid,
        ) {
            let visible_range = calculate_visible_tiles(view_port_rect, &self.view_transform);
            let current_fram_fabric =
                build_fabric_scene(graph, tile_manager, spatial_grid, &visible_range, self.view_transform.scale);
            self.scene.append(&current_fram_fabric, Some(transform));
        } else {
            // Fallback to your placeholder shapes if no graph is loaded
            render_vello(self, viewport);
        }

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("main encoder"),
        });

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

        // --- render egui on top ---
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
            _ => {}
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
}

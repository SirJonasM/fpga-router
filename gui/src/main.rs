use egui::{CentralPanel, SidePanel};
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use router::{FabricGraph, TileId, TileManager};
use std::collections::{HashMap, VecDeque};
use std::str::SplitWhitespace;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use vello::peniko::{Color, Fill};
use vello::{Renderer, RendererOptions, Scene};
use winit::keyboard::{Key, NamedKey, SmolStr};

use vello::kurbo::{Affine, Rect};
use wgpu::RequestAdapterOptions;
use wgpu::{
    CommandEncoderDescriptor, DeviceDescriptor, Features, Instance, Limits, LoadOp, Operations, PowerPreference, PresentMode,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
};
use winit::window::Window;
use winit::{dpi::PhysicalSize, event::*, event_loop::EventLoop, window::WindowBuilder};

const XXXXXX: usize = 4;

struct App {
    window: Arc<Window>,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: Queue,
    config: SurfaceConfiguration,

    egui_state: EguiWinitState,
    egui_ctx: egui::Context,
    egui_renderer: EguiRenderer,

    view_transform: ViewTransform,
    vello_renderer: Renderer,
    scene: Scene,
    fabric_scene: Option<Scene>,
    routing_scene: Option<Scene>,

    input_handler: InputHandler,

    router: Router,

    queues: Messages,
    load_status: LoadStatus, // To show a "Loading..." spinner in UI
}

pub struct ViewTransform {
    pub pan: vello::kurbo::Vec2,
    pub scale: f64,
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
struct Router {
    current_graph: Option<Arc<FabricGraph>>,
    current_tile_manager: Option<Arc<TileManager>>,
}

#[derive(Default)]
struct Messages {
    rx_graph: Option<Receiver<router::FabricGraph>>,
    rx_tile_manager: Option<Receiver<router::TileManager>>,
}

impl App {
    async fn new(window: Arc<Window>) -> Self {
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

        let fabric_scene = Some(build_fabric_scene(&graph, &tile_manager));
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
            fabric_scene,
            routing_scene,
            input_handler: Default::default(),
            router,
            queues: Default::default(),
            load_status: LoadStatus::Idle,
            view_transform: ViewTransform::default(),
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;

            self.surface.configure(&self.device, &self.config);
        }
    }

    fn ui(&mut self) -> egui::Rect {
        SidePanel::left("left_panel")
            .resizable(true)
            .default_width(200.0)
            .show(&self.egui_ctx, |ui| {
                ui.heading("egui Controls");

                ui.label("Hello from egui!");

                ui.separator();

                if ui.button("Button 1").clicked() {
                    println!("Button 1 clicked");
                }
            });

        if self.input_handler.state == InputHandlerState::Command {
            self.render_command_palette();
        }

        let mut viewport = egui::Rect::NOTHING;

        let response = CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(&self.egui_ctx, |ui| {
                viewport = ui.min_rect();

                ui.painter()
                    .rect_stroke(viewport, 0.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE));
            })
            .response.interact(egui::Sense::drag());
        // 1. Handle Zooming (Scroll)
        let scroll_delta = self.egui_ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if let Some(mouse_pos) = self.egui_ctx.input(|i| i.pointer.hover_pos()) {
                let zoom_factor = (scroll_delta as f64 * 0.001).exp();

                let mouse_vec = vello::kurbo::Vec2::new(
                    mouse_pos.x as f64 - viewport.min.x as f64 - self.view_transform.pan.x,
                    mouse_pos.y as f64 - viewport.min.y as f64 - self.view_transform.pan.y,
                );

                let new_scale = self.view_transform.scale * zoom_factor;
                self.view_transform.pan -= mouse_vec * (zoom_factor - 1.0);
                self.view_transform.scale = new_scale;
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.view_transform.pan.x += delta.x as f64;
            self.view_transform.pan.y += delta.y as f64;
        }

        viewport
    }

    fn render_vello(&mut self, viewport: egui::Rect) {
        let scene = &mut self.scene;
        scene.reset();

        let offset_x = viewport.min.x as f64;
        let offset_y = viewport.min.y as f64;

        let rect = Rect::new(offset_x + 500.0, offset_y + 500.0, offset_x + 750.0, offset_y + 750.0);

        scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(255, 255, 255), None, &rect);

        let circle = vello::kurbo::Circle::new((offset_x + 450.0, offset_y + 120.0), 70.0);

        scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(38, 139, 210), None, &circle);

        let mut triangle = vello::kurbo::BezPath::new();

        triangle.move_to((offset_x + 200.0, offset_y + 260.0));
        triangle.line_to((offset_x + 450.0, offset_y + 500.0));
        triangle.line_to((offset_x + 50.0, offset_y + 500.0));
        triangle.close_path();

        scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(133, 153, 0), None, &triangle);
    }
    fn render_command_palette(&mut self) {
        egui::Window::new("Command Palette")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .fixed_size([500.0, 40.0])
            .show(&self.egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(": {}", self.input_handler.buffer))
                            .strong()
                            .size(20.0)
                            .color(egui::Color32::LIGHT_BLUE),
                    );
                });
            });
    }
    fn process_command(&mut self, command: Command) {
        match command {
            Command::LoadGraph => {
                let scene = if let Some(graph) = &self.router.current_graph {
                    self.router
                        .current_tile_manager
                        .as_ref()
                        .map(|tile_manager| build_fabric_scene(graph, tile_manager))
                } else {
                    None
                };
                self.fabric_scene = scene;
            }
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

    fn check_background_tasks(&mut self) {
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
    fn render_loading(&self) {
        if let LoadStatus::Loading(message) = &self.load_status {
            egui::Window::new("Loading").show(&self.egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(message);
                });
            });
        }
    }

    fn render(&mut self) {
        if let Some(command) = self.input_handler.pop_command() {
            self.process_command(command);
        }

        self.check_background_tasks();

        self.render_loading();

        let output = self.surface.get_current_texture().unwrap();

        let view = output.texture.create_view(&TextureViewDescriptor::default());

        // --- egui begin frame ---
        let raw_input = self.egui_state.take_egui_input(&self.window);

        self.egui_ctx.begin_frame(raw_input);

        let viewport = self.ui();

        let full_output = self.egui_ctx.end_frame();

        let paint_jobs = self.egui_ctx.tessellate(full_output.shapes, self.egui_ctx.pixels_per_point());

        // Upload egui textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // --- render vello ---
        self.scene.reset();

        let transform = vello::kurbo::Affine::translate((
            viewport.min.x as f64 + self.view_transform.pan.x,
            viewport.min.y as f64 + self.view_transform.pan.y,
        )) * vello::kurbo::Affine::scale(self.view_transform.scale);

        if let Some(ref fabric) = self.fabric_scene {
            // Append the fabric scene (tiles/luts) with the viewport offset
            self.scene.append(fabric, Some(transform));
        } else {
            // Fallback to your placeholder shapes if no graph is loaded
            self.render_vello(viewport);
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
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("FPGA Router")
            .with_inner_size(PhysicalSize::new(1280, 720))
            .build(&event_loop)
            .unwrap(),
    );

    let mut app = pollster::block_on(App::new(window.clone()));

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, window_id } if window_id == app.window.id() => {
                let response = app.egui_state.on_window_event(&app.window, &event);

                if response.consumed {
                    return;
                }

                match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }

                    WindowEvent::Resized(size) => {
                        app.resize(size);
                    }

                    WindowEvent::RedrawRequested => {
                        app.render();
                    }
                    WindowEvent::KeyboardInput { event, .. } if !event.state.is_pressed() => match event.logical_key {
                        Key::Character(ref c) if c == ":" => {
                            app.input_handler.state = InputHandlerState::Command;
                        }
                        Key::Named(named_key) => app.input_handler.handle_named_key(named_key),
                        Key::Character(c) => app.input_handler.handle_char(c),
                        _ => {}
                    },
                    _ => {}
                }
            }

            Event::AboutToWait => {
                app.window.request_redraw();
            }

            _ => {}
        })
        .unwrap();
}

impl InputHandler {
    fn pop_command(&mut self) -> Option<Command> {
        self.command_queue.pop_front()
    }
    fn handle_named_key(&mut self, key: NamedKey) {
        match key {
            NamedKey::Escape if self.state == InputHandlerState::Command => {
                self.buffer.clear();
                self.state = InputHandlerState::Idle;
            }
            NamedKey::Space => self.handle_char(SmolStr::new(" ")),
            NamedKey::Backspace => {
                self.buffer.pop();
            }
            NamedKey::Enter if self.state == InputHandlerState::Command => {
                if let Some(command) = Command::parse_command(&self.buffer) {
                    self.command_queue.push_back(command);
                }
                self.state = InputHandlerState::Idle;
                self.buffer.clear();
            }
            _ => {}
        }
    }
    fn handle_char(&mut self, input: SmolStr) {
        match self.state {
            InputHandlerState::Command => {
                self.buffer += &input;
            }
            InputHandlerState::Idle => {
                if input == ":" {
                    self.state = InputHandlerState::Command;
                }
            }
        }
    }
}

#[derive(Default)]
struct InputHandler {
    buffer: String,
    state: InputHandlerState,
    command_queue: VecDeque<Command>,
}

#[derive(Default, Eq, PartialEq)]
enum InputHandlerState {
    Command,
    #[default]
    Idle,
}

#[derive(Debug)]
enum Command {
    Next(usize),
    LoadBel(String),
    LoadPips(String),
    LoadGraph,
    Pause,
}
impl Command {
    fn parse_command(command: &str) -> Option<Self> {
        println!("parsing command: {command}");
        let mut full_command = command.split_whitespace();
        if let Some(m) = full_command.next() {
            return match m {
                "next" => Self::parse_next(full_command),
                "load-graph" => Some(Self::LoadGraph),
                "pause" => Some(Self::Pause),
                "load-bel" => Self::parse_load_bel(full_command),
                "load-pips" => Self::parse_load_pips(full_command),
                _ => None,
            };
        }
        None
    }
    fn parse_next(mut arguments: SplitWhitespace) -> Option<Self> {
        arguments
            .next()
            .and_then(|amount| amount.parse::<usize>().ok())
            .map(Self::Next)
    }
    fn parse_load_bel(mut arguments: SplitWhitespace) -> Option<Self> {
        arguments.next().map(|file| Self::LoadBel(file.to_string()))
    }
    fn parse_load_pips(mut arguments: SplitWhitespace) -> Option<Self> {
        arguments.next().map(|file| Self::LoadPips(file.to_string()))
    }
}

enum LoadStatus {
    Loading(String),
    Idle,
}

fn get_tile_pos(tile: &TileId) -> (f64, f64) {
    (
        tile.0 as f64 * (TILE_WIDTH + TILE_PADDING),
        tile.1 as f64 * (TILE_WIDTH + TILE_PADDING),
    )
}
const TILE_WIDTH: f64 = 110.0;
const TILE_HEIGHT: f64 = 100.0;
const TILE_PADDING: f64 = 20.0;

const LUT_WIDTH: f64 = 20.0;
const LUT_HEIGHT: f64 = 15.0;
const LUT_MARGIN: f64 = 10.0;
const LUT_SPACING: f64 = 9.0;

const PIN_LEN: f64 = 1.0;

fn build_fabric_scene(graph: &FabricGraph, tile_manager: &TileManager) -> Scene {
    let mut scene = vello::Scene::new();

    for (tile_id, tile) in &tile_manager.0 {
        let (tx, ty) = get_tile_pos(tile_id);

        // 1. Draw Tile Boundary
        let rect = vello::kurbo::Rect::new(tx, ty, tx + TILE_WIDTH, ty + TILE_HEIGHT);
        scene.fill(
            vello::peniko::Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            vello::peniko::Color::rgb8(25, 25, 25), 
            None,
            &rect,
        );

        // 2. Draw LUTs inside the tile
        let luts_per_row = ((TILE_WIDTH - (2.0 * LUT_MARGIN)) / (LUT_WIDTH + LUT_SPACING))
            .floor()
            .max(1.0) as usize;

        for (i, lut) in tile.luts.iter().enumerate() {
            let row = i / luts_per_row;
            let col = i % luts_per_row;

            let lx = tx + LUT_MARGIN + (col as f64 * (LUT_WIDTH + LUT_SPACING));
            let ly = ty + LUT_MARGIN + (row as f64 * (LUT_HEIGHT + LUT_SPACING));
            let lut_rect = vello::kurbo::Rect::new(lx, ly, lx + LUT_WIDTH, ly + LUT_HEIGHT);

            scene.stroke(
                &vello::kurbo::Stroke::new(1.5),
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::rgb8(100, 100, 110),
                None,
                &lut_rect,
            );

            let num_inputs = lut.input_pin.len();
            for (j, (pin_name, _state)) in lut.input_pin.iter().enumerate() {
                let spacing = LUT_HEIGHT / (num_inputs as f64 + 1.0);
                let py = ly + (spacing * (j as f64 + 1.0));

                let line = vello::kurbo::Line::new((lx - PIN_LEN, py), (lx, py));
                scene.stroke(
                    &vello::kurbo::Stroke::new(0.5),
                    vello::kurbo::Affine::IDENTITY,
                    vello::peniko::Color::WHITE,
                    None,
                    &line,
                );
            }

            let py = ly + LUT_HEIGHT/2.0;
            let out_line = vello::kurbo::Line::new((lx + LUT_WIDTH, py), (lx + LUT_WIDTH + PIN_LEN, py));
            scene.stroke(
                &vello::kurbo::Stroke::new(0.5),
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::rgb8(0, 255, 150), 
                None,
                &out_line,
            );
        }
    }
    // Output Label would be at (lx + LUT_WIDTH + PIN_LEN + 2.0, oy)
    scene
}

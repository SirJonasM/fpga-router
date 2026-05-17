use egui::{CentralPanel, SidePanel};
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use std::sync::Arc;
use vello::peniko::{Color, Fill};
use vello::{Renderer, RendererOptions, Scene};

use vello::kurbo::{Affine, BezPath, Circle, Point, Rect};
use wgpu::RequestAdapterOptions;
use wgpu::{
    CommandEncoderDescriptor, DeviceDescriptor, Features, Instance, Limits, LoadOp, Operations, PowerPreference, PresentMode,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
};
use winit::window::Window;
use winit::{dpi::PhysicalSize, event::*, event_loop::EventLoop, window::WindowBuilder};

struct App {
    window: Arc<Window>,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: Queue,
    config: SurfaceConfiguration,

    egui_state: EguiWinitState,
    egui_ctx: egui::Context,
    egui_renderer: EguiRenderer,

    vello_renderer: Renderer,
    scene: Scene,
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
            .resizable(false)
            .default_width(200.0)
            .show(&self.egui_ctx, |ui| {
                ui.heading("egui Controls");

                if ui.button("Button 1").clicked() {
                    println!("Button 1 clicked");
                }

                if ui.button("Button 2").clicked() {
                    println!("Button 2 clicked");
                }

                ui.separator();

                ui.label("Hello from egui!");
            });

        let mut viewport = egui::Rect::NOTHING;

        CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(&self.egui_ctx, |ui| {
                viewport = ui.min_rect();

                ui.painter()
                    .rect_stroke(viewport, 0.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE));
            });

        viewport
    }

    fn render_vello(&mut self, viewport: egui::Rect) {
        self.scene.reset();

        let offset_x = viewport.min.x as f64;
        let offset_y = viewport.min.y as f64;

        //
        // Rectangle
        //
        let rect = Rect::new(offset_x + 50.0, offset_y + 50.0, offset_x + 250.0, offset_y + 180.0);

        self.scene
            .fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(255,255,255), None, &rect);

        //
        // Circle
        //
        let circle = vello::kurbo::Circle::new((offset_x + 450.0, offset_y + 120.0), 70.0);

        self.scene
            .fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(38, 139, 210), None, &circle);

        //
        // Triangle
        //
        let mut triangle = vello::kurbo::BezPath::new();

        triangle.move_to((offset_x + 200.0, offset_y + 260.0));
        triangle.line_to((offset_x + 450.0, offset_y + 500.0));
        triangle.line_to((offset_x + 50.0, offset_y + 500.0));
        triangle.close_path();

        self.scene
            .fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(133, 153, 0), None, &triangle);
    }
    fn render(&mut self) {
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
        self.render_vello(viewport);

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
            .with_title("Vello + egui + wgpu")
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

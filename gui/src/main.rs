mod app;
mod constants;
mod gui;
mod input;
mod layout;
mod render;

use std::sync::Arc;
use winit::keyboard::Key;
use winit::{dpi::PhysicalSize, event::*, event_loop::EventLoop, window::WindowBuilder};

use crate::app::App;
use crate::input::InputHandlerState;

#[cfg(feature = "big")]
pub const XXXXXX: usize = 8;
#[cfg(not(feature = "big"))]
pub const XXXXXX: usize = 4;

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
                    WindowEvent::KeyboardInput { event, .. } if !event.state.is_pressed() => {
                        match event.logical_key {
                            Key::Character(ref c) if c == ":" => {
                                app.input_handler.state = InputHandlerState::Command;
                            }
                            Key::Named(named_key) => app.input_handler.handle_named_key(named_key),
                            Key::Character(c) => app.input_handler.handle_char(c),
                            _ => {}
                        }
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

#[derive(Default)]
enum LoadStatus {
    Loading(String),
    #[default]
    Idle,
}

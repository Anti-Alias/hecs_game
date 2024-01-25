use std::time::{SystemTime, Duration};
use glam::Vec2;
use wgpu::TextureFormat;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceEvent, ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Window, WindowBuilder};
use crate::{App, AppBuilder, AppRunner, Cursor, GraphicsState, InputRequest, InputRequests, Keyboard, Plugin};

/// Opens a window and injects a [`GraphicsState`] for use in a graphics engine.
/// Adds a runner that is synced with the framerate.
pub struct WindowPlugin {
    frame_rate: u32,
    window_width: u32,
    window_height: u32,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            frame_rate: 60,
            window_width: 512,
            window_height: 512
        }
    }
}

impl Plugin for WindowPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        let event_loop = EventLoopBuilder::<()>::with_user_event().build().unwrap();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        builder.game().init(|_| GraphicsState::new(&window, TextureFormat::Depth24Plus));
        builder.runner(WindowRunner {
            frame_rate: self.frame_rate,
            window_width: self.window_width,
            window_height: self.window_height,
            event_loop: Some(event_loop),
            window,
        });
    }
}

/**
 * Opens a window and uses it to power an underlying [`App`].
 * For rendering applications on Windows, Linux and OSX.
 */
pub struct WindowRunner {
    frame_rate: u32,
    window_width: u32,
    window_height: u32,
    event_loop: Option<EventLoop::<()>>,
    window: Window,
}

impl WindowRunner {
    
    /// Desired frame rate when in exclusive fullscreen mode.
    pub fn with_frame_rate(mut self, frame_rate: u32) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    /// Default window size to use when in windowed mode.
    pub fn with_window_size(mut self, width: u32, height: u32) -> Self {
        self.window_width = width;
        self.window_height = height;
        self
    }
}

impl AppRunner for WindowRunner {
    fn run(&mut self, mut app: App) {

        let event_loop = self.event_loop.take().unwrap();
        let window = &mut self.window;
        
        // Starts game loop
        let mut last_update: Option<SystemTime> = None;
        event_loop.run(move |event, target| {
            match event {
                Event::WindowEvent { event, .. } => handle_window_event(
                    event,
                    &window,
                    target,
                    &mut app,
                    &window,
                    &mut last_update
                ),
                Event::DeviceEvent { event, .. } => handle_device_event(event, &mut app),
                _ => {}
            }
        }).unwrap();
    }
}

fn handle_window_event(
    event: WindowEvent,
    _window: &Window,
    target: &EventLoopWindowTarget<()>,
    app: &mut App,
    window: &Window,
    last_update: &mut Option<SystemTime>,
) {
    match event {
        WindowEvent::Resized(size) => {
            app.game
                .get::<&mut GraphicsState>()
                .resize(size.width, size.height)
        },
        WindowEvent::KeyboardInput { event, .. } => {
            let key_code = match event.physical_key {
                PhysicalKey::Code(key_code) => key_code,
                PhysicalKey::Unidentified(_) => return,
            };
            let mut keyboard = app.game.get::<&mut Keyboard>();
            match event.state {
                ElementState::Pressed => keyboard.press(key_code),
                ElementState::Released => keyboard.release(key_code),
            }
        },
        WindowEvent::CursorMoved { position, .. } => {
            let mut cursor = app.game.get::<&mut Cursor>();
            cursor.position = Vec2::new(position.x as f32, position.y as f32);
        },
        WindowEvent::RedrawRequested => {
            run_game_logic(app, last_update, window, target);   // Game logic
            window.request_redraw();                            // Submits request to render next frame
        },
        WindowEvent::CloseRequested => target.exit(),
        _ => {}
    }
}

fn handle_device_event(event: DeviceEvent, app: &mut App) {
    match event {
        DeviceEvent::MouseMotion { delta } => {
            let mut cursor = app.game.get::<&mut Cursor>();
            cursor.movement += Vec2::new(delta.0 as f32, delta.1 as f32);
        },
        _ => {}
    }
}

fn run_game_logic<'a>(
    app: &'a mut App,
    last_update: &mut Option<SystemTime>,
    window: &Window,
    target: &EventLoopWindowTarget<()>,
) {
    // Computes delta since last frame.
    let now = SystemTime::now();
    let delta = match *last_update {
        Some(last_update) => now.duration_since(last_update).unwrap(),
        None => Duration::ZERO,
    };
    *last_update = Some(now);

    // Runs logic and handles
    app.run_frame(delta);

    // Handles quit request
    if app.quit_requested {
        target.exit();
    }

    // Handles input requests
    let mut requests = app.game.take::<InputRequests>();
    let mut cursor = app.game.get::<&mut Cursor>();
    while let Some(request) = requests.pop() {
        match request {
            InputRequest::SetCursorPosition(position) => {
                let position = PhysicalPosition::new(position.x as i32, position.y as i32);
                if let Err(_) = window.set_cursor_position(position) {
                    log::error!("Failed to set cursor position");
                    continue;
                }
            },
            InputRequest::HideCursor => {
                cursor.is_visible = false;
                window.set_cursor_visible(false);
            },
            InputRequest::ShowCursor => {
                cursor.is_visible = true;
                window.set_cursor_visible(true);
            },
            InputRequest::GrabCursor => {
                if let Err(_) = window.set_cursor_grab(CursorGrabMode::Confined) {
                    if let Err(_) = window.set_cursor_grab(CursorGrabMode::Locked) {
                        log::error!("Failed to hide cursor");
                        continue;
                    }
                }
                cursor.is_grabbed = true;
            },
            InputRequest::UngrabCursor => {
                if let Err(_) = window.set_cursor_grab(CursorGrabMode::None) {
                    log::error!("Failed to show cursor");
                }
                cursor.is_grabbed = false;
            },
        }
    }
}
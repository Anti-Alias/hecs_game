use std::time::{SystemTime, Duration};
use glam::Vec2;
use wgpu::TextureFormat;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget};
use winit::keyboard::PhysicalKey;
use winit::monitor::{MonitorHandle, VideoMode};
use winit::window::{CursorGrabMode, Fullscreen, Window as WinitWindow, WindowBuilder};
use crate::{App, AppBuilder, AppRunner, Cursor, GraphicsState, Keyboard, Plugin, WindowRequest, WindowRequests};

/// Opens a window and injects a [`GraphicsState`] for use in a graphics engine.
/// Adds a runner that is synced with the framerate.
pub struct WindowPlugin {
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            window_width: 512,
            window_height: 512
        }
    }
}

impl Plugin for WindowPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        let event_loop = EventLoopBuilder::<()>::with_user_event().build().unwrap();
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(self.window_width, self.window_height))
            .build(&event_loop).unwrap();
        let current_monitor = window.current_monitor().expect("Failed to get current monitor");
        let mut inner_window = Window::new(current_monitor);
        for monitor in window.available_monitors() {
            for video_mode in monitor.video_modes() {
                inner_window.video_modes.push((monitor.clone(), video_mode));
            }
        }
        builder.game()
            .add(GraphicsState::new(&window, TextureFormat::Depth24Plus))
            .add(inner_window);
        builder.runner(WindowRunner {
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
    event_loop: Option<EventLoop::<()>>,
    window: WinitWindow,
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

/// Window domain
pub struct Window {
    /// Current fullscreen state
    pub fullscreen: Option<Fullscreen>,
    /// All video modes supported across all monitors.
    pub video_modes: Vec<(MonitorHandle, VideoMode)>,
    /// Monitor this window resides on.
    pub current_monitor: MonitorHandle,
    /// Size of the window's inner content
    pub(crate) size: Vec2,
}

impl Window {

    pub(crate) fn new(current_monitor: MonitorHandle) -> Self {
        Self {
            fullscreen: None,
            video_modes: Vec::new(),
            current_monitor,
            size: Vec2::ZERO,
        }
    }

    pub fn fullscreen(&self) -> Option<&Fullscreen> {
        self.fullscreen.as_ref()
    }

    /// Video modes of all monitors
    pub fn video_modes(&self) -> impl Iterator<Item = &(MonitorHandle, VideoMode)> {
        self.video_modes.iter()
    }

    /// Video modes of current monitor
    pub fn current_video_modes(&self) -> impl Iterator<Item = &VideoMode> {
        self.video_modes
            .iter()
            .filter_map(|(handle, mode)| {
                if handle != &self.current_monitor {
                    return None;
                }
                Some(mode)
            })
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }
}

fn handle_window_event(
    event: WindowEvent,
    _window: &WinitWindow,
    target: &EventLoopWindowTarget<()>,
    app: &mut App,
    window: &WinitWindow,
    last_update: &mut Option<SystemTime>,
) {
    match event {
        WindowEvent::Resized(size) => {
            let mut inner_window = app.game.get::<&mut Window>();
            inner_window.size = Vec2::new(size.width as f32, size.height as f32);
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
        WindowEvent::MouseWheel { delta, .. } => {
            let mut cursor = app.game.get::<&mut Cursor>();
            match delta {
                MouseScrollDelta::LineDelta(dx, dy) => {
                    cursor.scroll.x += dx;
                    cursor.scroll.y += dy;
                },
                MouseScrollDelta::PixelDelta(position) => {
                    cursor.scroll.x += position.x as f32;
                    cursor.scroll.y += position.y as f32;
                },
            }
        }
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
    window: &WinitWindow,
    target: &EventLoopWindowTarget<()>,
) {
    // Computes delta since last frame.
    let now = SystemTime::now();
    let delta = match *last_update {
        Some(last_update) => now.duration_since(last_update).unwrap(),
        None => Duration::ZERO,
    };
    *last_update = Some(now);

    // Updates window's current monitor
    {
        let mut inner_window = app.game.get::<&mut Window>();
        let current_monitor = window.current_monitor().expect("Could not acquire window's current monitor");
        if inner_window.current_monitor != current_monitor {
            inner_window.current_monitor = current_monitor;
        }
    }

    // Runs logic and handles
    app.run_frame(delta);

    // Handles quit request
    if app.quit_requested {
        target.exit();
    }

    // Handles input requests
    let mut requests = app.game.take::<WindowRequests>();
    let mut cursor = app.game.get::<&mut Cursor>();
    while let Some(request) = requests.pop() {
        match request {
            WindowRequest::SetCursorPosition(position) => {
                let position = PhysicalPosition::new(position.x as i32, position.y as i32);
                if let Err(_) = window.set_cursor_position(position) {
                    log::error!("Failed to set cursor position");
                    continue;
                }
            },
            WindowRequest::SetCursorVisible(true) => {
                cursor.is_visible = true;
                window.set_cursor_visible(true);
            },
            WindowRequest::SetCursorVisible(false) => {
                cursor.is_visible = false;
                window.set_cursor_visible(false);
            },
            WindowRequest::SetCursorGrab(true) => {
                if let Err(_) = window.set_cursor_grab(CursorGrabMode::Confined) {
                    if let Err(_) = window.set_cursor_grab(CursorGrabMode::Locked) {
                        log::error!("Failed to hide cursor");
                        continue;
                    }
                }
                cursor.is_grabbed = true;
            },
            WindowRequest::SetCursorGrab(false) => {
                if let Err(_) = window.set_cursor_grab(CursorGrabMode::None) {
                    log::error!("Failed to show cursor");
                }
                cursor.is_grabbed = false;
            },
            WindowRequest::SetFullscreen(fullscreen) => {
                let mut inner_window = app.game.get::<&mut Window>();
                window.set_fullscreen(fullscreen.clone());
                inner_window.fullscreen = fullscreen;
            },
            
        }
    }
}
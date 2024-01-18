use std::time::{SystemTime, Duration};
use winit::event::{WindowEvent, Event, ElementState};
use winit::event_loop::{EventLoop, EventLoopWindowTarget};
use winit::keyboard::PhysicalKey;
use winit::window::{WindowBuilder, Window};
use crate::{App, Input, ExternalRequest, GraphicsState, AppBuilder, AppRunner, Plugin};

/// Opens a window and injects a [`GraphicsState`] for use in a graphics engine.
/// Adds a runner that is synced with the framerate.
pub struct WinitPlugin {
    frame_rate: u32,
    window_width: u32,
    window_height: u32,
}

impl Default for WinitPlugin {
    fn default() -> Self {
        Self {
            frame_rate: 60,
            window_width: 512,
            window_height: 512
        }
    }
}

impl Plugin for WinitPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        builder.game()
            .init(|_| Input::new())
            .init(|_| GraphicsState::new(&window));
        builder.runner(WinitRunner {
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
pub struct WinitRunner {
    frame_rate: u32,
    window_width: u32,
    window_height: u32,
    event_loop: Option<EventLoop<()>>,
    window: Window,
}

impl WinitRunner {
    
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

impl AppRunner for WinitRunner {
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
            let mut input = app.game.get::<&mut Input>();
            match event.state {
                ElementState::Pressed => input.keyboard.press(key_code),
                ElementState::Released => input.keyboard.release(key_code),
            }
        },
        WindowEvent::RedrawRequested => {
            run_game_logic(app, last_update, target);               // Game logic
            app.game.get::<&mut Input>().sync_previous_state();     // Sync Input with previous state
            window.request_redraw();                                // Submits request to render next frame
        },
        WindowEvent::CloseRequested => target.exit(),
        _ => {}
    }
}

fn run_game_logic<'a>(
    app: &'a mut App,
    last_update: &mut Option<SystemTime>,
    target: &EventLoopWindowTarget<()>,
) {
    // Computes delta since last frame.
    let now = SystemTime::now();
    let delta = match *last_update {
        Some(last_update) => now.duration_since(last_update).unwrap(),
        None => Duration::ZERO,
    };
    *last_update = Some(now);

    // Runs logic and handles outgoing requests
    for request in app.run_frame(delta) {
        match request {
            ExternalRequest::Quit => target.exit(),
        }
    }
}
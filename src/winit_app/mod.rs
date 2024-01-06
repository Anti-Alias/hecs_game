use std::time::SystemTime;
use winit::event::{WindowEvent, Event, ElementState};
use winit::event_loop::{EventLoop, EventLoopWindowTarget};
use winit::keyboard::PhysicalKey;
use winit::window::{WindowBuilder, Window};
use crate::{App, Input, ExternalRequest, GraphicsState};

/**
 * Opens a window and uses it to power an underlying [`App`].
 * For rendering applications on Windows, Linux and OSX.
 */
pub struct WinitApp {
    game_runner: App,
    frame_rate: u32,
    window_width: u32,
    window_height: u32,
}

impl WinitApp {
    
    pub fn new(game_runner: App) -> Self {
        Self {
            game_runner,
            frame_rate: 60,
            window_width: 16*50,
            window_height: 9*50,
        }
    }

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

    pub fn run(mut self) {

        // Sets up event loop and window
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new().build(&event_loop).unwrap();

        // Configures game
        let game = &mut self.game_runner.game;
        game.init(|| Input::new());
        game.init(|| GraphicsState::new(&window));

        // Starts game loop
        let mut last_update: Option<SystemTime> = None;
        event_loop.run(move |event, target| {
            match event {
                Event::WindowEvent { event, .. } => handle_window_event(event, &window, target, &mut self.game_runner, &mut last_update),
                _ => {}
            }
        }).unwrap();
    }
}

fn handle_window_event(
    event: WindowEvent,
    _window: &Window,
    target: &EventLoopWindowTarget<()>,
    runner: &mut App,
    last_update: &mut Option<SystemTime>,
) {
    match event {
        WindowEvent::Resized(size) => {
            let mut graphics_state = runner.game.get_mut::<GraphicsState>();
            graphics_state.resize(size.width, size.height)
        },
        WindowEvent::KeyboardInput { event, .. } => {
            let mut input = runner.game.get_mut::<Input>();
            let key_code = match event.physical_key {
                PhysicalKey::Code(key_code) => key_code,
                PhysicalKey::Unidentified(_) => return,
            };
            match event.state {
                ElementState::Pressed => input.keyboard.press(key_code),
                ElementState::Released => input.keyboard.release(key_code),
            }
        },
        WindowEvent::RedrawRequested => {
            run_game_logic(runner, last_update, target);
            runner.game
                .get_mut::<Input>()
                .sync_previous_state();
        },
        WindowEvent::CloseRequested => target.exit(),
        _ => {}
    }
}

fn run_game_logic<'a>(
    runner: &'a mut App,
    last_update: &mut Option<SystemTime>,
    target: &EventLoopWindowTarget<()>,
) {
    let now = SystemTime::now();
    let requests = match *last_update {
        Some(last) => {
            let delta = now.duration_since(last).unwrap();
            runner.run_frame(delta)
        },
        None => runner.run_tick(),
    };
    *last_update = Some(now);
    
    for request in requests {
        match request {
            ExternalRequest::Quit => target.exit(),
        }
    }
}
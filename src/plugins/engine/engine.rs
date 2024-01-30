use std::time::Duration;
use winit::keyboard::KeyCode;
use winit::monitor::{MonitorHandle, VideoMode};
use winit::window::Fullscreen;
use crate::g3d::{GpuMaterial, GpuMesh};
use crate::{AppBuilder, AssetManager, AssetPlugin, EcsPlugin, Game, GraphicsPlugin, InputPlugin, Keyboard, Plugin, RunContext, Stage, Window, WindowFeatures, WindowPlugin, WindowRequests};

/**
 * Main game engine plugin.
 */
pub struct EnginePlugin {
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for EnginePlugin {
    fn default() -> Self {
        Self {
            window_width: 512,
            window_height: 512
        }
    }
}

impl Plugin for EnginePlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        //env_logger::init();

        builder
            .plugin(InputPlugin)
            .plugin(WindowPlugin {
                window_width: self.window_width,
                window_height: self.window_height,
                features: WindowFeatures::default(),
            })
            .plugin(EcsPlugin)
            .plugin(AssetPlugin)
            .plugin(GraphicsPlugin)
            .tick_duration(Duration::from_secs_f64(1.0/60.0));
        builder.system(Stage::PreUpdate, toggle_fullscreen);

        let game = builder.game();
        let mut assets = game.get::<&mut AssetManager>();
        assets.add_storage::<GpuMesh>();
        assets.add_storage::<GpuMaterial>();
    }
}

fn toggle_fullscreen(game: &mut Game, _ctx: RunContext) {
    let keyboard = game.get::<&Keyboard>();
    let window = game.get::<&Window>();
    let mut requests = game.get::<&mut WindowRequests>();
    if keyboard.is_pressed(KeyCode::AltLeft) && keyboard.is_just_pressed(KeyCode::Enter) {
        match window.fullscreen() {
            Some(_) => {
                requests.set_fullscreen(None)
            },
            None => {
                let Some(current_monitor) = window.current_monitor() else { return };
                let video_mode = select_fullscreen_mode(current_monitor, window.current_video_modes());
                if let Some(video_mode) = video_mode {
                    requests.set_fullscreen(Some(Fullscreen::Exclusive(video_mode.clone())))
                }
                else {
                    log::error!("Failed to select video mode");
                }
            },
        }
    }
}

fn select_fullscreen_mode<'a, I>(monitor: &MonitorHandle, video_modes: I) -> Option<&'a VideoMode>
where
    I: Iterator<Item = &'a VideoMode>
{
    let hertz = monitor.refresh_rate_millihertz().unwrap_or(60000);
    video_modes
        .filter(|mode| mode.size() == monitor.size())
        .filter(|mode| mode.refresh_rate_millihertz() == hertz)
        .next()
}
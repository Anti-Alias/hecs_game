use std::time::Duration;
use winit::keyboard::KeyCode;
use winit::monitor::{MonitorHandle, VideoMode};
use winit::window::Fullscreen;

use crate::{AppBuilder, AssetPlugin, EcsPlugin, Game, GraphicsPlugin, InputPlugin, Keyboard, Plugin, RunContext, Stage, Window, WindowPlugin, WindowRequests};

/**
 * Main game engine plugin.
 */
pub struct EnginePlugin;
impl Plugin for EnginePlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder
            .plugin(InputPlugin)
            .plugin(WindowPlugin::default())
            .plugin(EcsPlugin)
            .plugin(AssetPlugin)
            .plugin(GraphicsPlugin)
            .tick_duration(Duration::from_secs_f64(1.0/60.0));
        builder.system(Stage::PreUpdate, toggle_fullscreen);
    }
}

fn toggle_fullscreen(game: &mut Game, _ctx: RunContext) {
    let keyboard = game.get::<&Keyboard>();
    let window = game.get::<&Window>();
    let mut requests = game.get::<&mut WindowRequests>();
    
    if keyboard.is_pressed(KeyCode::AltLeft) && keyboard.is_just_pressed(KeyCode::Enter) {
        match window.fullscreen() {
            Some(_) => requests.set_fullscreen(None),
            None => {
                let video_mode = select_fullscreen_mode(&window.current_monitor, window.current_video_modes());
                let Some(video_mode) = video_mode else {
                    log::error!("Failed to select video mode");
                    return;
                };
                requests.set_fullscreen(Some(Fullscreen::Exclusive(video_mode.clone())))
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
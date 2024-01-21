use std::time::Duration;
use crate::{AppBuilder, Plugin, WindowPlugin, GraphicsPlugin, InputPlugin, EcsPlugin, AssetPlugin};

/**
 * Main game-engine plugin.
 */
pub struct ClientPlugin;
impl Plugin for ClientPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder
            .plugin(InputPlugin)
            .plugin(WindowPlugin::default())
            .plugin(EcsPlugin)
            .plugin(AssetPlugin)
            .plugin(GraphicsPlugin)
            .tick_duration(Duration::from_secs_f64(1.0/60.0));        
    }
}
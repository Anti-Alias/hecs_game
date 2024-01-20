use std::time::Duration;
use crate::core::CorePlugin;
use crate::{AppBuilder, Plugin, WinitPlugin, GraphicsPlugin};

/**
 * Main game-engine plugin.
 */
pub struct EnginePlugin;
impl Plugin for EnginePlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder
            .plugin(CorePlugin)
            .plugin(WinitPlugin::default())
            .plugin(GraphicsPlugin)
            .tick_duration(Duration::from_secs_f64(1.0/60.0));        
    }
}
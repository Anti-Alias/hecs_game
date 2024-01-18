use std::time::Duration;
use crate::core::CorePlugin;
use crate::{AppBuilder, Plugin, WinitPlugin, GraphicsPlugin};
const TICK_DURATION: Duration = Duration::from_secs(1);

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
            .tick_duration(TICK_DURATION);        
    }
}
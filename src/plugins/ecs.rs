use hecs::World;
use crate::{App, Plugin};

pub struct EcsPlugin;
impl Plugin for EcsPlugin {
    fn install(&mut self, app: &mut App) {
        app.game.init(|_| World::new());
    }
}
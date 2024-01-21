use hecs::World;
use crate::Plugin;

pub struct EcsPlugin;
impl Plugin for EcsPlugin {
    fn install(&mut self, builder: &mut crate::AppBuilder) {
        builder.game().init(|_| World::new());
    }
}
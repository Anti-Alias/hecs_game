use hecs::World;
use crate::{Plugin, AssetManager, FileProtocol};

pub struct CorePlugin;
impl Plugin for CorePlugin {
    fn install(&mut self, builder: &mut crate::AppBuilder) {
        builder.game()
            .init(|_| World::new())
            .init(|_| AssetManager::builder()
                .default_protocol(FileProtocol)
                .build()
            );
    }
}
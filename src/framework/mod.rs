mod game;
mod app;
mod script;

pub use game::*;
pub use app::*;
use hecs::World;
pub use script::*;
use crate::{AssetManager, FileProtocol};

/**
 * Adds core functionality common across all apps.
 */
pub fn core_plugin(app: &mut AppConfig) {
    app.game()
        .init(|| World::new())
        .init(|| AssetManager::builder()
            .default_protocol(FileProtocol)
            .build()
        );
}
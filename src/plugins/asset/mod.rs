mod asset;
mod manager;
mod loader;
mod handle;
mod protocol;
mod path_parts;

pub use asset::*;
pub use manager::*;
pub use loader::*;
pub use handle::*;
pub use protocol::*;
pub use path_parts::*;

use crate::{Plugin, AppBuilder};

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        let manager = AssetManager::default();
        manager.add_protocol(FileProtocol, true);
        builder.game().add(manager);
    }
}
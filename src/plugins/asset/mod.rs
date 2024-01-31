mod storage;
mod asset;
mod protocol;
mod path_parts;
mod loader;
mod manager;

pub use storage::*;
pub use asset::*;
pub use protocol::*;
pub use path_parts::*;
pub use loader::*;
pub use manager::*;

use crate::{AppBuilder, Game, Plugin, RunContext, Stage};


pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        let mut manager = AssetManager::new();
        manager.add_protocol(FileProtocol, true);
        builder.game().add(manager);
        builder.system(Stage::Asset, handle_asset_messages);
    }
}

fn handle_asset_messages(game: &mut Game, _ctx: RunContext) {
    let mut assets = game.get::<&mut AssetManager>();
    assets.set_path_prefix(Some("assets"));
    assets.try_handle_messages();
}
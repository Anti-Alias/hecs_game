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

use crate::{App, Game, Plugin, RunContext, Stage};


pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn install(&mut self, app: &mut App) {
        let mut manager = AssetManager::new();
        manager.add_protocol(FileProtocol, true);
        app.game.add(manager);
        app.add_system(Stage::Asset, handle_asset_messages);
    }
}

fn handle_asset_messages(game: &mut Game, _ctx: RunContext) {
    let mut assets = game.get::<&mut AssetManager>();
    assets.set_path_prefix(Some("assets"));
    assets.try_handle_messages();
}
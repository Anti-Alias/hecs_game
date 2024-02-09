mod map;
mod tile;
mod tileset;
mod layer;
mod parse;

pub use map::*;
pub use tile::*;
pub use tileset::*;
pub use layer::*;

use crate::{App, AssetManager, Plugin};

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn install(&mut self, app: &mut App) {
        let mut assets = app.game.get::<&mut AssetManager>();
        assets.add_storage::<TiledMap>();
        assets.add_storage::<Tileset>();
        assets.add_loader(TmxLoader);
        assets.add_loader(TsxLoader);
    }
}
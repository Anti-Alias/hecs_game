mod map;
mod tile;
mod tileset;
mod layer;
mod parse;

pub use map::*;
pub use tile::*;
pub use tileset::*;
pub use layer::*;

use crate::{AssetManager, Plugin};

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn install(&mut self, builder: &mut crate::AppBuilder) {
        let game = builder.game();
        let mut assets = game.get::<&mut AssetManager>();
        assets.add_storage::<TiledMap>();
        assets.add_storage::<Tileset>();
        assets.add_loader(TmxLoader);
        assets.add_loader(TsxLoader);
    }
}
use hecs_game::map::{GroupLayer, LayerKind, TiledMap, Tileset};
use hecs_game::*;

fn main() {
    let mut builder = App::builder();
    builder
        .plugin(EnginePlugin::default())
        .plugin(FlycamPlugin)
        .event_handler(start);
    builder.run();
}

fn start(_game: &mut Game, _event: &StartEvent, ctx: &mut RunContext) {
    ctx.start_script(Stage::Update, load_map("maps/map.tmx"))
}


/// Instruction that loads a map and spawns it in the world when ready.
struct SpawnMap {
    path: String,
    map_handle: Option<Handle<TiledMap>>,
}
fn load_map(path: impl Into<String>) -> SpawnMap {
    SpawnMap {
        path: path.into(),
        map_handle: None,
    }
}

impl Task for SpawnMap {

    fn start(&mut self, game: &mut Game, ctx: &mut ScriptContext) {
        let ctx = ctx.run_context;
        let manager = game.get::<&AssetManager>();
        self.map_handle = Some(manager.load(&self.path));
        println!("Started loading {} on tick {}", self.path, ctx.tick());
    }

    fn run(&mut self, game: &mut Game, ctx: &mut ScriptContext) -> bool {
        
        // Waits for map handle to be ready.
        let manager = game.get::<&AssetManager>();
        let map_handle = self.map_handle.as_ref().unwrap();
        match manager.readiness_of(map_handle) {
            Readiness::Ready => {},
            Readiness::NotReady => return false,
            Readiness::Failed => panic!("Map filed to load"),
        }
        
        // Spawns map contents
        let map_storage = manager.storage::<TiledMap>();
        let tileset_storage = manager.storage::<Tileset>();
        let texture_storage = manager.storage::<Texture>();

        let map = map_storage.get(map_handle).unwrap();
        let tilesets = map.tilesets.map
        spawn_map(map, &tileset_storage, &texture_storage);

        true
    }
}

fn spawn_map(
    map: &TiledMap,
    tilesets: &[Tileset],
    textures: &AssetStorage<Texture>,
) {
    for layer in &map.layers {
        match &layer.kind {
            LayerKind::TileLayer(_) => panic!("Tile layers not allowed at root"),
            LayerKind::GroupLayer(layer) => handle_group_layer(layer, tilesets, map),
        }
    }
}

fn handle_group_layer(group_layer: &GroupLayer, tilesets: &[Tileset], map: &TiledMap) {
    for layer in group_layer.iter() {
        match &layer.kind {
            LayerKind::TileLayer(tile_layer) => {
                let (min_x, min_y, max_x, max_y) = tile_layer.bounds(map);
                for x in min_x..max_x {
                    for y in min_y..max_y {
                        let tile_gid = tile_layer.get_tile_gid(x, y, map);
                    }
                }
            },
            LayerKind::GroupLayer(_) => panic!("Sub group layers not allowed"),
        }
    }
}
use hecs_game::map::{TiledMap, Tileset};
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


// Convenience function.
fn load_map(path: impl Into<String>) -> LoadMap {
    LoadMap {
        path: path.into(),
        map_handle: None,
    }
}

struct LoadMap {
    path: String,
    map_handle: Option<Handle<TiledMap>>,
}

impl Task for LoadMap {

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
        
        // Gets map contents
        let map_storage = manager.storage::<TiledMap>();
        let tileset_storage = manager.storage::<Tileset>();

        let map = map_storage.get(map_handle).unwrap();
        println!("{map:#?}");
        for tileset_entry in &map.tilesets {
            let tileset_entry = tileset_storage.get(&tileset_entry.tileset).unwrap();
            println!("{tileset_entry:#?}");
        }
        println!("Finished loading {} on tick {}", self.path, ctx.tick());

        true
    }
}
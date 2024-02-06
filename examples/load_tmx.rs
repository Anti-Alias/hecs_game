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
        handle: None,
    }
}

struct LoadMap {
    path: String,
    handle: Option<Handle<TiledMap>>,
}

impl Task for LoadMap {

    fn start(&mut self, game: &mut Game, ctx: &mut ScriptContext) {
        let ctx = ctx.run_context;
        let manager = game.get::<&AssetManager>();
        self.handle = Some(manager.load(&self.path));
        println!("Started loading {} on tick {}", self.path, ctx.tick());
    }
    fn run(&mut self, game: &mut Game, ctx: &mut ScriptContext) -> bool {
        let ctx = ctx.run_context;
        let manager = game.get::<&AssetManager>();
        let storage = manager.storage::<TiledMap>();
        let handle = self.handle.as_ref().unwrap();
        let map = match storage.get(handle) {
            AssetState::Loading => return false,
            AssetState::Loaded(map) => map,
            AssetState::Failed => panic!("Failed to load map on tick {}", ctx.tick()),
        };
        println!("Finished loading {} on tick {}", self.path, ctx.tick());
        println!("{map:#?}");
        true
    }
}
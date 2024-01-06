use std::time::Duration;
use hecs_game::{Game, App, AssetManager, WinitApp};
use hecs::World;

const TICK_DURATION: Duration = Duration::from_secs(1);

fn main() {
    env_logger::init();
    let game = Game::builder()
        .domain(World::new())
        .domain(AssetManager::builder().build())
        .build();
    let app = App::builder(game)
        .tick_duration(TICK_DURATION)
        .build();
    WinitApp::new(app).run();
}

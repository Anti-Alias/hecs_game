use std::time::Duration;

use hecs_game::{Game, Stage, Queue, GameRunner, ExternalRequest};
use hecs::World;

fn main() {

    // Creates game and runner
    let game = Game::builder()
        .domain(World::new())
        .build();
    let mut runner = GameRunner::builder(game)
        .runner(Stage::Update, start, true)
        .build();

    // Runs game loop
    let mut tick = 1;
    loop {
        println!("----- TICK {tick} -----");
        for request in runner.run_tick() {
            match request {
                ExternalRequest::Quit => break,
            }
        }
        std::thread::sleep(Duration::from_millis(1000));
        tick += 1;
    }
}


fn start(_game: &mut Game, commands: &mut Queue) {
    println!("START");
    commands
        .disable_runner(start);
}
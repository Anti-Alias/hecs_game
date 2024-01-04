use std::time::Duration;
use hecs_game::{Game, Stage, GameRunner, ExternalRequest, RunContext, Instruction, ScriptContext, instruction};
use hecs::World;

const TICK_DURATION: Duration = Duration::from_secs(1);

fn main() {

    // Creates game and wraps it in a runner
    env_logger::init();
    let game = Game::builder()
        .domain(World::new())
        .build();
    let mut runner = GameRunner::builder(game)
        .tick_duration(TICK_DURATION)
        .system(Stage::PreUpdate, start, true)
        .build();

    // Executes the runner in a loop
    loop {
        for request in runner.run_frame(TICK_DURATION) {
            match request {
                ExternalRequest::Quit => break,
            }
        }
        std::thread::sleep(TICK_DURATION);
    }
}



fn start(_game: &mut Game, mut ctx: RunContext) {
    println!("START");
    ctx.start_script(Stage::Update, MyScript);
    ctx.disable_system(start);
}


struct MyScript;
impl Instruction for MyScript {
    fn start(&mut self, _game: &mut Game, ctx: &mut ScriptContext) {
        use instruction::{prnt, wait_secs, add};
        prnt("Hello, world", ctx);
        wait_secs(3, ctx);
        prnt("How are you today?", ctx);
        wait_secs(3, ctx);
        add(MyScript, ctx);
    }
}
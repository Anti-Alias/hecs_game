use std::time::Duration;
use hecs_game::{Game, Stage, App, ExternalRequest, RunContext, Instruction, ScriptContext, Instructor};
use hecs::World;

const TICK_DURATION: Duration = Duration::from_secs(1);

fn main() {

    // Creates game and wraps it in a runner
    env_logger::init();
    let game = Game::builder()
        .domain(World::new())
        .build();
    let mut app = App::builder(game)
        .tick_duration(TICK_DURATION)
        .system(Stage::PreUpdate, start, true)
        .build();

    // Executes the runner in a loop
    loop {
        for request in app.run_frame(TICK_DURATION) {
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
        let mut instructor = Instructor(ctx);
        instructor
            .init_var("times_run", 0)
            .print("Hello, world")
            .wait_secs(1)
            .print("How are you today?")
            .wait_secs(1)
            .inline(|_game, ctx| {
                let times_run: &mut i32 = ctx.var_mut("times_run");
                *times_run += 1;
                if *times_run < 3 {
                    ctx.add(MyScript);
                }
            });
    }
}
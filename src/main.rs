use std::time::Duration;
use hecs_game::{App, WinitRunner, AppConfig, graphics_plugin, core_plugin};

const TICK_DURATION: Duration = Duration::from_secs(1);

fn main() {
    env_logger::init();
    App::config()
        .with_plugin(plugin)
        .run(WinitRunner::new());
}

fn plugin(app: &mut AppConfig) {
    app.set_tick_duration(TICK_DURATION);
    app.add_plugin(core_plugin);
    app.add_plugin(graphics_plugin);
}
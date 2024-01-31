use std::collections::VecDeque;
use std::time::Duration;
use log::warn;
use tracing::instrument;
use vecmap::VecSet;
use crate::{DynEvent, Event, EventBus, EventHandler, Game, HashMap, Script, StartEvent};
    
/**
 * Adds logic to a [`Game`] by executing [`System`]s across it.
 * This happens when invoking run_tick() and run_frame().
 */
pub struct App {
    pub game: Game,                                     // Game to update state via systems.
    pub(crate) quit_requested: bool,                    // If true, app has requested that it quit.
    tick: u64,                                          // Current tick.
    tick_accum: Duration,                               // Time accumulated for current tick.
    tick_duration: Duration,                            // Length of time for a single game tick.
    systems: HashMap<System, SystemMeta>,               // Systems that manipulate the state of the Game.
    enabled_systems: HashMap<Stage, VecSet<System>>,    // Subset of systems that are enabled.
    scripts: HashMap<Stage, Vec<Script>>,               // Scripts.
    event_queue: VecDeque<DynEvent>,                    // Enqueued events
    event_bus: EventBus,                                // Place to fire events, and attach event handlers.
    commands: VecDeque<Box<dyn Command>>,
    app_requests: VecDeque<AppRequest>,
}

impl App {

    pub fn builder() -> AppBuilder {
        AppBuilder {
            app: Self {
                game: Game::new(),
                quit_requested: false,
                tick: 1,
                tick_accum: Duration::ZERO,
                tick_duration: Duration::from_secs_f64(1.0/60.0),
                systems: HashMap::default(),
                enabled_systems: HashMap::default(),
                scripts: HashMap::default(),
                event_queue: VecDeque::default(),
                event_bus: EventBus::default(),
                commands: VecDeque::new(),
                app_requests: VecDeque::new(),
            },
            runner: None,
        }
    }

    pub fn tick_duration(&self) -> Duration { self.tick_duration }

    /**
     * Advances the game logic by a frame.
     * Runs all per-frame stages.
     * Runs all per-tick stages if enough time has accumulated.
     */
    #[instrument(skip(self))]
    pub fn run_frame(&mut self, delta: Duration) {
        
        // Determines how many times to run per-tick stages
        self.tick_accum += delta;
        let mut num_ticks = 0;
        while self.tick_accum >= self.tick_duration {
            self.tick_accum -= self.tick_duration;
            num_ticks += 1;
        }
        let partial_ticks = self.tick_accum.as_secs_f32() / self.tick_duration.as_secs_f32();

        // Fires StartEvent if this is the first tick
        let is_tick = num_ticks > 0;
        if is_tick && self.tick == 1 {
            self.event_queue.push_back(DynEvent::new(StartEvent));
        }

        // Runs per-tick stages
        for _ in 0..num_ticks {
            self.run_stage(Stage::PreUpdate, self.tick_duration, true, partial_ticks);
            self.run_stage(Stage::Update, self.tick_duration, true, partial_ticks);
            self.run_stage(Stage::UpdatePhysics, self.tick_duration, true, partial_ticks);
            self.run_stage(Stage::PostUpdate, self.tick_duration, true, partial_ticks);
            self.run_stage(Stage::Cleanup, self.tick_duration, true, partial_ticks);
            self.tick += 1; 
        }

        // Runs per-frame stages
        self.run_stage(Stage::Asset, delta, is_tick, partial_ticks);
        self.run_stage(Stage::Render, delta, is_tick, partial_ticks);
    }

    /**
     * Runs all [`System`]s within a [`Stage`], then executes enqueued tasks.
     */
    #[instrument(skip(self))]
    fn run_stage(&mut self, stage: Stage, delta: Duration, is_tick: bool, partial_ticks: f32) {

        // Runs systems for stage specified.
        if let Some(systems) = self.enabled_systems.get_mut(&stage) {
            for system in systems.iter().copied() {
                let ctx = RunContext {
                    commands: &mut self.commands,
                    app_requests: &mut self.app_requests,
                    event_queue: &mut self.event_queue,
                    delta,
                    is_tick,
                    partial_ticks,
                };
                system(&mut self.game, ctx);
            }
        }

        // Runs scripts for stage specified.
        if let Some(scripts) = self.scripts.get_mut(&stage) {
            scripts.retain_mut(|script | {
                let ctx = RunContext {
                    commands: &mut self.commands,
                    app_requests: &mut self.app_requests,
                    event_queue: &mut self.event_queue,
                    delta,
                    is_tick,
                    partial_ticks,
                };
                let finished = script.run(&mut self.game, ctx);
                !finished
            });
        }

        // Handles app requests emitted by systems and scripts.
        while let Some(app_request) = self.app_requests.pop_front() {
            match app_request {
                AppRequest::EnableSystem(system)            => self.enable_system(system),
                AppRequest::DisableSystem(system)           => self.disable_system(system),
                AppRequest::StartScript { stage, script }   => self.start_script(stage, script),
                AppRequest::Quit                            => self.quit_requested = true,
            }
        }

        // Runs commands emitted by systems and scripts.
        while let Some(mut command) = self.commands.pop_front() {
            command.run(&mut self.game);
        }

        // Runs event bus for all queued events
        while !self.event_queue.is_empty() {
            let mut event_queue = std::mem::take(&mut self.event_queue);
            let mut ctx = RunContext {
                commands: &mut self.commands,
                app_requests: &mut self.app_requests,
                event_queue: &mut self.event_queue,
                delta,
                is_tick,
                partial_ticks,
            };
            while let Some(event) = event_queue.pop_front() {
                self.event_bus.handle_event(&mut self.game, event, &mut ctx);
            }
        }
    }

    fn enable_system(&mut self, system: System) {
        let Some(system_meta) = self.systems.get_mut(&system) else {
            warn!("System {system:?} not registered");
            return;
        };
        system_meta.enabled_counter += 1;
        if system_meta.enabled_counter == 1 {
            self.enabled_systems
                .entry(system_meta.stage)
                .or_default()
                .insert(system);
        }
    }

    fn disable_system(&mut self, system: System) {
        let Some(system_meta) = self.systems.get_mut(&system) else {
            warn!("System {system:?} not registered");
            return;
        };
        system_meta.enabled_counter -= 1;
        if system_meta.enabled_counter == 0 {
            self.enabled_systems
                .entry(system_meta.stage)
                .or_default()
                .remove(&system);
        }
    }

    fn start_script(&mut self, stage: Stage, script: Script) {
        self.scripts
            .entry(stage)
            .or_default()
            .push(script);
    }
}


pub struct AppBuilder {
    app: App,
    runner: Option<Box<dyn AppRunner>>,
}

impl AppBuilder {

    /**
     * Reference to underlying [`Game`].
     */
    pub fn game(&mut self) -> &mut Game { &mut self.app.game }

    /// Adds a system to the stage specified.
    pub fn system(&mut self, stage: Stage, system: System) -> &mut Self {
        self.system_enabled(stage, system, true);
        self
    }

    /// Adds a system to the stage specified.
    pub fn system_enabled(&mut self, stage: Stage, system: System, enabled: bool) -> &mut Self {
        if self.app.systems.contains_key(&system) {
            panic!("Duplicate system {system:?}");
        }
        let enabled_counter = if enabled { 1 } else { 0 };
        self.app.systems.insert(system, SystemMeta { enabled_counter, stage });
        if enabled {
            self.app.enabled_systems
                .entry(stage)
                .or_default()
                .insert(system);
        }
        self
    }

    pub fn event_handler<E: Event>(&mut self, handler: EventHandler<E>) -> &mut Self {
        self.app.event_bus.add_handler(handler);
        self
    }

    pub fn plugin(&mut self, mut plugin: impl Plugin) -> &mut Self {
        plugin.install(self);
        self
    }

    pub fn tick_duration(&mut self, tick_duration: Duration) -> &mut Self {
        self.app.tick_duration = tick_duration;
        self
    }

    pub fn tick_rate(&mut self, tick_rate: f64) -> &mut Self {
        self.app.tick_duration = Duration::from_secs_f64(1.0 / tick_rate);
        self
    }

    pub fn runner(&mut self, runner: impl AppRunner + 'static) {
        self.runner = Some(Box::new(runner));
    }

    /// Finishes building [`App`] and immediately runs it.
    pub fn run(mut self) {
        
        #[cfg(feature = "profile")]
        {
            use tracing_chrome::ChromeLayerBuilder;
            use tracing_subscriber::prelude::*;
            let (chrome_layer, _guard) = ChromeLayerBuilder::new().include_args(true).build();
            tracing_subscriber::registry().with(chrome_layer).init();
            let mut runner = self.runner.take().expect("Runner not configured");
            runner.run(self.app);
        }
        #[cfg(not(feature = "profile"))]
        {
            env_logger::init();
            let mut runner = self.runner.take().expect("Runner not configured");
            runner.run(self.app);
        }
    }
}

/// Responsible for running an [`App`].
pub trait AppRunner {
    fn run(&mut self, app: App);
}

/**
 * Some function or object that adds functionality to an [`App`].
 */
pub trait Plugin {
    fn install(&mut self, builder: &mut AppBuilder);
}

impl<F> Plugin for F
where F: FnMut(&mut AppBuilder)
{
    fn install(&mut self, builder: &mut AppBuilder) {
        self(builder);
    }
}

pub struct RunContext<'a> {
    commands: &'a mut VecDeque<Box<dyn Command>>,
    app_requests: &'a mut VecDeque<AppRequest>,
    event_queue: &'a mut VecDeque<DynEvent>,
    delta: Duration,
    is_tick: bool,
    partial_ticks: f32,
}

impl<'a> RunContext<'a> {

    /**
     * Time since the last frame or tick, depending on the [`Stage`].
     */
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /**
     * Time since the last frame or tick, depending on the [`Stage`].
     */
    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    pub fn is_tick(&self) -> bool {
        self.is_tick
    }

    pub fn partial_ticks(&self) -> f32 {
        self.partial_ticks
    }

    /**
     * Requests that the following [`Command`] be executed at the end of the current [`Stage`](crate::Stage).
     */
    pub fn run_command(&mut self, command: impl Command) {
        self.commands.push_back(Box::new(command));
    }

    pub fn quit(&mut self) {
        self.app_requests.push_back(AppRequest::Quit);
    }

    /**
     * Requests that a [`Script`] be run over during the [`Stage`] specified.
     */
    pub fn start_script(&mut self, stage: Stage, script: impl Into<Script>) {
        let script = script.into();
        self.app_requests.push_back(AppRequest::StartScript { stage, script });
    }

    /**
     * Requests that a [`System`] be enabled.
     */
    pub fn enable_system(&mut self, system: System) {
        self.app_requests.push_back(AppRequest::EnableSystem(system));
    }

    /**
     * Requests that a [`System`] be disabled.
     */
    pub fn disable_system(&mut self, system: System) {
        self.app_requests.push_back(AppRequest::DisableSystem(system));
    }

    /**
     * Queues an event to be fired at the desired stage.
     */
    pub fn fire<E: Event>(&mut self, event: E) {
        self.event_queue.push_back(DynEvent::new(event));
    }
}

/// Function that runs over a [`Game`] and updates its state.
pub type System = fn(&mut Game, ctx: RunContext);

/// Metadata for a [`System`].
pub(crate) struct SystemMeta {
    pub enabled_counter: i32,
    pub stage: Stage,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Stage {
    /// Per tick.
    /// Decision-making stage.
    /// Maps inputs to "decisions".
    /// Runs AI which emit "decisions".
    PreUpdate,
    /// Per tick.
    /// Execution of decisions in PreUpdate.
    /// Main logic.
    Update,
    /// Per tick.
    /// Runs physics engine.
    UpdatePhysics,
    /// Per tick.
    /// Runs reaction-code based on the outcomes of Upate and UpdatePhysics.
    /// IE: Hitbox / hurtbox.
    PostUpdate,
    /// Per tick.
    /// Cleanup code for things that happened this tick.
    Cleanup,
    /// Per frame.
    /// Runs logic pertaining to asset management.
    Asset,
    /// Per frame.
    /// Updates animations and renders.
    Render,
}


/**
 * A command to run once at the end of a game tick.
 */
pub trait Command: Send + Sync + 'static {
    fn run(&mut self, game: &mut Game);
}

impl<F> Command for F
where
    F: FnMut(&mut Game) + Send + Sync + 'static
{
    fn run(&mut self, game: &mut Game) {
        self(game);
    }
}

/**
 * Command to leverage external functionality.
 */
pub(crate) enum AppRequest {
    EnableSystem(System),
    DisableSystem(System),
    StartScript {
        stage: Stage,
        script: Script,
    },
    Quit,
}
use std::collections::{HashMap, VecDeque, vec_deque};
use std::time::Duration;
use log::warn;
use vecmap::VecSet;
use crate::{Game, Script, ScriptWithId, ScriptId};

/**
 * Adds logic to a [`Game`] by executing [`System`]s across it.
 * This happens when invoking run_tick() and run_frame().
 */
pub struct App {
    pub game: Game,                                     // Game to update state via systems.
    tick: u64,                                          // Current tick.
    tick_accum: Duration,                               // Time accumulated for current tick.
    tick_duration: Duration,                            // Length of time for a single game tick.
    scripts: HashMap<Stage, Vec<ScriptWithId>>,         // Scripts.
    script_seq: u64,
    systems: HashMap<System, SystemMeta>,               // Systems that manipulate the state of the Game.
    enabled_systems: HashMap<Stage, VecSet<System>>,    // Subset of systems that are enabled.
    commands: VecDeque<Box<dyn Command>>,
    app_requests: VecDeque<AppRequest>,
    external_requests: VecDeque<ExternalRequest>,
}

impl App {

    pub fn builder() -> AppBuilder {
        AppBuilder {
            app: Self {
                game: Game::new(),
                tick: 1,
                tick_accum: Duration::ZERO,
                tick_duration: Duration::from_secs_f64(1.0/60.0),
                scripts: HashMap::new(),
                script_seq: 0,
                systems: HashMap::new(),
                enabled_systems: HashMap::new(),
                commands: VecDeque::new(),
                app_requests: VecDeque::new(),
                external_requests: VecDeque::new(),
            },
            runner: None,
        }
    }

    pub fn tick_duration(&self) -> Duration { self.tick_duration }

    /**
     * Runs all per-frame [`Stage`]s.
     * If enough time has accumulated, each per-tick [`Stage`]s as well.
     * Good for client applications.
     */
    pub fn run_frame(&mut self, delta: Duration) -> vec_deque::Iter<'_, ExternalRequest> {
        log::trace!("----- TICK {} -----", self.tick);
        self.tick_accum += delta;
        self.external_requests.clear();
        self.run_stage(Stage::Input, delta);
        while self.tick_accum >= self.tick_duration {
            self.run_stage(Stage::PreUpdate, self.tick_duration);
            self.run_stage(Stage::Update, self.tick_duration);
            self.run_stage(Stage::UpdatePhysics, self.tick_duration);
            self.run_stage(Stage::PostUpdate, self.tick_duration);
            self.run_stage(Stage::Cleanup, self.tick_duration);
            self.tick_accum -= self.tick_duration;
        }
        self.run_stage(Stage::Render, delta);
        self.tick += 1;
        return self.external_requests.iter()
    }

    /**
     * Runs all per-frame [`Stage`]s and per-tick [`Stage`]s.
     * Good for server applications.
     */
    pub fn run_tick(&mut self) -> vec_deque::Iter<'_, ExternalRequest> {
        self.run_frame(self.tick_duration)
    }

    /**
     * Runs all [`System`]s within a [`Stage`], then executes enqueued tasks.
     */
    fn run_stage(&mut self, stage: Stage, delta: Duration) {

        // Runs systems for stage specified.
        if let Some(systems) = self.enabled_systems.get_mut(&stage) {
            for system in systems.iter().copied() {
                let ctx = RunContext {
                    script_seq: &mut self.script_seq,
                    commands: &mut self.commands,
                    app_requests: &mut self.app_requests,
                    external_requests: &mut self.external_requests,
                    delta,
                };
                system(&mut self.game, ctx);
            }
        }

        // Runs scripts for stage specified.
        if let Some(scripts) = self.scripts.get_mut(&stage) {
            scripts.retain_mut(|script | {
                let ctx = RunContext {
                    script_seq: &mut self.script_seq,
                    commands: &mut self.commands,
                    app_requests: &mut self.app_requests,
                    external_requests: &mut self.external_requests,
                    delta,
                };
                let finished = script.script.run(&mut self.game, ctx);
                !finished
            });
        }

        // Handles app requests emitted by systems and scripts.
        while let Some(app_request) = self.app_requests.pop_front() {
            match app_request {
                AppRequest::EnableSystem(system)         => self.enable_system(system),
                AppRequest::DisableSystem(system)        => self.disable_system(system),
                AppRequest::StartScript { id, stage, script }  => self.run_script(id, stage, script),
                AppRequest::StopScript(id)                   => self.stop_script(id),
            }
        }

        // Runs commands emitted by systems and scripts.
        while let Some(mut command) = self.commands.pop_front() {
            command.run(&mut self.game);
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

    fn run_script(&mut self, id: ScriptId, stage: Stage, script: Script) {
        self.scripts
            .entry(stage)
            .or_default()
            .push(ScriptWithId { script, id });
    }

    fn stop_script(&mut self, script_id: ScriptId) {
        for scripts in self.scripts.values_mut() {
            let Some(position) = scripts
                .iter()
                .position(|script| script.id == script_id) else { continue };
            scripts.remove(position);
            return;
        }
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
    pub fn add_system(&mut self, stage: Stage, system: System, enabled: bool) -> &mut Self {
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

    pub fn plugin(&mut self, mut plugin: impl Plugin) -> &mut Self {
        plugin.install(self);
        self
    }

    pub fn tick_duration(&mut self, tick_duration: Duration) {
        self.app.tick_duration = tick_duration;
    }

    pub fn runner(&mut self, runner: impl AppRunner + 'static) {
        self.runner = Some(Box::new(runner));
    }

    /// Finishes building [`App`] and immediately runs it.
    pub fn run(mut self) {
        let mut runner = self.runner.take().expect("Runner not configured");
        runner.run(self.app);
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
    script_seq: &'a mut u64,
    commands: &'a mut VecDeque<Box<dyn Command>>,
    app_requests: &'a mut VecDeque<AppRequest>,
    external_requests: &'a mut VecDeque<ExternalRequest>,
    delta: Duration,
}

impl<'a> RunContext<'a> {

    /**
     * Time since the last frame or tick, depending on the [`Stage`].
     */
    pub fn delta(&self) -> Duration { self.delta }

    /**
     * Requests that the following [`Command`] be executed at the end of the current [`Stage`](crate::Stage).
     */
    pub fn run_command(&mut self, command: impl Command) {
        self.commands.push_back(Box::new(command));
    }

    /**
     * Requests that a [`Script`] be run over during the [`Stage`] specified.
     */
    pub fn start_script(&mut self, stage: Stage, script: impl Into<Script>) -> ScriptId {
        let id = ScriptId(*self.script_seq);
        *self.script_seq += 1;
        let script = script.into();
        self.app_requests.push_back(AppRequest::StartScript { id, stage, script });
        id
    }

    /**
     * Requests that a [`Script`] be stopped.
     */
    pub fn stop_script(&mut self, id: ScriptId) {
        self.app_requests.push_back(AppRequest::StopScript(id));
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
     * Requests that the [`Game`] quit.
     */
    pub fn quit(&mut self) {
        self.external_requests.push_back(ExternalRequest::Quit);
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
    /// Reads input devices (mouse, controllers etc).
    /// Stores those inputs into domain(s) for future reading.
    Input,
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
    /// Per frame.
    /// Updates animations and renders.
    Render,
    /// Per frame.
    /// Any code that needs to clear data structures every tick.
    Cleanup,
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
        id: ScriptId,
        stage: Stage,
        script: Script,
    },
    StopScript(ScriptId),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ExternalRequest {
    Quit,
}
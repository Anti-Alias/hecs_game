use std::collections::{HashMap, VecDeque};
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
    runner_requests: VecDeque<RunnerRequest>,
    external_requests: VecDeque<ExternalRequest>,
}

impl App {

    pub fn builder(game: Game) -> AppBuilder {
        AppBuilder(Self {
            game,
            tick: 1,
            tick_accum: Duration::ZERO,
            tick_duration: Duration::from_secs_f64(1.0/60.0),
            scripts: HashMap::new(),
            script_seq: 0,
            systems: HashMap::new(),
            enabled_systems: HashMap::new(),
            commands: VecDeque::new(),
            runner_requests: VecDeque::new(),
            external_requests: VecDeque::new(),
        })
    }

    /**
     * Runs all per-frame [`Stage`]s.
     * If enough time has accumulated, each per-tick [`Stage`]s as well.
     * Good for client applications.
     */
    pub fn run_frame(&mut self, delta: Duration) -> impl Iterator<Item = ExternalRequest> + '_ {
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
        return self.external_requests.iter().copied()
    }

    /**
     * Runs all per-frame [`Stage`]s and per-tick [`Stage`]s.
     * Good for server applications.
     */
    pub fn run_tick(&mut self) -> impl Iterator<Item = ExternalRequest> + '_ {
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
                    runner_requests: &mut self.runner_requests,
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
                    runner_requests: &mut self.runner_requests,
                    external_requests: &mut self.external_requests,
                    delta,
                };
                let finished = script.script.run(&mut self.game, ctx);
                !finished
            });
        }

        // Handles runner requests emitted by systems and scripts.
        while let Some(runner_req) = self.runner_requests.pop_front() {
            match runner_req {
                RunnerRequest::EnableSystem(system)         => self.enable_system(system),
                RunnerRequest::DisableSystem(runner)        => self.disable_system(runner),
                RunnerRequest::StartScript { id, stage, script }  => self.run_script(id, stage, script),
                RunnerRequest::StopScript(id)                   => self.stop_script(id),
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

pub struct AppBuilder(App);
impl AppBuilder {

    /// Adds a runner to the stage specified.
    pub fn system(mut self, stage: Stage, system: System, enabled: bool) -> Self {
        if self.0.systems.contains_key(&system) {
            panic!("Duplicate system {system:?}");
        }
        let enabled_counter = if enabled { 1 } else { 0 };
        self.0.systems.insert(system, SystemMeta { enabled_counter, stage });
        if enabled {
            self.0.enabled_systems
                .entry(stage)
                .or_default()
                .insert(system);
        }
        self
    }

    /**
     * Sets duration of a tick. Defaults to 1/60 seconds.
     */
    pub fn tick_duration(mut self, tick_duration: Duration) -> Self {
        self.0.tick_duration = tick_duration;
        self
    }

    pub fn build(self) -> App {
        self.0
    }
}

pub struct RunContext<'a> {
    script_seq: &'a mut u64,
    commands: &'a mut VecDeque<Box<dyn Command>>,
    runner_requests: &'a mut VecDeque<RunnerRequest>,
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
        self.runner_requests.push_back(RunnerRequest::StartScript { id, stage, script });
        id
    }

    /**
     * Requests that a [`Script`] be stopped.
     */
    pub fn stop_script(&mut self, id: ScriptId) {
        self.runner_requests.push_back(RunnerRequest::StopScript(id));
    }

    /**
     * Requests that a [`System`] be enabled.
     */
    pub fn enable_system(&mut self, runner: System) {
        self.runner_requests.push_back(RunnerRequest::EnableSystem(runner));
    }

    /**
     * Requests that a [`System`] be disabled.
     */
    pub fn disable_system(&mut self, runner: System) {
        self.runner_requests.push_back(RunnerRequest::DisableSystem(runner));
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
pub(crate) enum RunnerRequest {
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
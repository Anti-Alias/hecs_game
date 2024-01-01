use std::any::{TypeId, Any};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;
use log::warn;
use vecmap::VecSet;

use crate::{Queue, ExternalRequest};

/// Game structure, which acts as a simple container of [`Domain`]s.
pub struct Game {
    domains: HashMap<TypeId, Box<dyn Any>>,
}

impl Game {
    pub fn builder() -> GameBuilder {
        GameBuilder(Self {
            domains: HashMap::new(),
        })
    }
    pub fn domain<D: Domain>(&self) -> &RefCell<D> {
        self.domain_checked().unwrap()
    }
    pub fn domain_checked<D: Domain>(&self) -> Option<&RefCell<D>> {
        let domain = self.domains.get(&TypeId::of::<D>())?;
        domain.downcast_ref::<RefCell<D>>()
    }
}

pub struct GameBuilder(Game);
impl GameBuilder {
    pub fn domain<D: Domain>(mut self, domain: D) -> Self {
        self.0.domains.insert(TypeId::of::<D>(), Box::new(domain));
        self
    }
    pub fn build(self) -> Game {
        self.0
    }
}

/**
 * Object that aids in running a [`Game`].
 */
pub struct GameRunner {
    pub game: Game,                                     // Game to update state via runners.
    tick_accum: Duration,                               // Time accumulated for current tick.
    tick_duration: Duration,                            // Length of time for a single game tick.
    runners: HashMap<Runner, RunnerMeta>,               // Runners that manipulate the state of the Game.
    enabled_runners: HashMap<Stage, VecSet<Runner>>,    // Subset of runners that are enabled.
    queue: Queue,                                       // Tasks to execute when Game is not being manipualted by a runner.
}

impl GameRunner {

    pub fn builder(game: Game) -> GameRunnerBuilder {
        GameRunnerBuilder(Self {
            game,
            tick_accum: Duration::ZERO,
            tick_duration: Duration::from_secs_f64(1.0/60.0),
            runners: HashMap::new(),
            enabled_runners: HashMap::new(),
            queue: Queue::new(),
        })
    }

    /**
     * Runs all per-frame [`Stage`]s.
     * If enough time has accumulated, each per-tick [`Stage`]s as well.
     * Good for client applications.
     */
    pub fn run_frame(&mut self, delta: Duration) -> impl Iterator<Item = ExternalRequest> + '_ {
        self.tick_accum += delta;
        self.queue.external_requests.clear();
        self.run_stage(Stage::Input);
        while self.tick_accum >= self.tick_duration {
            self.run_stage(Stage::PreUpdate);
            self.run_stage(Stage::Update);
            self.run_stage(Stage::UpdatePhysics);
            self.run_stage(Stage::PostUpdate);
            self.run_stage(Stage::Cleanup);
            self.tick_accum -= self.tick_duration;
        }
        self.run_stage(Stage::Render);
        return self.queue.external_requests.iter().copied()
    }

    /**
     * Runs all per-frame [`Stage`]s and per-tick [`Stage`]s.
     * Good for server applications.
     */
    pub fn run_tick(&mut self) -> impl Iterator<Item = ExternalRequest> + '_ {
        self.run_frame(self.tick_duration)
    }

    /**
     * Runs all [`Runner`]s within a [`Stage`], then executes enqueued tasks.
     */
    fn run_stage(&mut self, stage: Stage) {
        let Some(runners) = self.enabled_runners.get_mut(&stage) else { return };
        for runner in runners.iter().copied() {
            runner(&mut self.game, &mut self.queue);
        }
        while let Some(mut command) = self.queue.commands.pop_front() {
            command.run(&mut self.game);
        }
        while let Some(runner_req) = self.queue.requests.pop_front() {
            match runner_req {
                crate::RunnerRequest::EnableRunner(runner) => self.enable_runner(runner),
                crate::RunnerRequest::DisableRunner(runner) => self.disable_runner(runner),
            }
        }
    }

    fn enable_runner(&mut self, runner: Runner) {
        let Some(runner_meta) = self.runners.get_mut(&runner) else {
            warn!("Runner {runner:?} not registered");
            return;
        };
        runner_meta.enabled_counter += 1;
        if runner_meta.enabled_counter == 1 {
            self.enabled_runners
                .entry(runner_meta.stage)
                .or_default()
                .insert(runner);
        }
    }

    fn disable_runner(&mut self, runner: Runner) {
        let Some(runner_meta) = self.runners.get_mut(&runner) else {
            warn!("Runner {runner:?} not registered");
            return;
        };
        runner_meta.enabled_counter -= 1;
        if runner_meta.enabled_counter == 0 {
            self.enabled_runners
                .entry(runner_meta.stage)
                .or_default()
                .remove(&runner);
        }
    }
}

pub struct GameRunnerBuilder(GameRunner);
impl GameRunnerBuilder {

    /// Adds a runner to the stage specified.
    pub fn runner(mut self, stage: Stage, runner: Runner, enabled: bool) -> Self {
        if self.0.runners.contains_key(&runner) {
            panic!("Duplicate runner {runner:?}");
        }
        let enabled_counter = if enabled { 1 } else { 0 };
        self.0.runners.insert(runner, RunnerMeta { enabled_counter, stage });
        if enabled {
            self.0.enabled_runners
                .entry(stage)
                .or_default()
                .insert(runner);
        }
        self
    }

    pub fn build(self) -> GameRunner {
        self.0
    }
}

/// Function that runs over a [`Game`] and updates its state.
pub type Runner = fn(&mut Game, commands: &mut Queue);

/// Metadata for a [`Runner`].
pub(crate) struct RunnerMeta {
    pub enabled_counter: i32,
    pub stage: Stage,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Stage {
    Input,
    PreUpdate,
    Update,
    UpdatePhysics,
    PostUpdate,
    Render,
    Cleanup,
}

/**
 * A place where logic of a certain variety is performed.
 * IE: Physics, Graphics, logic etc.
 */
pub trait Domain: Any + Send + Sync {}
impl<D: Any + Send + Sync> Domain for D {}
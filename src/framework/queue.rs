use std::collections::VecDeque;
use crate::{Game, Runner};

/**
 * Allows encoding tasks that will be executed at the end of the [`Stage`](crate::Stage).
 */
pub struct Queue {
    pub(crate) commands: VecDeque<Box<dyn Command>>,
    pub(crate) requests: VecDeque<RunnerRequest>,
    pub(crate) external_requests: VecDeque<ExternalRequest>,
}
impl Queue {

    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
            requests: VecDeque::new(),
            external_requests: VecDeque::new(),
        }
    }

    /**
     * Pushes a [`Command`] that will manipulate the [`Game`] at the end of the current [`Stage`](crate::Stage).
     */
    pub fn execute(&mut self, command: impl Command) -> &mut Self {
        self.commands.push_back(Box::new(command));
        self
    }

    /**
     * Requests that a [`Runner`] be enabled.
     */
    pub fn enable_runner(&mut self, runner: Runner) -> &mut Self {
        self.requests.push_back(RunnerRequest::EnableRunner(runner));
        self
    }

    /**
     * Requests that a [`Runner`] be disabled.
     */
    pub fn disable_runner(&mut self, runner: Runner) -> &mut Self {
        self.requests.push_back(RunnerRequest::DisableRunner(runner));
        self
    }

    /**
     * Requests that the [`Game`] quit.
     */
    pub fn quit(&mut self) -> &mut Self {
        self.external_requests.push_back(ExternalRequest::Quit);
        self
    }
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
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RunnerRequest {
    EnableRunner(Runner),
    DisableRunner(Runner),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ExternalRequest {
    Quit,
}
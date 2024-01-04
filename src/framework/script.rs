use std::collections::VecDeque;
use crate::{Game, RunContext};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub struct ScriptId(pub(crate) u64);

/**
 * A series of [`Instruction`]s to run one after another.
 */
pub struct Script {
    current: Option<Box<dyn Instruction>>,
    instructions: VecDeque<Box<dyn Instruction>>,
    stopped: bool,
}

impl Script {

    pub(crate) fn new() -> Self {
        Self {
            current: None,
            instructions: VecDeque::new(),
            stopped: false,
        }
    }

    /**
     * Adds an instruction to the end of the script.
     */
    pub fn add(&mut self, instruction: impl Instruction) -> &mut Self {
        self.instructions.push_back(Box::new(instruction));
        self
    }

    /**
     * Advances by a single instruction. Re-runs instruction next tick if not finished.
     * Returns true if all instructions are consumed.
     */
    pub(crate) fn run(&mut self, game: &mut Game, mut run_context: RunContext) -> bool {
        if self.stopped { return false }
        let mut current_ins = match self.current.take() {
            Some(current_ins) => current_ins,
            None => {
                let Some(mut current_ins) = self.instructions.pop_front() else { return true };
                current_ins.start(game, &mut ScriptContext::new(&mut run_context, self));
                current_ins
            },
        };

        loop {
            let finished = current_ins.run(game, &mut ScriptContext::new(&mut run_context, self));
            if finished {
                current_ins = match self.instructions.pop_front() {
                    Some(current) => current,
                    None => return true,
                };
                current_ins.start(game, &mut ScriptContext::new(&mut run_context, self));
            }
            else {
                self.current = Some(current_ins);
                return false;
            }
        }
    }
}

pub(crate) struct ScriptWithId {
    pub script: Script,
    pub id: ScriptId,
}

impl<I: Instruction> From<I> for Script {
    fn from(instruction: I) -> Self {
        let mut script = Self::new();
        script.add(instruction);
        script
    }
}

/**
 * Some task that runs for one or more game ticks.
 */
pub trait Instruction: Send + Sync + 'static {
    /**
     * Executed right before run() is invoked for the first time.
     */
    fn start(&mut self, _game: &mut Game, _ctx: &mut ScriptContext) {}

    /**
     * Runs the instruction for a single tick.
     * Returns true if instruction is finished.
     */
    fn run(&mut self, _game: &mut Game, _ctx: &mut ScriptContext) -> bool { true }
}

/**
 * Parameters passed into the various methods belonging to [`Task`].
 */
pub struct ScriptContext<'a> {
    pub run_context: &'a RunContext<'a>,
    script: &'a mut Script,
    insert_index: usize,
}

impl<'a> ScriptContext<'a> {

    fn new(run_context: &'a RunContext<'a>, script: &'a mut Script) -> Self {
        Self {
            run_context,
            script,
            insert_index: 0,
        }
    }

    pub fn add(&mut self, instruction: impl Instruction) -> &mut Self {
        self.script.instructions.insert(self.insert_index, Box::new(instruction));
        self.insert_index += 1;
        self
    }
}
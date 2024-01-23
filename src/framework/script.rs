use std::any::Any;
use std::collections::VecDeque;
use derive_more::*;
use crate::{Game, RunContext, HashMap};

/**
 * A series of [`Instruction`]s to run one after another.
 */
pub struct Script {
    current: Option<Box<dyn Instruction>>,
    instructions: VecDeque<Box<dyn Instruction>>,
    variables: HashMap<VarKey, Box<dyn Any>>,
    stopped: bool,
}

impl Script {

    pub(crate) fn new() -> Self {
        Self {
            current: None,
            instructions: VecDeque::new(),
            variables: HashMap::default(),
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

impl<I: Instruction> From<I> for Script {
    fn from(instruction: I) -> Self {
        let mut script = Self::new();
        script.add(instruction);
        script
    }
}

/**
 * Value of a variable stored in a [`Script`].
 */
pub trait VarValue: Any + Send + Sync {}
impl<T: Any + Send + Sync> VarValue for T {}

/**
 * Hash of a variable name.
 */
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub struct VarKey(u64);
impl From<&str> for VarKey {
    fn from(value: &str) -> Self {
        Self(fxhash::hash64(value))
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

    /**
     * Sets a variable.
     */
    pub fn set_var<V: VarValue>(&mut self, var_key: impl Into<VarKey>, var_value: V) {
        self.set_var_boxed(var_key.into(), Box::new(var_value));
    }

    /**
     * Sets a variable.
     */
    pub fn set_var_boxed(&mut self, var_key: VarKey, var_value: Box<dyn Any + Send + Sync>) {
        self.script.variables.insert(var_key, var_value);
    }

    /**
     * Unsets a variable.
     */
    pub fn unset_var(&mut self, var_key: impl Into<VarKey>) {
        self.script.variables.remove(&var_key.into());
    }

    /**
     * Sets a variable.
     */
    pub fn contains_var(&mut self, var_key: impl Into<VarKey>) -> bool {
        self.script.variables.contains_key(&var_key.into())
    }

    /**
     * Retrieves a variable.
     * Panics if not found, or was of a different type.
     */
    pub fn var<V: VarValue>(&self, var_name: impl Into<VarKey>) -> &V {
        self.try_var(var_name).unwrap()
    }

    /**
     * Retrieves a variable.
     * Panics if not found, or was of a different type.
     */
    pub fn var_mut<V: VarValue>(&mut self, var_name: impl Into<VarKey>) -> &mut V {
        self.try_var_mut(var_name).unwrap()
    }

    /**
     * Retrieves a variable.
     */
    pub fn try_var<V: VarValue>(&self, var_name: impl Into<VarKey>) -> Result<&V, ScriptError> {
        let box_any = self.script.variables
            .get(&var_name.into())
            .ok_or(ScriptError::VariableNotFound)?;
        let value = box_any
            .downcast_ref::<V>()
            .ok_or(ScriptError::IncorrectVariableType)?;
        Ok(value)
    }

    /**
     * Retrieves a variable.
     */
    pub fn try_var_mut<V: VarValue>(&mut self, var_name: impl Into<VarKey>) -> Result<&mut V, ScriptError> {
        let box_any = self.script.variables
            .get_mut(&var_name.into())
            .ok_or(ScriptError::VariableNotFound)?;
        let value = box_any
            .downcast_mut::<V>()
            .ok_or(ScriptError::IncorrectVariableType)?;
        Ok(value)
    }
}

#[derive(Error, Display, Debug)]
pub enum ScriptError {
    VariableNotFound,
    IncorrectVariableType,
}
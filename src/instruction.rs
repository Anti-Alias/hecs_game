use std::time::Duration;
use crate::{ScriptContext, Instruction, Game};

/// A simple print instruction.
pub struct Print(pub String);
impl Instruction for Print {
    fn start(&mut self, _game: &mut Game, _ctx: &mut ScriptContext) {
        println!("{}", self.0);
    }
}

/// A simple wait instruction.
pub struct Wait(pub Duration);
impl Instruction for Wait {
    fn run(&mut self, _game: &mut Game, ctx: &mut ScriptContext) -> bool {
        let delta = ctx.run_context.delta();
        if delta > self.0 {
            self.0 = Duration::ZERO;
            true
        }
        else {
            self.0 -= delta;
            false
        }
    }
}

/**
 * Inline instruction that runs a block of code once during its start() invocation.
*/
pub struct Inline<F>(pub F);
impl<F> Instruction for Inline<F>
where
    F: FnMut(&mut Game, &mut ScriptContext) + Send + Sync + 'static
{
    fn start(&mut self, game: &mut Game, ctx: &mut ScriptContext) {
        self.0(game, ctx);
    }
}

// ---------------- Syntactic sugar functions ----------------
pub fn prnt(message: impl Into<String>, ctx: &mut ScriptContext) {
    ctx.add(Print(message.into()));
}

pub fn wait_secs(secs: u64, ctx: &mut ScriptContext) {
    ctx.add(Wait(Duration::from_secs(secs)));
}

pub fn wait_secs_f32(secs: f32, ctx: &mut ScriptContext) {
    ctx.add(Wait(Duration::from_secs_f32(secs)));
}

pub fn wait_millis(millis: u64, ctx: &mut ScriptContext) {
    ctx.add(Wait(Duration::from_millis(millis)));
}

pub fn inline<F>(ctx: &mut ScriptContext, callback: F)
where F: FnMut(&mut Game, &mut ScriptContext) + Send + Sync + 'static {
    ctx.add(Inline(callback));
}

pub fn add(instruction: impl Instruction, ctx: &mut ScriptContext) {
    ctx.add(instruction);
}
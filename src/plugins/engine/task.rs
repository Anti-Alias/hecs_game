use std::time::Duration;
use crate::{ScriptContext, Task, Game, VarValue, VarKey};

/// A simple print task.
pub struct Print(pub String);
impl Task for Print {
    fn start(&mut self, _game: &mut Game, _ctx: &mut ScriptContext) {
        println!("{}", self.0);
    }
}

/// A simple wait task.
pub struct Wait(pub Duration);
impl Task for Wait {
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
 * Inline task that runs a block of code once during its start() invocation.
*/
pub struct Inline<F>(pub F);
impl<F> Task for Inline<F>
where
    F: FnMut(&mut Game, &mut ScriptContext) + Send + Sync + 'static
{
    fn start(&mut self, game: &mut Game, ctx: &mut ScriptContext) {
        self.0(game, ctx);
    }
}


// ---------------- Syntactic sugar functions ----------------
pub struct Instructor<'a, 'b>(pub &'a mut ScriptContext<'b>);
impl<'a, 'b> Instructor<'a, 'b> {

    pub fn print(&mut self, message: impl Into<String>) -> &mut Self {
        self.0.add(Print(message.into()));
        self
    }
    
    /// Waits for a period of time.
    pub fn wait_secs(&mut self, secs: u64) -> &mut Self {
        self.0.add(Wait(Duration::from_secs(secs)));
        self
    }
    
    /// Waits for a period of time.
    pub fn wait_secs_f32(&mut self, secs: f32) -> &mut Self {
        self.0.add(Wait(Duration::from_secs_f32(secs)));
        self
    }
    
    /// Waits for a period of time.
    pub fn wait_millis(&mut self, millis: u64) -> &mut Self {
        self.0.add(Wait(Duration::from_millis(millis)));
        self
    }
    
    /// Performs some inline task that completes immediately.
    pub fn inline<F>(&mut self, callback: F) -> &mut Self
    where F: FnMut(&mut Game, &mut ScriptContext) + Send + Sync + 'static {
        self.0.add(Inline(callback));
        self
    }
    
    /**
     * Sets a variable, overwriting the old one if it exists.
    */
    pub fn set_var<V: VarValue>(&mut self, var_key: impl Into<VarKey>, var_value: V) -> &mut Self {
        let var_key = var_key.into();
        let mut var_value = Some(Box::new(var_value));
        self.inline(move|_game, ctx| {
            let var_value = var_value.take().unwrap();
            ctx.set_var_boxed(var_key, var_value)
        });
        self
    }
    
    /**
     * Sets a variable if it does not exist.
    */
    pub fn init_var(&mut self, var_key: impl Into<VarKey>, var_value: impl VarValue) -> &mut Self {
        let var_key = var_key.into();
        let mut var_value = Some(Box::new(var_value));
        self.inline(move|_game, ctx| {
            if !ctx.contains_var(var_key) {
                let var_value = var_value.take().unwrap();
                ctx.set_var_boxed(var_key, var_value);
            }
        });
        self
    }
    
    pub fn add(&mut self, task: impl Task) -> &mut Self {
        self.0.add(task);
        self
    }
}

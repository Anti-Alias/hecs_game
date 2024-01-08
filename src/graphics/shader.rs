use vecmap::VecSet;


pub struct ShaderDefs(VecSet<String>);
impl ShaderDefs {
    
    pub(crate) fn new() -> Self {
        Self(VecSet::new())
    }
    
    pub fn add(&mut self, shader_def: impl Into<String>) {
        self.0.insert(shader_def.into());
    }

    pub fn is_def(&self, def: impl AsRef<str>) -> bool {
        let def = def.as_ref();
        self.0.contains(def)
    }
}
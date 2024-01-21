use vecmap::VecSet;
use derive_more::*;

/// Stores flags that are used during shader preprocessing.
/// These flags determine if #ifdef blocks get included or stripped out in the final shader.
pub struct ShaderPreprocessor(VecSet<String>);

impl ShaderPreprocessor {
    
    pub(crate) fn new() -> Self {
        Self(VecSet::new())
    }
    
    pub fn add(&mut self, shader_def: impl Into<String>) {
        self.0.insert(shader_def.into());
    }

    pub fn is_defined(&self, def: impl AsRef<str>) -> bool {
        let def = def.as_ref();
        self.0.contains(def)
    }

    /**
     * Preprocesses shader code.
     */
    pub fn preprocess(&mut self, shader_template: &str) -> Result<String, ShaderDefError> {
        let mut result = String::new();
        let mut state = State::new(shader_template);
        self.inner_preprocess(&mut result, &mut state)?;
        Ok(result)
    }

    fn inner_preprocess(&mut self, result: &mut String, state: &mut State) -> Result<(), ShaderDefError> {

        while let Some(line) = state.line {
            let trim_line = line.trim();

            // Handles command
            if trim_line.starts_with('#') {
                let (command, param) = trim_line.split_once(' ').unwrap_or((trim_line, ""));
                let param = param.trim();
                match command {
                    "#ifdef" => {
                        state.next_line();
                        if self.0.contains(param) {
                            state.ifdef_count += 1;
                            self.inner_preprocess(result, state)?;
                        }
                        else {
                            Self::skip_past_endif(state)?;
                        }
                    },
                    "#ifndef" => {
                        state.next_line();
                        if !self.0.contains(param) {
                            state.ifdef_count += 1;
                            self.inner_preprocess(result, state)?;
                        }
                        else {
                            Self::skip_past_endif(state)?;
                        }
                    },
                    "#endif" => {
                        if !param.is_empty() {
                            return Err(ShaderDefError::new(state.line_num, ShaderDefErrorKind::UnexpectedParam))
                        }
                        if state.ifdef_count == 0 {
                            return Err(ShaderDefError::new(state.line_num, ShaderDefErrorKind::UnexpectedEndif))
                        }
                        else {
                            state.next_line();
                            state.ifdef_count -= 1;
                            return Ok(());
                        }
                    },
                    _ => return Err(ShaderDefError::new(state.line_num, ShaderDefErrorKind::InvalidCommand)),
                }
            }

            // Handles normal line
            else {
                result.push_str(line);
                state.next_line();
                if state.line.is_some() {
                    result.push('\n');
                }
            }
        }

        // At EOF, fail if we're in an #ifdef block.
        if state.ifdef_count != 0 {
            return Err(ShaderDefError::new(state.line_num, ShaderDefErrorKind::MissingEndif))
        }
        Ok(())
    }

    fn skip_past_endif(state: &mut State) -> Result<(), ShaderDefError> {
        let mut ifdef_counter = 1;
        while let Some(line) = state.line {
            let line = line.trim_start();
            if line.starts_with("#ifdef") {
                ifdef_counter += 1;
            }
            else if line.starts_with("#endif") {
                ifdef_counter -= 1;
                if ifdef_counter == 0 {
                    state.next_line();
                    return Ok(())
                }
            }
            state.next_line();
        }
        return Err(ShaderDefError::new(state.line_num, ShaderDefErrorKind::MissingEndif))
    }
}

/// Current state of preprocessing.
struct State<'a> {
    line_num: u32,                  // Current line number
    line: Option<&'a str>,          // Contents of current line
    template: Option<&'a str>,      // Remainder of the template to parse
    ifdef_count: u32,               // Counter for ifdef/endif validation
}

impl<'a> State<'a> {

    fn new(template: &'a str) -> Self {
        let mut result = Self {
            line_num: 0,
            line: None,
            template: Some(template),
            ifdef_count: 0,
        };
        result.next_line();
        result
    }

    fn next_line(&mut self) {
        match self.template {
            Some(template) => {
                let (line, template) = match template.split_once("\n") {
                    Some((line, template)) => (Some(line), Some(template)),
                    None => (Some(template), None),
                };
                self.line = line;
                self.template = template;
            },
            None => {
                self.line = None;
                self.template = None;
            }
        }
    }
}


#[derive(Error, Copy, Clone, Eq, PartialEq, Display, Debug)]
#[display(fmt="Preprocessing error on line {line_num}: {kind}")]
pub struct ShaderDefError {
    pub line_num: u32,
    pub kind: ShaderDefErrorKind,
}

impl ShaderDefError {
    pub fn new(line_num: u32, kind: ShaderDefErrorKind) -> Self {
        Self { line_num, kind }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Display, Debug)]
pub enum ShaderDefErrorKind {
    #[display(fmt="Invalid command")]
    InvalidCommand,
    #[display(fmt="Invalid param")]
    UnexpectedParam,
    #[display(fmt="Not inside of an #ifdef block")]
    NotInsideIfdefBlock,
    #[display(fmt="Missing #endif at the end of the file")]
    MissingEndif,
    #[display(fmt="Unexpected #endif")]
    UnexpectedEndif,
}


#[cfg(test)]
mod test {
    use crate::ShaderPreprocessor;

    #[test]
    fn ifdef() {
        let template =
"This is a normal line.
#ifdef HERP
This line will be included.
#endif
#ifdef DERP
This line will be stripped out.
#endif
This is another normal line";
        let mut defs = ShaderPreprocessor::new();
        defs.add("HERP");
        let result = defs.preprocess(template);
        let expected =
"This is a normal line.
This line will be included.
This is another normal line";
        assert_eq!(Ok(expected.to_owned()), result);
    }


    #[test]
    fn ifdef_whilespace() {
        let template =
"   This is a normal line   .  
#ifdef HERP  
   This line will be included.  
 #endif
     #ifdef DERP
 This line will be stripped out.  
    #endif
 This is another normal line";
        let mut defs = ShaderPreprocessor::new();
        defs.add("HERP");
        let result = defs.preprocess(template);
        let expected =
"   This is a normal line   .  
   This line will be included.  
 This is another normal line";
        assert_eq!(Ok(expected.to_owned()), result);
    }

    #[test]
    fn ifndef() {
        let template  =
"This is a normal line
#ifndef HERP
This line will be included
#endif
#ifndef DERP
This line will be sripped out
#endif
This is another normal line";
        let mut defs = ShaderPreprocessor::new();
        defs.add("DERP");
        let result = defs.preprocess(template);
        let expected =
"This is a normal line
This line will be included
This is another normal line";
        assert_eq!(Ok(expected.to_owned()), result);
    }

    #[test]
    fn ifdef_nested() {
        let template =
"This is a normal line.
#ifdef HERP
#ifdef DERP
This line will be included.
#endif
#endif
This is another normal line";
        let mut defs = ShaderPreprocessor::new();
        defs.add("HERP");
        defs.add("DERP");
        let result = defs.preprocess(template);
        let expected =
"This is a normal line.
This line will be included.
This is another normal line";
        assert_eq!(Ok(expected.to_owned()), result);
    }
}
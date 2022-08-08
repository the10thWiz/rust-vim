mod namespace;
mod value;
mod expr;

use expr::ValueError;
use namespace::NamespaceError;

use crate::value::VimFunction;
use crate::value::Function;
use crate::value::Value;
use crate::namespace::NameSpaced;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub enum VimError {
    UnexpectedKeyword(&'static str),
    UnexpectedEof,
    Exit,
    InvalidParams,
    Expected(&'static str),
    NamespaceError(NamespaceError),
    ValError(ValueError),
    VariableUndefined,
}

impl From<NamespaceError> for VimError {
    fn from(n: NamespaceError) -> Self {
        Self::NamespaceError(n)
    }
}
impl From<ValueError> for VimError {
    fn from(n: ValueError) -> Self {
        Self::ValError(n)
    }
}

pub trait Command<S> {
    fn execute(&self, range: CmdRange<'_>, bang: bool, commands: &str, state: &mut S);
}

pub trait BuiltinFunction<S> {
    fn execute(&self, args: Vec<Value>, state: &mut S);
}

pub enum CmdRange<'a> {
    Whole,
    Select(&'a str),
    RangeFrom(usize),
    RangeTo(usize),
    Range { start: usize, end: usize },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Section {
    Script,
    Function,
    If,
    While,
    For,
}

enum RunTy<'a> {
    Now,
    Skip,
    SkipEndIf,
    Function(&'a mut VimFunction),
}

pub struct VimScriptCtx<S> {
    commands: HashMap<String, Arc<dyn Command<S>>>,
    functions: NameSpaced<Function<S>>,
    variables: NameSpaced<Value>,
}

impl<S> VimScriptCtx<S> {
    pub fn init() -> Self {
        Self {
            commands: HashMap::new(),
            functions: NameSpaced::default(),
            variables: NameSpaced::default(),
        }
    }

    pub fn run(&mut self, script: &str) {
        let mut script = Tokenizer { script };
        match self.run_inner(&mut script, Section::Script, RunTy::Now) {
            Ok(()) | Err(VimError::Exit) => (),
            Err(e) => todo!("Handle Error {e:?}"),
        }
    }

    fn run_inner(
        &mut self,
        script: &mut Tokenizer,
        section: Section,
        mut run: RunTy<'_>,
    ) -> Result<(), VimError> {
        while let Some(line) = script.next() {
            match line.command {
                "if" => (),
                "elseif" => (),
                "else" => {
                    if section == Section::If {
                        if matches!(run, RunTy::Skip) {
                            self.run_inner(script, Section::If, RunTy::Now)?;
                        }
                    } else {
                        return Err(VimError::UnexpectedKeyword("else"));
                    }
                }
                "endif" => {
                    if section == Section::If {
                        return Ok(());
                    } else {
                        return Err(VimError::UnexpectedKeyword("endif"));
                    }
                }
                "for" => (),
                "endfor" => {
                    if section == Section::For {
                        return Ok(());
                    } else {
                        return Err(VimError::UnexpectedKeyword("endfor"));
                    }
                }
                "while" => (),
                "endwhile" => {
                    if section == Section::While {
                        return Ok(());
                    } else {
                        return Err(VimError::UnexpectedKeyword("endwhile"));
                    }
                }
                "function" => (),
                "endfunction" => {
                    if section == Section::Function {
                        return Ok(());
                    } else {
                        return Err(VimError::UnexpectedKeyword("endfunction"));
                    }
                }
                "let" => {
                    if line.range.is_some() || line.bang {
                        return Err(VimError::InvalidParams);
                    } else if let Some((name, val)) = line.params.split_once("=") {
                        let val = self.eval(val)?;
                        self.variables.insert(name.trim(), val)?;
                    } else {
                        return Err(VimError::Expected("="));
                    }
                },
                "silent" => todo!("Run cmd & supress output"),
                "execute" => todo!("Eval expression & run resulting script"),
                "call" => todo!("Call function"),
                "finish" => return Err(VimError::Exit),
                "exit" => return Err(VimError::Exit),
                _ => match &mut run {
                    RunTy::Skip | RunTy::SkipEndIf => (),
                    RunTy::Now => todo!("Run normal command"),
                    RunTy::Function(f) => f.inner.push(line.to_owned()),
                },
            }
        }
        if section == Section::Script {
            Ok(())
        } else {
            Err(VimError::UnexpectedEof)
        }
    }

    pub fn eval(&mut self, expr: &str) -> Result<Value, VimError> {
        expr::parse(expr.trim(), self)
    }

    fn get_func(&self, name: impl AsRef<str>) -> Option<&Function<S>> {
        self.functions.get(name).ok().flatten()
    }

    pub fn lookup(&self, variable: impl AsRef<str>) -> Result<&Value, VimError> {
        if variable.as_ref().starts_with("v:") {
            match variable.as_ref() {
                "v:true" => Ok(&Value::TRUE),
                "v:false" => Ok(&Value::FALSE),
                "v:null" => Ok(&Value::NULL),
                _ => Err(VimError::VariableUndefined)
            }
        } else {
            self.variables.get(variable)?.map_or(Err(VimError::VariableUndefined), Ok)
        }
    }
}

struct Tokenizer<'a> {
    script: &'a str,
}

impl<'a> Tokenizer<'a> {
    fn get_next(&mut self) -> Option<Line<'a>> {
        let mut last = ' ';
        let (line, next) = self
            .script
            .split_once(|c: char| {
                let result = (last != '\\' && c == '\n') || c == '|';
                if !c.is_whitespace() {
                    last = c;
                }
                return result;
            })
            .unwrap_or((self.script, ""));
        self.script = next.trim();
        Line::new(line.trim())
    }

    pub fn next(&mut self) -> Option<Line<'a>> {
        while !self.script.is_empty() {
            if let Some(line) = self.get_next() {
                return Some(line);
            }
        }
        None
    }
}

struct Line<'a> {
    range: Option<&'a str>,
    command: &'a str,
    bang: bool,
    params: &'a str,
}

impl<'a> Line<'a> {
    pub fn new(line: &'a str) -> Option<Self> {
        let line = line.trim();
        if line.starts_with("\"") {
            return None;
        }
        let (range, line) = Self::split_range(line);
        let (command, line) = Self::split_command(line);
        let (bang, params) = Self::split_bang(line);
        if !bang && command.is_empty() {
            return None;
        }
        Some(Self {
            range,
            command,
            bang,
            params,
        })
    }

    pub fn split_range<'b>(line: &'b str) -> (Option<&'b str>, &'b str) {
        line.split_once(|c: char| c.is_alphanumeric())
            .map_or((None, line), |(a, b)| (Some(a), b))
    }

    pub fn split_command<'b>(line: &'b str) -> (&'b str, &'b str) {
        line.split_once(|c: char| !c.is_alphanumeric())
            .unwrap_or((line, ""))
    }

    pub fn split_bang<'b>(line: &'b str) -> (bool, &'b str) {
        if line.starts_with("!") {
            (true, line.trim_start_matches("!"))
        } else {
            (false, line)
        }
    }

    fn to_owned(&self) -> LineOwned {
        LineOwned {
            range: self.range.map(|s| s.to_string()),
            command: self.command.to_string(),
            bang: self.bang,
            params: self.params.to_string(),
        }
    }
}

struct LineOwned {
    range: Option<String>,
    command: String,
    bang: bool,
    params: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct TestContext;

    pub fn test_ctx() -> VimScriptCtx<TestContext> {
        VimScriptCtx::init()
    }
}

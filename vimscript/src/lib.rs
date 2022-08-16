mod expr;
mod namespace;
mod value;

use expr::ValueError;
use namespace::NamespaceError;
use value::Names;

use crate::namespace::NameSpaced;
use crate::value::Function;
use crate::value::Value;
use crate::value::VimFunction;
use std::collections::HashMap;
use std::fmt::Arguments;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

pub trait State {
    fn set_silent(&mut self, silent: bool);
    fn echo(&mut self, msg: Arguments);
}

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
    FunctionUndefined,
    CommandUndefined,
    TimeOut,
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
    fn execute(
        &self,
        range: CmdRange<'_>,
        bang: bool,
        commands: &str,
        ctx: &mut VimScriptCtx<S>,
        state: &mut S,
    );
}

pub trait BuiltinFunction<S> {
    fn execute(&self, args: Vec<Value>, ctx: &mut VimScriptCtx<S>, state: &mut S) -> Value;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CmdRange<'a> {
    CurrentLine,
    Whole,
    Select(&'a str),
    RangeFrom(usize),
    RangeTo(usize),
    Range { start: usize, end: usize },
}

#[derive(Debug)]
pub enum CmdRangeOwned {
    CurrentLine,
    Whole,
    Select(String),
    RangeFrom(usize),
    RangeTo(usize),
    Range { start: usize, end: usize },
}

impl CmdRangeOwned {
    pub fn as_ref(&self) -> CmdRange {
        match self {
            CmdRangeOwned::CurrentLine => CmdRange::CurrentLine,
            CmdRangeOwned::Whole => CmdRange::Whole,
            CmdRangeOwned::Select(s) => CmdRange::Select(s.as_str()),
            CmdRangeOwned::RangeFrom(start) => CmdRange::RangeFrom(*start),
            CmdRangeOwned::RangeTo(end) => CmdRange::RangeTo(*end),
            CmdRangeOwned::Range { start, end } => CmdRange::Range {
                start: *start,
                end: *end,
            },
        }
    }
}

impl<'a> CmdRange<'a> {
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::CurrentLine)
    }

    pub fn to_owned(&self) -> CmdRangeOwned {
        match self {
            CmdRange::CurrentLine => CmdRangeOwned::CurrentLine,
            CmdRange::Whole => CmdRangeOwned::Whole,
            CmdRange::Select(s) => CmdRangeOwned::Select(s.to_string()),
            CmdRange::RangeFrom(start) => CmdRangeOwned::RangeFrom(*start),
            CmdRange::RangeTo(end) => CmdRangeOwned::RangeTo(*end),
            CmdRange::Range { start, end } => CmdRangeOwned::Range {
                start: *start,
                end: *end,
            },
        }
    }
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

impl RunTy<'_> {
    fn act(
        &mut self,
        line: Line,
        action: impl FnOnce(Line) -> Result<(), VimError>,
    ) -> Result<(), VimError> {
        match self {
            Self::Skip | Self::SkipEndIf => Ok(()),
            Self::Function(f) => {
                f.inner.push(line.to_owned());
                Ok(())
            }
            Self::Now => action(line),
        }
    }
}

pub struct VimScriptCtx<S> {
    commands: HashMap<String, Arc<dyn Command<S>>>,
    functions: NameSpaced<Function<S>>,
    variables: NameSpaced<Value>,
    timeout: Instant,
    silence_level: usize,
}

impl<S: State> VimScriptCtx<S> {
    pub fn init() -> Self {
        let mut variables = NameSpaced::default();
        variables.insert_builtin("v:true", Value::Bool(true));
        variables.insert_builtin("v:false", Value::Bool(false));
        variables.insert_builtin("v:null", Value::Nil);
        Self {
            commands: HashMap::new(),
            functions: NameSpaced::default(),
            variables,
            timeout: Instant::now() + Duration::from_secs(5),
            silence_level: 0,
        }
    }

    pub fn run(&mut self, script: &str, state: &mut S) {
        self.timeout = Instant::now() + Duration::from_secs(5);
        let mut script = Tokenizer { script };
        match self.run_inner(&mut script, Section::Script, RunTy::Now, state) {
            Ok(()) | Err(VimError::Exit) => (),
            Err(e) => todo!("Handle Error {e:?}"),
        }
    }

    fn run_inner(
        &mut self,
        script: &mut Tokenizer,
        section: Section,
        mut run: RunTy<'_>,
        state: &mut S,
    ) -> Result<(), VimError> {
        while let Some(line) = script.next()? {
            if self.timeout < Instant::now() {
                return Err(VimError::TimeOut);
            }
            self.run_line(script, line, section, run, state)?;
        }
        if section == Section::Script {
            Ok(())
        } else {
            Err(VimError::UnexpectedEof)
        }
    }

    fn run_line(
        &mut self,
        script: &mut Tokenizer,
        line: Line,
        section: Section,
        mut run: RunTy<'_>,
        state: &mut S,
        ) -> Result<(), VimError> {
        match line.command {
            "if" => match &mut run {
                RunTy::Skip | RunTy::SkipEndIf => {
                    self.run_inner(script, Section::If, RunTy::SkipEndIf, state)?
                }
                RunTy::Function(f) => f.inner.push(line.to_owned()),
                RunTy::Now => {
                    if self.eval(line.params, state)?.to_bool(self) {
                        self.run_inner(script, Section::If, RunTy::Now, state)?
                    } else {
                        self.run_inner(script, Section::If, RunTy::Skip, state)?
                    }
                }
            },
            "elseif" => {
                if section == Section::If {
                    match &mut run {
                        RunTy::Function(f) => f.inner.push(line.to_owned()),
                        RunTy::SkipEndIf => (),
                        RunTy::Skip => {
                            if self.eval(line.params, state)?.to_bool(self) {
                                run = RunTy::Now;
                            } else {
                                run = RunTy::SkipEndIf;
                            }
                        }
                        RunTy::Now => {
                            run = RunTy::SkipEndIf;
                        }
                    }
                } else {
                    return Err(VimError::UnexpectedKeyword("else"));
                }
            }
            "else" => {
                if section == Section::If {
                    match &mut run {
                        RunTy::Function(f) => f.inner.push(line.to_owned()),
                        RunTy::SkipEndIf => (),
                        RunTy::Skip => {
                            run = RunTy::Now;
                        }
                        RunTy::Now => {
                            run = RunTy::SkipEndIf;
                        }
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
            "for" => {
                // todo: parse params
                let (names, expr) = Names::parse(line.params)?;
                let expr = expr
                    .trim()
                    .strip_prefix("in")
                    .ok_or(VimError::Expected("in"))?;
                let val = self.eval(expr, state)?;
                for v in val.into_iter() {
                    self.variables.enter_local();
                    names.iter(v, |name, v| {
                        self.variables
                            .insert(name, v)
                            .map(|_| ())
                            .map_err(|e| e.into())
                    })?;
                    // self.insert_var(name, vals)?;
                    self.run_inner(&mut script.clone(), Section::For, RunTy::Now, state)?;
                    self.variables.leave_local();
                }
                self.run_inner(script, Section::For, RunTy::Skip, state)?;
            }
            "endfor" => {
                if section == Section::For {
                    return Ok(());
                } else {
                    return Err(VimError::UnexpectedKeyword("endfor"));
                }
            }
            "while" => {
                while self.eval(line.params, state)?.to_bool(self) {
                    self.run_inner(&mut script.clone(), Section::While, RunTy::Now, state)?;
                }
                self.run_inner(script, Section::While, RunTy::Skip, state)?;
            }
            "endwhile" => {
                if section == Section::While {
                    return Ok(());
                } else {
                    return Err(VimError::UnexpectedKeyword("endwhile"));
                }
            }
            "function" => run.act(line, |line| {
                let mut function = VimFunction::new(vec![line.params.to_owned()]);
                self.run_inner(
                    script,
                    Section::Function,
                    RunTy::Function(&mut function),
                    state,
                )?;
                self.functions
                    .insert("TODO", Function::VimScript(function))?;
                Ok(())
            })?,
            "endfunction" => {
                if section == Section::Function {
                    return Ok(());
                } else {
                    return Err(VimError::UnexpectedKeyword("endfunction"));
                }
            }
            "let" => run.act(line, |line| {
                if line.range.is_some() || line.bang {
                    Err(VimError::InvalidParams)
                } else if let Some((name, val)) = line.params.split_once('=') {
                    let val = self.eval(val, state)?;
                    self.variables.insert(name.trim(), val)?;
                    Ok(())
                } else {
                    Err(VimError::Expected("="))
                }
            })?,
            "silent" => run.act(line, |full_line|{
                if let Some(line) = Line::new(full_line.params)? {
                    self.silence_level += 1;
                    state.set_silent(self.silence_level > 0);
                    self.run_line(script, line, section, run, state)?;
                    state.set_silent(self.silence_level > 0);
                    self.silence_level -= 1;
                }
                Ok(())
            })?,
            "execute" => run.act(line, |line|{
                let v = self.eval(line.params, state)?.to_string(self);
                self.run_inner(&mut Tokenizer { script:v.as_str() }, Section::Script, RunTy::Now, state)
            })?,
            "finish" => return Err(VimError::Exit),
            "exit" => return Err(VimError::Exit),
            _ => run.act(line, |line| {
                if let Some(cmd) = self.commands.get(line.command) {
                    Arc::clone(cmd).execute(line.range, line.bang, line.params, self, state);
                    Ok(())
                } else {
                    Err(VimError::CommandUndefined)
                }
            })?,
        }
        Ok(())
    }

    pub fn run_function(
        &mut self,
        f: &str,
        args: Vec<Value>,
        state: &mut S,
    ) -> Result<Value, VimError> {
        match self.get_func(f) {
            Some(Function::VimScript(_f)) => todo!("Vimscript Functions"),
            Some(Function::Builtin(f)) => Ok(f.clone().execute(args, self, state)),
            None => Err(VimError::FunctionUndefined),
        }
    }

    pub fn eval(&mut self, expr: &str, state: &mut S) -> Result<Value, VimError> {
        expr::parse(expr.trim(), self, state)
    }

    fn get_func(&self, name: impl AsRef<str>) -> Option<&Function<S>> {
        self.functions.get(name).ok().flatten()
    }

    pub fn lookup(&self, variable: impl AsRef<str>) -> Result<&Value, VimError> {
        self.variables
            .get(variable)?
            .map_or(Err(VimError::VariableUndefined), Ok)
    }

    pub fn insert_var(
        &mut self,
        name: impl Into<String>,
        val: Value,
    ) -> Result<Option<Value>, VimError> {
        self.variables.insert(name, val).map_err(|e| e.into())
    }

    pub fn remove_var(&mut self, name: impl AsRef<str>) -> Result<Option<Value>, VimError> {
        self.variables.remove(name).map_err(|e| e.into())
    }

    pub fn command(
        &mut self,
        name: impl Into<String>,
        command: Arc<dyn Command<S> + 'static>,
    ) -> &mut Self {
        self.commands.insert(name.into(), command);
        self
    }

    pub fn builtin(
        &mut self,
        name: impl Into<String>,
        command: Arc<dyn BuiltinFunction<S> + 'static>,
    ) -> &mut Self {
        self.functions
            .insert_builtin(name.into(), Function::Builtin(command));
        self
    }
}

#[derive(Debug, Clone)]
struct Tokenizer<'a> {
    script: &'a str,
}

impl<'a> Tokenizer<'a> {
    fn get_next(&mut self) -> Result<Option<Line<'a>>, VimError> {
        let mut last = ' ';
        let (line, next) = self
            .script
            .split_once(|c: char| {
                let result = (last != '\\' && c == '\n') || c == '|';
                if !c.is_whitespace() {
                    last = c;
                }
                result
            })
            .unwrap_or((self.script, ""));
        self.script = next.trim();
        Line::new(line.trim())
    }

    pub fn next(&mut self) -> Result<Option<Line<'a>>, VimError> {
        while !self.script.is_empty() {
            if let Some(line) = self.get_next()? {
                return Ok(Some(line));
            }
        }
        Ok(None)
    }
}

#[derive(Debug)]
struct Line<'a> {
    range: CmdRange<'a>,
    command: &'a str,
    bang: bool,
    params: &'a str,
}

impl<'a> Line<'a> {
    pub fn new(line: &'a str) -> Result<Option<Self>, VimError> {
        let line = line.trim();
        if line.starts_with('\"') {
            return Ok(None);
        }
        let (range, line) = Self::split_range(line)?;
        let (command, line) = Self::split_command(line);
        let (bang, params) = Self::split_bang(line);
        if !bang && command.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            range,
            command,
            bang,
            params: params.trim(),
        }))
    }

    pub fn split_range(line: &str) -> Result<(CmdRange, &str), VimError> {
        if let Some(line) = line.strip_prefix('/') {
            let mut last = '/';
            if let Some((pattern, line)) = line.split_once(|c: char| {
                // Filter for \/ to allow escapes
                let res = c == '/' && last != '\\';
                last = c;
                res
            }) {
                Ok((CmdRange::Select(pattern), line))
            } else {
                Err(VimError::Expected("/"))
            }
        } else if let Some(line) = line.strip_prefix('%') {
            Ok((CmdRange::Whole, line))
        } else {
            let idx = line.find(|c: char| c.is_alphabetic()).unwrap_or(line.len());
            let rem = &line[idx..];
            match line[..idx].split_once(',') {
                Some(("", "")) => Ok((CmdRange::Whole, rem)),
                Some(("", end)) => str::parse(end)
                    .map(|e| (CmdRange::RangeTo(e), rem))
                    .map_err(|_| VimError::Expected("Number")),
                Some((start, "")) => str::parse(start)
                    .map(|s| (CmdRange::RangeFrom(s), rem))
                    .map_err(|_| VimError::Expected("Number")),
                Some((start, end)) => Ok((
                    CmdRange::Range {
                        start: str::parse(start).map_err(|_| VimError::Expected("Number"))?,
                        end: str::parse(end).map_err(|_| VimError::Expected("Number"))?,
                    },
                    rem,
                )),
                None => Ok((CmdRange::CurrentLine, rem)),
            }
        }
    }

    pub fn split_command(line: &str) -> (&str, &str) {
        if let Some(idx) = line.find(|c: char| !c.is_alphanumeric()) {
            (&line[..idx], &line[idx..])
        } else {
            (line, "")
        }
    }

    pub fn split_bang(line: &str) -> (bool, &str) {
        if let Some(line) = line.strip_prefix('!') {
            (true, line)
        } else {
            (false, line)
        }
    }

    fn to_owned(&self) -> LineOwned {
        LineOwned {
            range: self.range.to_owned(),
            command: self.command.to_string(),
            bang: self.bang,
            params: self.params.to_string(),
        }
    }
}

#[derive(Debug)]
struct LineOwned {
    range: CmdRangeOwned,
    command: String,
    bang: bool,
    params: String,
}

impl LineOwned {
    fn as_ref(&self) -> Line {
        Line {
            range: self.range.as_ref(),
            command: self.command.as_str(),
            bang: self.bang,
            params: self.params.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    pub struct TestContext;

    impl State for TestContext {
        fn set_silent(&mut self, _s: bool) { }
    }

    pub fn test_ctx() -> VimScriptCtx<TestContext> {
        VimScriptCtx::init()
    }

    pub struct ExpectCall(
        Box<dyn Fn(CmdRange<'_>, bool, &str, &mut VimScriptCtx<TestContext>, &mut TestContext)>,
        AtomicUsize,
    );

    pub fn command(
        f: impl Fn(CmdRange<'_>, bool, &str, &mut VimScriptCtx<TestContext>, &mut TestContext) + 'static,
    ) -> (Arc<ExpectCall>, Arc<ExpectCall>) {
        let call = Arc::new(ExpectCall(Box::new(f), AtomicUsize::new(0)));
        (call.clone(), call)
    }

    impl Command<TestContext> for ExpectCall {
        fn execute(
            &self,
            range: CmdRange<'_>,
            bang: bool,
            commands: &str,
            ctx: &mut VimScriptCtx<TestContext>,
            state: &mut TestContext,
        ) {
            self.0(range, bang, commands, ctx, state);
            self.1.fetch_add(1, Ordering::AcqRel);
        }
    }

    impl ExpectCall {
        pub fn called(&self) -> usize {
            self.1.load(Ordering::Acquire)
        }
    }

    #[test]
    fn simple_command() {
        let (guard, cmd) = command(|_r, _b, _a, _c, _s| ());
        test_ctx()
            .command("Test", cmd)
            .run("Test", &mut TestContext);
        assert_eq!(guard.called(), 1, "Was not called once");
    }

    macro_rules! check_command {
        ($cmd:literal, $name:literal => $exp:expr) => {
            check_command!($cmd, $name, 1 => $exp);
        };
        ($cmd:literal, $name:literal, $num:literal => $exp:expr) => { {
            let (guard, cmd) = command($exp);
            test_ctx().command($name, cmd).run($cmd, &mut TestContext);
            assert_eq!(guard.called(), $num, "Was not called once");
        } };
    }

    #[test]
    fn ranged_command() {
        check_command!("Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::CurrentLine);
        });
        check_command!("1,Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::RangeFrom(1));
        });
        check_command!(",1Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::RangeTo(1));
        });
        check_command!("1,4Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::Range { start: 1, end: 4 });
        });
        check_command!(",Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::Whole);
        });
        check_command!("%Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::Whole);
        });
        check_command!("/smth/Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::Select("smth"));
        });
        check_command!("/smt\\/h/Test", "Test" => |r, _b, _a, _c, _s| {
            assert_eq!(r, CmdRange::Select("smt\\/h"));
        });
    }

    #[test]
    fn command_params() {
        check_command!("Test abc", "Test" => |_r, _b, a, _c, _s| {
            assert_eq!(a, "abc");
        });
        check_command!("Test abcasdfasdl;fkjasd;lkfjsad;lfkj", "Test" => |_r, _b, a, _c, _s| {
            assert_eq!(a, "abcasdfasdl;fkjasd;lkfjsad;lfkj");
        });
        check_command!("Test/abcasdfasdl;fkjasd;lkfjsad;lfkj", "Test" => |_r, _b, a, _c, _s| {
            assert_eq!(a, "/abcasdfasdl;fkjasd;lkfjsad;lfkj");
        });
    }

    #[test]
    fn command_bang() {
        check_command!("Test!", "Test" => |_r, b, _a, _c, _s| {
            assert!(b);
        });
        check_command!("Test! some cmd", "Test" => |_r, b, a, _c, _s| {
            assert!(b);
            assert_eq!(a, "some cmd");
        });
    }

    #[test]
    fn let_expr() {
        let mut ctx = test_ctx();
        ctx.run("let g:a = ''", &mut TestContext);
        assert_eq!(ctx.lookup("g:a").unwrap(), &Value::str(""));
        ctx.run("let g:b = g:a", &mut TestContext);
        assert_eq!(ctx.lookup("g:b").unwrap(), &Value::str(""));
    }

    #[test]
    fn multi_command() {
        check_command!("Test | Test", "Test", 2 => |_, _, _, _c, _|());
    }

    #[test]
    fn if_expr() {
        check_command!("if v:false | Test | endif ", "Test", 0 => |_, _, _, _c, _|());
        check_command!("if v:true  | Test | endif ", "Test", 1 => |_, _, _, _c, _|());
        check_command!("if v:false | else | Test | endif ", "Test", 1 => |_, _, _, _c, _|());
    }

    #[test]
    fn while_expr() {
        check_command!("while v:false | Test | endwhile ", "Test", 0 => |_, _, _, _c, _|());
        check_command!("let g:a = 0 | while g:a < 4 | Test | let g:a = g:a + 1 | endwhile ", "Test", 4 => |_, _, _, _c, _|());
    }

    #[test]
    fn for_expr() {
        check_command!("for a in [] | Test | endfor ", "Test", 0 => |_, _, _, _c, _|());
        check_command!("for a in [0] | Test | endfor ", "Test", 1 => |_, _, _, ctx, s| {
            assert_eq!(ctx.eval("a", s).unwrap(), Value::Integer(0));
        });
    }
}

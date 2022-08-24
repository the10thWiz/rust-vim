#![feature(round_char_boundary, concat_idents)]

mod args;
mod buffer;
mod builtin;
mod cli;
mod cursor;
mod keymap;
mod options;
mod util;
mod window;

use crate::buffer::BufferSelect;
use std::{
    borrow::Cow,
    fmt::Display,
    fs::File,
    io::{self, ErrorKind, Read, Stdout, StdoutLock, Write},
    path::{Path, PathBuf},
    time::Duration, panic::Location,
};

use args::Args;
use backtrace::{Backtrace, BacktraceFmt, BacktraceFrame, BacktraceSymbol, BytesOrWideString};
use buffer::BufferRef;
use clap::Parser;
use cli::{Cli, CliState};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent,
    },
    style::{Color, ContentStyle, SetBackgroundColor, Stylize},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, DisableLineWrap, EnableLineWrap,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
    QueueableCommand,
};
use cursor::Cursor;
use keymap::{Action, KeyState, MapAction, MapSet};
use log::{error, info};
use options::{Options, Opts};
use util::{Area, Pos};
use vimscript::{Id, IdProcuder, State, Value, VimError, VimScriptCtx};
use window::{Scroll, WinMode, Window};

pub use crossterm::Result;

pub trait Lockable {
    type Lock: Write;
    fn lock(&self) -> Self::Lock;
}

impl Lockable for Stdout {
    type Lock = StdoutLock<'static>;
    fn lock(&self) -> Self::Lock {
        self.lock()
    }
}

pub trait EventReader {
    type Act: Action;
    fn on_key(&mut self, key: KeyEvent) -> Self::Act;
    fn on_mouse(&mut self, mouse: MouseEvent) -> Self::Act;
}

pub trait Renderable {
    fn set_area(&mut self, new_area: Area);
    fn area(&self) -> Area;
    fn cursor_pos(&self) -> Cursor;
    fn draw<W: Write>(&mut self, term: &mut W) -> Result<()>;
}

pub enum WindowSet {
    Window(Window),
    Horizontal(Vec<WindowSet>, usize, Area),
    Vertical(Vec<WindowSet>, usize, Area),
}

// impl From<&Vec<BufferRef>> for WindowSet {
// }

impl WindowSet {
    fn new(ids: &mut IdProcuder, buffers: &[BufferRef]) -> Self {
        if buffers.len() == 1 {
            Self::Window(Window::new(ids.get(), buffers[0].clone()))
        } else {
            Self::Horizontal(
                buffers
                    .iter()
                    .map(|b| Self::Window(Window::new(ids.get(), b.clone())))
                    .collect(),
                0,
                Area::default(),
            )
        }
    }

    fn get_focus(&self) -> &Window {
        match self {
            Self::Window(w) => w,
            Self::Horizontal(set, focused, _) | Self::Vertical(set, focused, _) => {
                set[*focused].get_focus()
            }
        }
    }

    fn get_focus_mut(&mut self) -> &mut Window {
        match self {
            Self::Window(w) => w,
            Self::Horizontal(set, focused, _) | Self::Vertical(set, focused, _) => {
                set[*focused].get_focus_mut()
            }
        }
    }

    pub fn redraw_all(&mut self) {
        match self {
            Self::Window(w) => w.redraw_all(),
            Self::Horizontal(set, _, _) | Self::Vertical(set, _, _) => {
                for s in set.iter_mut() {
                    s.redraw_all();
                }
            }
        }
    }

    /// Move the focus in the direction requested
    ///
    /// Returns whether the motion could be completed
    fn move_focus(&mut self, motion: Scroll) -> bool {
        match self {
            Self::Window(_) => false,
            Self::Horizontal(set, focused, _) => match motion {
                Scroll::Left => {
                    if set[*focused].move_focus(motion) {
                        true
                    } else if *focused == 0 {
                        false
                    } else {
                        *focused -= 1;
                        true
                    }
                }
                Scroll::Right => {
                    if set[*focused].move_focus(motion) {
                        true
                    } else if *focused >= set.len() - 1 {
                        false
                    } else {
                        *focused += 1;
                        true
                    }
                }
                Scroll::Up | Scroll::Down => set[*focused].move_focus(motion),
            },
            Self::Vertical(set, focused, _) => match motion {
                Scroll::Up => {
                    if set[*focused].move_focus(motion) {
                        true
                    } else if *focused == 0 {
                        false
                    } else {
                        *focused -= 1;
                        true
                    }
                }
                Scroll::Down => {
                    if set[*focused].move_focus(motion) {
                        true
                    } else if *focused >= set.len() - 1 {
                        false
                    } else {
                        *focused += 1;
                        true
                    }
                }
                Scroll::Left | Scroll::Right => set[*focused].move_focus(motion),
            },
        }
    }

    /// Sets the current focus to the window contianing the buffer selected by the criteria
    fn jump_to(&mut self, criteria: &impl BufferSelect) -> bool {
        match self {
            Self::Window(w) => w.buffer_select(criteria),
            Self::Horizontal(set, focused, _) | Self::Vertical(set, focused, _) => {
                for (i, s) in set.iter_mut().enumerate() {
                    if s.jump_to(criteria) {
                        *focused = i;
                        return true;
                    }
                }
                false
            }
        }
    }

    fn buffer(&self) -> &BufferRef {
        match self {
            Self::Window(w) => w.buffer(),
            Self::Horizontal(set, focused, _) | Self::Vertical(set, focused, _) => {
                set[*focused].buffer()
            }
        }
    }

    fn split_vertical(&mut self, new: Window) {
        match self {
            Self::Window(w) => {
                let area = w.area();
                let mut n = Self::Vertical(vec![Self::Window(new)], 1, area);
                std::mem::swap(self, &mut n);
                if let Self::Vertical(set, ..) = self {
                    set.insert(0, n);
                } else {
                    unreachable!("self was just set to vertical");
                }
                self.set_area(area);
            }
            Self::Horizontal(set, focused, _) => {
                set[*focused].split_vertical(new);
            }
            Self::Vertical(v, _, area) => {
                v.push(Self::Window(new));
                let area = *area;
                self.set_area(area);
            }
        }
    }

    fn split_horizontal(&mut self, new: Window) {
        match self {
            Self::Window(w) => {
                let area = w.area();
                let mut n = Self::Horizontal(vec![Self::Window(new)], 1, area);
                std::mem::swap(self, &mut n);
                if let Self::Horizontal(set, ..) = self {
                    set.insert(0, n);
                } else {
                    unreachable!("self was just set to vertical");
                }
                self.set_area(area);
            }
            Self::Vertical(set, focused, _) => {
                set[*focused].split_horizontal(new);
            }
            Self::Horizontal(v, _, area) => {
                v.push(Self::Window(new));
                let area = *area;
                self.set_area(area);
            }
        }
    }

    /// Removes window with matching Id. returns whether the set as a whole needs to be removed.
    fn remove_window(&mut self, id: Id) -> bool {
        match self {
            Self::Window(w) => w.id() == id,
            Self::Vertical(set, focused, area) | Self::Horizontal(set, focused, area) => {
                let area = *area;
                if let Some(idx) = set.iter_mut().position(|w| w.remove_window(id)) {
                    if *focused > idx {
                        *focused -= 1;
                    }
                    set.remove(idx);
                }
                if set.len() == 1 {
                    let mut w = set.remove(0);
                    std::mem::swap(self, &mut w);
                }
                self.set_area(area);
                false
            }
        }
    }
}

impl Renderable for WindowSet {
    fn set_area(&mut self, new_area: Area) {
        match self {
            Self::Window(w) => w.set_area(new_area),
            Self::Horizontal(set, _, area) => {
                *area = new_area;
                let total: usize = set.iter().map(|w| w.area().width()).sum();
                let mut cur = 0;
                for win in set.iter_mut() {
                    let percent = win.area().width() as f64 / total as f64;
                    let new_width = percent * new_area.width() as f64;
                    win.set_area(Area {
                        x: cur,
                        y: new_area.y,
                        w: new_width as usize,
                        h: new_area.height(),
                    });
                    cur += new_width as usize;
                }
                if let Some(set) = set.last_mut() {
                    let mut area = set.area();
                    area.w += new_area.width() - cur;
                    set.set_area(area);
                }
            }
            Self::Vertical(set, _, area) => {
                *area = new_area;
                let total: usize = set.iter().map(|w| w.area().height()).sum();
                let mut cur = 0;
                for win in set.iter_mut() {
                    let percent = win.area().height() as f64 / total as f64;
                    let new_height = percent * new_area.height() as f64;
                    win.set_area(Area {
                        x: cur,
                        y: new_area.y,
                        w: new_area.width(),
                        h: new_height as usize,
                    });
                    cur += new_height as usize;
                }
                if let Some(set) = set.last_mut() {
                    let mut area = set.area();
                    area.h += new_area.height() - cur;
                    set.set_area(area);
                }
            }
        }
    }

    fn area(&self) -> Area {
        match self {
            Self::Window(w) => w.area(),
            Self::Horizontal(_, _, a) | Self::Vertical(_, _, a) => *a,
        }
    }

    fn cursor_pos(&self) -> Cursor {
        self.get_focus().cursor_pos()
    }

    fn draw<W: Write>(&mut self, term: &mut W) -> Result<()> {
        match self {
            Self::Window(w) => w.draw(term),
            Self::Vertical(set, _, _) | Self::Horizontal(set, _, _) => {
                set.iter_mut().try_for_each(|w| w.draw(term))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalState {
    Window,
    Cli,
    Exit,
}

pub struct Vim {
    inner: VimInner,
    ctx: VimScriptCtx<VimInner>,
}

impl std::ops::Deref for Vim {
    type Target = VimInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for Vim {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Vim {
    pub fn new() -> Self {
        let mut ctx = VimScriptCtx::init();
        cli::commands::default(&mut ctx);
        Self {
            inner: VimInner::new(),
            ctx,
        }
    }

    fn init(&mut self) {
        self.inner.init(&mut self.ctx);
    }

    pub fn execute(&mut self, script: &str) {
        match self.ctx.run(script, &mut self.inner) {
            Ok(()) => (),
            Err(e) => self.inner.message(format!("{e:?}")),
        }
    }

    pub fn exec_file(&mut self, file: impl AsRef<Path>) {
        self.ctx.set_script(Some(self.inner.get_next_script_id()));
        match self.exec_file_inner(file) {
            Ok(()) => (),
            Err(e) => self.inner.message(format!("{e:?}")),
        }
        self.ctx.set_script(None);
    }

    fn exec_file_inner(&mut self, file: impl AsRef<Path>) -> std::result::Result<(), VimError> {
        let mut s = String::new();
        File::open(file)?.read_to_string(&mut s)?;
        self.ctx.run(s.as_str(), &mut self.inner)
    }

    fn on_event(&mut self, event: Event) {
        match event {
            Event::Resize(c, r) => self.inner.update_area((c, r)),
            Event::Key(k) => {
                if k.code == KeyCode::Char('c') && k.modifiers == KeyModifiers::CONTROL {
                    self.inner.state = TerminalState::Exit;
                } else {
                    let state = self.inner.get_state();
                    match self.state {
                        TerminalState::Window => match self.inner.map_set.on_key(k, state) {
                            MapAction::Act(rep, a) => {
                                for _ in 0..rep {
                                    a.run(self);
                                }
                            }
                            MapAction::Wait => info!("{:?}", self.inner.map_set),
                            MapAction::None => self.inner.get_focus_mut().on_key(k).run(self),
                        },
                        TerminalState::Cli => self.inner.cli.on_key(k).run(self),
                        TerminalState::Exit => (),
                    }
                }
            }
            Event::Mouse(m) => match self.state {
                TerminalState::Window => self.inner.get_focus_mut().on_mouse(m).run(self),
                TerminalState::Cli => self.inner.cli.on_mouse(m).run(self),
                TerminalState::Exit => (),
            },
        }
    }
}

impl Default for Vim {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VimInner {
    args: Args,
    options: Options,
    buffers: Vec<BufferRef>,
    windows: WindowSet,
    focus: usize,
    floating: Vec<Window>,
    size: (u16, u16),
    state: TerminalState,
    cursor: Cursor,
    map_set: MapSet,
    cli: CliState,
    silent: bool,
    buffer_id: IdProcuder,
    window_id: IdProcuder,
    script_id: IdProcuder,
}

impl State for VimInner {
    fn set_silent(&mut self, silent: bool) {
        self.silent = silent;
    }

    fn echo(&mut self, msg: std::fmt::Arguments) {
        if !self.silent {
            self.message(format!("{}", msg));
        }
    }

    fn get_option(&self, name: &str) -> std::result::Result<Value, VimError> {
        self.options.get(name).map(|v| v.into())
    }
}

impl Default for VimInner {
    fn default() -> Self {
        Self::new()
    }
}

impl VimInner {
    pub fn new() -> Self {
        let mut buffer_id = IdProcuder::default();
        let mut window_id = IdProcuder::default();
        let mut script_id = IdProcuder::default();
        let args = Args::parse();
        let mut buffers: Vec<_> = args
            .files
            .iter()
            .map(|p| BufferRef::from_file(&mut buffer_id, p.clone()).unwrap())
            .collect();
        if buffers.is_empty() {
            buffers.push(BufferRef::empty(&mut buffer_id));
        }
        Self {
            args,
            options: Options::new(),
            windows: WindowSet::new(&mut window_id, &buffers),
            buffers,
            floating: vec![],
            size: (0, 0),
            state: TerminalState::Window,
            cursor: Cursor::invalid(),
            focus: 0,
            map_set: MapSet::global(),
            cli: CliState::new(),
            silent: false,
            buffer_id,
            window_id,
            script_id,
        }
    }

    pub fn shell_expand<'v>(&self, var: impl Into<Cow<'v, str>>) -> Cow<'v, str> {
        let var = var.into();
        if var.contains('$') {
            let mut var = var.as_ref();
            let mut ret = String::new();
            while let Some((prefix, rem)) = var.split_once('$') {
                ret += prefix;
                let end = rem
                    .find(|c: char| !c.is_alphanumeric())
                    .unwrap_or(rem.len());
                if let Ok(val) = std::env::var(&rem[..end]) {
                    ret += &val;
                } else if &rem[..end] == "XDG_CONFIG_HOME" {
                    ret += std::env::var("HOME").unwrap_or("~".to_string()).as_str();
                    ret += "/.config";
                } else if &rem[..end] == "XDG_DATA_HOME" {
                    ret += std::env::var("HOME").unwrap_or("~".to_string()).as_str();
                    ret += "/.cache";
                }
                var = &rem[end..];
            }
            ret += var;
            Cow::Owned(ret)
        } else {
            var
        }
    }

    pub fn find_on_rtp(
        &self,
        name: impl AsRef<str>,
    ) -> std::result::Result<(String, File), io::Error> {
        let name = name.as_ref();
        for path in self.options.runtimepath.split(',') {
            let name = self.shell_expand(format!("{path}/{name}"));
            if let Ok(f) = File::open(name.as_ref()) {
                return Ok((name.into_owned(), f));
            }
        }
        Err(io::Error::new(
            ErrorKind::NotFound,
            "File was not found on 'runtimepath'",
        ))
    }

    fn init(&mut self, ctx: &mut VimScriptCtx<Self>) {
        builtin::builtin_functions(ctx);
        if let Ok((init_path, mut init)) = self.find_on_rtp("init.vim") {
            info!("Using {init_path} as init file");
            let mut buf = String::new();
            if let Ok(_) = init.read_to_string(&mut buf) {
                ctx.set_script(Some(self.get_next_script_id()));
                match ctx.run(buf.as_str(), self) {
                    Ok(()) => (),
                    Err(e) => self.echo(format_args!("{e:?}")),
                }
                ctx.set_script(None);
            }
        } else {
            info!("`init.vim` not found");
        }
        // TODO: potentially run additional init files as requested by the main init file. These
        // can and should be with a seperate script id, and ideally I should allow lazy loading at
        // some point. For now, this is good enough for me. (sort of - the rtp doesn't work right
        // yet)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn start_cli(&mut self, ty: Cli) {
        self.cli.start(ty);
        self.state = TerminalState::Cli;
    }

    pub fn end_cli(&mut self) {
        self.cli.end();
        self.state = TerminalState::Window;
    }

    pub fn exit(&mut self) {
        self.state = TerminalState::Exit;
    }

    pub fn set_mode(&mut self, mode: WinMode) -> &mut Window {
        self.message(mode.get_message().to_string());
        let win = self.get_focus_mut();
        win.set_mode(mode);
        win
    }

    pub fn move_focus(&mut self, motion: Scroll) {
        if self.focus == self.floating.len() {
            self.windows.move_focus(motion);
        } else {
            todo!("Floating window motion")
        }
    }

    pub fn select_focus(&mut self, criteria: impl BufferSelect) {
        if self.windows.jump_to(&criteria) {
            self.focus = self.floating.len();
        } else {
            for (i, w) in self.floating.iter().enumerate() {
                if w.buffer_select(&criteria) {
                    self.focus = i;
                    break;
                }
            }
        }
    }

    fn update_area(&mut self, size: (u16, u16)) {
        if size.1 == 0 || size.0 == 0 {
            panic!("Why is the terminal at a size of {:?}?", size);
        }
        if size != self.size {
            self.windows.set_area(Area {
                x: 0,
                y: 0,
                w: size.0 as usize,
                h: size.1 as usize - 1,
            });
            self.cli.set_area(Area {
                x: 0,
                y: size.1 as usize - 1,
                w: size.0 as usize,
                h: 1,
            });
            // TODO: adjust floating windows
            self.size = size;
        }
    }

    pub fn get_state(&self) -> KeyState {
        match self.state {
            TerminalState::Cli => KeyState::Cli,
            TerminalState::Exit => KeyState::Normal,
            TerminalState::Window => self.get_focus().get_state(),
        }
    }

    fn draw<W: Write>(&mut self, mut lock: W) -> Result<()> {
        self.windows.draw(&mut lock)?;
        self.cli.draw(&mut lock)?;
        match self.state {
            TerminalState::Window => {
                self.cursor = self.get_focus().cursor_pos();
                Pos(
                    self.size.0.saturating_sub(20) as usize,
                    self.size.1.saturating_sub(1) as usize,
                )
                .move_cursor(&mut lock)?;
                self.map_set.draw(&mut lock, self.get_state())?;
            }
            TerminalState::Cli => self.cursor = self.cli.cursor_pos(),
            TerminalState::Exit => (),
        }
        self.cursor.draw(&mut lock)?;
        Ok(())
    }

    pub fn exiting(&self) -> bool {
        self.state == TerminalState::Exit
    }

    pub fn get_focus(&self) -> &Window {
        if self.focus == self.floating.len() {
            self.windows.get_focus()
        } else {
            &self.floating[self.focus]
        }
    }

    pub fn get_focus_mut(&mut self) -> &mut Window {
        if self.focus == self.floating.len() {
            self.windows.get_focus_mut()
        } else {
            &mut self.floating[self.focus]
        }
    }

    pub fn get_message(&self) -> &str {
        self.cli.get_message()
    }

    pub fn message(&mut self, message: String) {
        self.cli.message(message);
    }

    pub fn err(&mut self, error: Result<()>) {
        if let Err(e) = error {
            self.message(format!("{e}"));
        }
    }

    pub fn create_empty_buffer(&mut self) -> BufferRef {
        let buffer = BufferRef::empty(&mut self.buffer_id);
        self.buffers.push(buffer.clone());
        buffer
    }

    pub fn open_file(&mut self, path: impl Into<PathBuf>) -> Result<BufferRef> {
        let buffer = BufferRef::from_file(&mut self.buffer_id, path)?;
        self.buffers.push(buffer.clone());
        Ok(buffer)
    }

    pub fn split_vertical(&mut self, buffer: BufferRef) {
        self.windows
            .split_vertical(Window::new(self.window_id.get(), buffer));
    }

    pub fn split_horizontal(&mut self, buffer: BufferRef) {
        self.windows
            .split_horizontal(Window::new(self.window_id.get(), buffer));
    }

    fn get_next_script_id(&mut self) -> Id {
        self.script_id.get()
    }
}

pub struct Curse<W: Lockable> {
    vim: Vim,
    terminal: W,
}

impl Curse<Stdout> {
    pub fn stdout() -> Self {
        Self::new(std::io::stdout())
    }
}

impl<W: Lockable> Curse<W> {
    pub fn new(terminal: W) -> Self {
        Self {
            terminal,
            vim: Vim::new(),
        }
    }

    pub fn run(mut self) -> Result<()> {
        std::panic::set_hook(Box::new(panic_cleanup));
        enable_raw_mode()?;
        {
            let mut lock = self.terminal.lock();
            lock.queue(DisableLineWrap)?;
            lock.queue(EnterAlternateScreen)?;
            lock.queue(EnableMouseCapture)?;
        }
        self.vim.init();
        self.event_loop()?;
        disable_raw_mode()?;
        {
            let mut lock = self.terminal.lock();
            lock.queue(EnableLineWrap)?;
            lock.queue(LeaveAlternateScreen)?;
            lock.queue(DisableMouseCapture)?;
        }
        Ok(())
    }

    fn event_loop(&mut self) -> Result<()> {
        self.vim.update_area(terminal::size()?);
        self.draw()?;
        while !self.vim.exiting() {
            if event::poll(Duration::from_millis(20))? {
                let e = event::read()?;
                self.vim.on_event(e);
            }
            self.draw()?;
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let mut lock = self.terminal.lock();
        self.vim.draw(&mut lock)?;
        lock.flush()?;
        Ok(())
    }
}

#[allow(unused_must_use)]
fn panic_cleanup(info: &std::panic::PanicInfo) {
    let mut terminal = std::io::stdout();
    disable_raw_mode();
    terminal.queue(EnableLineWrap);
    terminal.queue(LeaveAlternateScreen);
    terminal.queue(DisableMouseCapture);
    terminal.flush();
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        error!("Error: {}", s);
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        error!("Error: {}", s);
    } else {
        error!("Error ty: {:?}", info.payload().type_id());
    }
    if let Some(loc) = info.location() {
        error!(
            "A Panic occured at: {}:{}:{}",
            loc.file(),
            loc.line(),
            loc.column()
        );
        error!("Full backtrace:\n{}", Trimmed(loc, Backtrace::new()));
    } else {
        error!("A Panic occured somewhere");
        error!("Full backtrace:\n{:?}", Backtrace::new());
    }
}

struct Trimmed<'a>(&'a Location<'a>, Backtrace);

fn symbol_starts_with(frame: &BacktraceFrame, pat: &str) -> bool {
    frame.symbols().iter().any(|s| {
        s.name()
            .map(|n| format!("{n}").starts_with(pat))
            .unwrap_or(true)
    })
}

fn is(location: &Location, symbol: &BacktraceSymbol) -> bool {
    location.file() == format!("{}", symbol.filename().unwrap().display())
        && location.line() == symbol.lineno().unwrap_or(0)
        && location.column() == symbol.colno().unwrap_or(0)
}

impl Display for Trimmed<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut path_formatter =
            |f: &mut std::fmt::Formatter<'_>, s: BytesOrWideString<'_>| s.fmt(f);
        let mut f = BacktraceFmt::new(fmt, backtrace::PrintFmt::Short, &mut path_formatter);
        f.add_context();
        self.1
            .frames()
            .iter()
            .skip_while(|f| !f.symbols().iter().any(|symbol: &BacktraceSymbol| is(self.0, symbol)))
            .take_while(|f| !symbol_starts_with(f, "std::rt::lang_start"))
            .map(|frame| f.frame().backtrace_frame(frame))
            .collect::<std::fmt::Result>()?;
        f.finish()
    }
}

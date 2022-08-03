#![feature(round_char_boundary)]

mod args;
mod buffer;
mod cli;
mod cursor;
mod keymap;
mod util;
mod window;

use std::{
    io::{Stdout, StdoutLock, Write},
    time::Duration,
};

use args::Args;
use buffer::BufferRef;
use clap::Parser;
use cli::{Cli, CliState};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent,
    },
    terminal::{
        self, disable_raw_mode, enable_raw_mode, DisableLineWrap, EnableLineWrap,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
    QueueableCommand,
};
use cursor::Cursor;
use keymap::{Action, MapAction, MapSet, State};
use log::error;
use util::Area;
use window::{Window, WinMode};

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

impl WindowSet {
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
}

impl Renderable for WindowSet {
    fn set_area(&mut self, new_area: Area) {
        match self {
            Self::Window(w) => w.set_area(new_area),
            Self::Horizontal(set, _, area) => {
                *area = new_area;
                todo!("Horizontal layout");
            }
            Self::Vertical(set, _, area) => {
                *area = new_area;
                todo!("Vertical layout");
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
                set.iter_mut().map(|w| w.draw(term)).collect()
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
    args: Args,
    buffers: Vec<BufferRef>,
    windows: WindowSet,
    focus: usize,
    floating: Vec<Window>,
    size: (u16, u16),
    state: TerminalState,
    cursor: Cursor,
    map_set: MapSet,
    cli: CliState,
}

impl Vim {
    pub fn new() -> Self {
        let args = Args::parse();
        let mut buffers: Vec<_> = args
            .files
            .iter()
            .map(|p| BufferRef::from_file(p.clone()).unwrap())
            .collect();
        if buffers.is_empty() {
            buffers.push(BufferRef::empty());
        }
        Self {
            args,
            windows: WindowSet::Window(Window::new(buffers[0].clone())),
            buffers,
            floating: vec![],
            size: (0, 0),
            state: TerminalState::Window,
            cursor: Cursor::invalid(),
            focus: 0,
            map_set: MapSet::global(),
            cli: CliState::new(),
        }
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

    fn on_event(&mut self, event: Event) {
        match event {
            Event::Resize(c, r) => self.update_area((c, r)),
            Event::Key(k) => {
                if k.code == KeyCode::Char('c') && k.modifiers == KeyModifiers::CONTROL {
                    self.state = TerminalState::Exit;
                } else {
                    match self.state {
                        TerminalState::Window => match self.map_set.on_key(k, self.get_state()) {
                            MapAction::Act(a) => a.run(self),
                            MapAction::Wait => (),
                            MapAction::None => self.get_focus_mut().on_key(k).run(self),
                        },
                        TerminalState::Cli => self.cli.on_key(k).run(self),
                        TerminalState::Exit => (),
                    }
                }
            }
            Event::Mouse(m) => match self.state {
                TerminalState::Window => self.get_focus_mut().on_mouse(m).run(self),
                TerminalState::Cli => self.cli.on_mouse(m).run(self),
                TerminalState::Exit => (),
            },
        }
    }

    pub fn get_state(&self) -> State {
        match self.state {
            TerminalState::Cli => State::Cli,
            TerminalState::Exit => State::Normal,
            TerminalState::Window => self.get_focus().get_state(),
        }
    }

    fn draw<W: Write>(&mut self, mut lock: W) -> Result<()> {
        self.windows.draw(&mut lock)?;
        self.cli.draw(&mut lock)?;
        match self.state {
            TerminalState::Window => {
                self.cursor = self.get_focus().cursor_pos();
            }
            TerminalState::Cli => self.cursor = self.cli.cursor_pos(),
            TerminalState::Exit => (),
        }
        self.cursor.draw(&mut lock)?;
        //self.message(format!("{:?}", self.cursor));
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
        enable_raw_mode()?;
        {
            let mut lock = self.terminal.lock();
            lock.queue(DisableLineWrap)?;
            lock.queue(EnterAlternateScreen)?;
            lock.queue(EnableMouseCapture)?;
        }
        std::panic::set_hook(Box::new(panic_cleanup));
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
    } else {
        error!("A Panic occured somewhere");
    }
}

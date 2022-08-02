mod buffer;
mod cursor;
mod util;
mod window;
mod keymap;
mod args;

use std::{
    io::{Stdout, StdoutLock, Write},
    time::Duration,
};

use args::Args;
use buffer::BufferRef;
use clap::Parser;
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
use keymap::{Action, MapSet, State, MapAction};
use log::error;
use util::Area;
use window::Window;

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
}

impl Vim {
    pub fn new() -> Self {
        let args = Args::parse();
        let empty = BufferRef::empty();
        Self {
            args,
            buffers: vec![empty.clone()],
            windows: WindowSet::Window(Window::new(empty)),
            floating: vec![],
            size: (0, 0),
            state: TerminalState::Window,
            cursor: Cursor::invalid(),
            focus: 0,
            map_set: MapSet::global(),
        }
    }

    fn update_area(&mut self, size: (u16, u16)) {
        if size != self.size {
            self.size = size;
            self.windows.set_area(Area {
                x: 0,
                y: 0,
                w: self.size.0,
                h: self.size.1,
            });
        }
    }

    fn on_event(&mut self, event: Event) {
        match event {
            Event::Resize(c, r) => self.update_area((c, r)),
            Event::Key(k) => {
                if k.code == KeyCode::Char('c') && k.modifiers == KeyModifiers::CONTROL {
                    self.state = TerminalState::Exit;
                } else {
                    match self.map_set.on_key(k, self.get_state()) {
                        MapAction::Act(a) => a.run(self),
                        MapAction::Wait => (),
                        MapAction::None => self.get_focus_mut().on_key(k).run(self),
                    }
                }
            }
            Event::Mouse(m) => self.get_focus_mut().on_mouse(m).run(self),
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
        self.cursor = self.get_focus().cursor();
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

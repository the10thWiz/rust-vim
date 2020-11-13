use log::{error, info, warn};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::sync::Arc;
use terminal::{error::Result, Action, Clear, KeyCode, KeyEvent, KeyModifiers, Terminal};

mod buffer;
mod channel;
mod commands;
mod keymap;
mod window;

use buffer::Buffer;
use window::{Area, Window};

const NOT_SHIFT: KeyModifiers =
    KeyModifiers::from_bits_truncate(KeyModifiers::CONTROL.bits() | KeyModifiers::ALT.bits());

type Command = dyn Fn(&mut EditorState, Vec<String>);
fn null(_: &mut EditorState, _: Vec<String>) {}
struct CommandExecutor {
    cmd: Arc<Command>,
    args: Vec<String>,
    done: bool,
}
impl CommandExecutor {
    fn null(done: bool) -> Self {
        Self {
            cmd: Arc::new(null),
            args: vec![],
            done,
        }
    }
    fn error(msg: &str) -> Self {
        Self {
            cmd: Arc::new(|s, a| {}),
            args: vec![msg.to_string()],
            done: true,
        }
    }
    fn from_option(cmd: Option<Arc<Command>>, msg: &str, args: Vec<String>) -> Self {
        if let Some(cmd) = cmd {
            Self {
                cmd,
                args,
                done: true,
            }
        } else {
            Self::error(msg)
        }
    }
    fn execute(self, state: &mut EditorState) {
        if self.done {
            state.set_mode(Mode::Normal());
        }
        let cmd = self.cmd;
        cmd(state, self.args);
    }
}

pub struct CommandState {
    cmds: HashMap<String, BTreeMap<String, Arc<Command>>>,
    cur_parser: Option<String>,
    cur_line: (String, String),
}

impl CommandState {
    fn init() -> Self {
        let mut s = Self {
            cmds: HashMap::new(),
            cur_parser: None,
            cur_line: (String::new(), String::new()),
        };
        let mut basic_cmds: BTreeMap<String, Arc<Command>> = BTreeMap::new();
        basic_cmds.insert("q".to_string(), Arc::new(|s, a| s.set_mode(Mode::Done())));
        s.add_command_group(":".to_string(), basic_cmds);
        s
    }
    pub fn add_cmd(&mut self, leader: String, command: String, action: Arc<Command>) {
        if let Some(map) = self.cmds.get_mut(&leader) {
            map.insert(command, action);
        } else {
            let mut group = BTreeMap::new();
            group.insert(command, action);
            self.add_command_group(leader, group);
        }
    }
    pub fn add_command_group(&mut self, leader: String, commands: BTreeMap<String, Arc<Command>>) {
        self.cmds.insert(leader, commands);
    }
    pub fn activate(&mut self, parser: &str) -> bool {
        if self.cmds.get(parser).is_some() {
            self.cur_parser = Some(parser.to_string());
            self.cur_line = (String::new(), String::new());
            false
        } else {
            error!("`{}` is not a valid command type", parser);
            true
        }
    }
    pub fn is_active(&self) -> bool {
        self.cur_parser.is_some()
    }
    pub fn set_line(&mut self, s: String) {
        if !self.is_active() {
            self.cur_line = (s, String::new());
        }
    }
    pub fn draw<W: Write>(&self, terminal: &mut Terminal<W>, size: (u16, u16)) -> Result<()> {
        if let Some(l) = &self.cur_parser {
            terminal.batch(Action::MoveCursorTo(0, size.1 - 1))?;
            terminal.batch(Action::ClearTerminal(Clear::CurrentLine))?;
            write!(terminal, "{}{}{}", l, self.cur_line.0, self.cur_line.1)?;
        } else {
            terminal.batch(Action::MoveCursorTo(0, size.1 - 1))?;
            terminal.batch(Action::ClearTerminal(Clear::CurrentLine))?;
            write!(terminal, "{}", self.cur_line.0)?;
        }
        Ok(())
    }
    fn parse(&self) -> CommandExecutor {
        if let Some(l) = &self.cur_parser {
            let line = format!("{}{}", self.cur_line.0, self.cur_line.1);
            let mut line = line.split(char::is_whitespace);
            if let Some(first) = line.next() {
                CommandExecutor::from_option(
                    self.cmds
                        .get(l)
                        .expect("command type doesn't exist")
                        .get(first)
                        .cloned(),
                    first,
                    line.map(|s| s.to_string()).collect(),
                )
            } else {
                CommandExecutor::null(true)
            }
        } else {
            CommandExecutor::null(true)
        }
    }
    fn on_key(&mut self, key: KeyEvent) -> CommandExecutor {
        if !key.modifiers.intersects(NOT_SHIFT) {
            match key.code {
                KeyCode::Char(ch) => {
                    self.cur_line.0.push(ch);
                    CommandExecutor::null(false)
                }
                KeyCode::Enter => {
                    let tmp = self.parse();
                    self.cur_parser = None;
                    self.cur_line = (String::new(), String::new());
                    tmp
                }
                _ => CommandExecutor::null(false),
            }
        } else {
            CommandExecutor::null(false)
        }
    }
    fn get_pos(&self) -> u16 {
        (self.cur_parser.as_ref().map(|s| s.len()).unwrap_or(0) + self.cur_line.0.len()) as u16
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal(),
    Visual(),
    VisualLine(),
    VisualBlock(),
    Command(),
    Done(),
}

pub enum WindowSet {
    Window(Window),
    Vertical(Vec<WindowSet>, Area),
    Horizontal(Vec<WindowSet>, Area),
}
impl WindowSet {
    fn update_area(&mut self, new_area: Area) {
        match self {
            Self::Window(w) => w.resize(new_area),
            Self::Vertical(v, a) => unimplemented!(),
            Self::Horizontal(v, a) => unimplemented!(),
        }
    }
    pub fn draw<W: Write>(&self, terminal: &mut Terminal<W>) -> Result<()> {
        match self {
            Self::Window(w) => w.draw(terminal)?,
            Self::Vertical(v, a) => unimplemented!(),
            Self::Horizontal(v, a) => unimplemented!(),
        }
        Ok(())
    }
    pub fn get_cursor(&self) -> (u16, u16) {
        match self {
            Self::Window(w) => w.get_screen_cursor(),
            Self::Vertical(v, a) => unimplemented!(),
            Self::Horizontal(v, a) => unimplemented!(),
        }
    }
    pub fn get_active(&mut self) -> &mut Window {
        match self {
            Self::Window(w) => w,
            Self::Vertical(v, a) => unimplemented!(),
            Self::Horizontal(v, a) => unimplemented!(),
        }
    }
}

pub struct EditorState {
    buffers: Vec<Arc<Buffer>>,
    windows: WindowSet,
    cur_size: (u16, u16),
    status_line: String,
    command_state: CommandState,
    mode: Mode,
    last_key: KeyEvent,
    normal_map: keymap::KeyMappings,
}

impl EditorState {
    pub fn init(size: (u16, u16)) -> Self {
        let mut s = Self {
            buffers: vec![],
            windows: WindowSet::Window(Window::new(Area::new(0, 0, size.1 - 3, size.0))),
            cur_size: size,
            status_line: "".to_string(),
            command_state: CommandState::init(),
            mode: Mode::Normal(),
            last_key: KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()),
            normal_map: keymap::KeyMappings::new(),
        };
        s.normal_map.add_basic_binding(
            KeyEvent::new(KeyCode::Char(':'), KeyModifiers::empty()),
            keymap::Action::st(&|s| s.set_mode(Mode::Command())),
        );
        commands::normal_map(&mut s.normal_map);
        s
    }
    pub fn on_key(&mut self, key: KeyEvent) {
        self.last_key = key;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            self.mode = Mode::Done();
            return;
        }
        match self.mode {
            Mode::Normal() => self.normal_map.on_key(key).execute(self),
            Mode::Command() => self.command_state.on_key(key).execute(self),
            _ => (),
        }
    }
    pub fn draw<W: Write>(&self, terminal: &mut Terminal<W>) -> Result<()> {
        terminal.batch(Action::ResetColor)?;
        self.command_state.draw(terminal, self.cur_size)?;
        //terminal.batch(Action::MoveCursorTo(0, 0))?;
        //write!(terminal, "S: {:?}", self.cur_size)?;
        terminal.batch(Action::MoveCursorTo(0, self.cur_size.1 - 2))?;
        write!(terminal, "{}", self.status_line)?;
        self.windows.draw(terminal)?;

        let (c, r) = match self.mode {
            Mode::Command() => (self.command_state.get_pos(), self.cur_size.1 - 1),
            _ => self.windows.get_cursor(),
        };
        terminal.act(Action::MoveCursorTo(c, r))?;
        terminal.flush_batch()
    }
    pub fn is_done(&self) -> bool {
        self.mode == Mode::Done()
    }
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        match self.mode {
            Mode::Command() => {
                if self.command_state.activate(":") {
                    self.mode = Mode::Normal();
                }
            }
            _ => (),
        }
    }
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn set_cmd_line(&mut self, s: String) {
        self.command_state.set_line(s);
    }
    pub fn update(&mut self, size: (u16, u16)) -> bool {
        self.status_line = format!("{:?}", self.mode);
        if self.cur_size != size {
            self.windows
                .update_area(Area::new(0, 0, size.1 - 3, size.0));
            self.cur_size = size;
        }
        false
    }
    pub fn active_window(&mut self) -> &mut Window {
        self.windows.get_active()
    }
}

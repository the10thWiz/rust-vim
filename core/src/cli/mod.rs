//
// cli.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

pub(crate) mod commands;

use std::fmt::Debug;

use crossterm::{
    cursor::CursorShape,
    event::{KeyEvent, KeyModifiers},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use enum_map::Enum;

use crate::{cursor::Cursor, keymap::Action, util::Area, EventReader, Renderable};

#[derive(Debug, Enum, PartialEq, Eq, Clone, Copy)]
pub enum Cli {
    Command,
    Message,
}

impl Cli {
    pub fn character(&self) -> char {
        match self {
            Self::Command => ':',
            Self::Message => ' ',
        }
    }
}

pub enum CliAction {
    Esc,
    Execute(String),
    None,
}

impl Action for CliAction {
    fn run(&self, state: &mut crate::Vim) {
        match self {
            Self::None => (),
            Self::Esc => state.end_cli(),
            Self::Execute(line) => {
                state.end_cli();
                state.execute(line);
            },
        }
    }
}

pub struct CliState {
    cur: Cli,
    cmd: (String, String),
    area: Area,
}

impl CliState {
    pub fn new() -> Self {
        Self {
            cur: Cli::Message,
            cmd: Default::default(),
            area: Area::default(),
        }
    }

    pub fn start(&mut self, ty: Cli) {
        self.cur = ty;
        self.cmd = Default::default();
    }

    pub fn end(&mut self) {
        self.cur = Cli::Message;
    }

    pub fn get_message(&self) -> &str {
        if self.cur == Cli::Message {
            self.cmd.0.as_str()
        } else {
            ""
        }
    }

    pub fn message(&mut self, message: String) {
        self.cur = Cli::Message;
        self.cmd = (message, String::new());
    }
}

impl EventReader for CliState {
    type Act = CliAction;

    fn on_key(&mut self, key: crossterm::event::KeyEvent) -> Self::Act {
        let KeyEvent { code, modifiers } = key;
        if modifiers == KeyModifiers::empty() {
            match code {
                crossterm::event::KeyCode::Char(ch) => {
                    self.cmd.0.push(ch);
                }
                crossterm::event::KeyCode::Backspace => {
                    self.cmd.0.pop();
                }
                crossterm::event::KeyCode::Enter => {
                    self.cmd.0.push_str(self.cmd.1.as_str());
                    self.cmd.1.clear();
                    return CliAction::Execute(std::mem::take(&mut self.cmd.0));
                }
                crossterm::event::KeyCode::Left => {
                    if let Some(ch) = self.cmd.0.pop() {
                        self.cmd.1.insert(0, ch);
                    }
                }
                crossterm::event::KeyCode::Right => {
                    if !self.cmd.1.is_empty() {
                        self.cmd.0.push(self.cmd.1.remove(0));
                    }
                }
                crossterm::event::KeyCode::Up => todo!("History"),
                crossterm::event::KeyCode::Down => todo!("History"),
                crossterm::event::KeyCode::Home => {
                    self.cmd.1.insert_str(0, self.cmd.0.as_str());
                    self.cmd.0.clear();
                }
                crossterm::event::KeyCode::End => {
                    self.cmd.0.push_str(self.cmd.1.as_str());
                    self.cmd.1.clear();
                }
                crossterm::event::KeyCode::PageUp => todo!(),
                crossterm::event::KeyCode::PageDown => todo!(),
                crossterm::event::KeyCode::Tab => todo!("Completion"),
                crossterm::event::KeyCode::BackTab => todo!("Completion"),
                crossterm::event::KeyCode::Delete => {
                    if !self.cmd.1.is_empty() {
                        self.cmd.1.remove(0);
                    }
                }
                crossterm::event::KeyCode::Insert => (),
                crossterm::event::KeyCode::F(_) => (),
                crossterm::event::KeyCode::Null => (),
                crossterm::event::KeyCode::Esc => {
                    self.cur = Cli::Message;
                    return CliAction::Esc;
                }
            }
        }
        CliAction::None
    }

    fn on_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Self::Act {
        // Cli doesn't use mouse at all
        CliAction::None
    }
}

impl Renderable for CliState {
    fn set_area(&mut self, new_area: Area) {
        self.area = new_area;
    }

    fn area(&self) -> Area {
        self.area
    }

    fn cursor_pos(&self) -> Cursor {
        Cursor::from_params(
            self.area.x + 1 + self.cmd.0.len(),
            self.area.y,
            CursorShape::Line,
        )
    }

    fn draw<W: std::io::Write>(&mut self, term: &mut W) -> crossterm::Result<()> {
        self.area.pos().move_cursor(term)?;
        term.queue(Clear(ClearType::CurrentLine))?;
        write!(term, "{}{}{}", self.cur.character(), self.cmd.0, self.cmd.1)?;
        Ok(())
    }
}

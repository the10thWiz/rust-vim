//
// window.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::io::Write;
use std::sync::{Arc, RwLock};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use crossterm::Result;

use crate::buffer::{Buffer, BufferRef};
use crate::cursor::CursorShape;
use crate::util::Pos;
use crate::{cursor::Motion, Area, Cursor, EventReader, Renderable};
use crate::{Command, Lockable};

bitfield::bitfield! {
    #[derive(Clone, Copy)]
    pub struct WindowProps(u64);
    impl Debug;
    pub border, set_border: 0;
    pub gutter, set_gutter: 1;
    pub linenum, set_linenum: 2;
    pub relative, set_relative: 3;
    pub status, set_status: 4;
    pub buffer, set_buffer: 5;
}

impl WindowProps {
    fn all() -> Self {
        let mut s = Self(0);
        s.set_border(true);
        s.set_gutter(true);
        s.set_linenum(true);
        s.set_status(true);
        s.set_buffer(true);
        s
    }

    fn none() -> Self {
        Self(0)
    }
}

impl Default for WindowProps {
    fn default() -> Self {
        let mut s = Self(0);
        s.set_gutter(true);
        s.set_linenum(true);
        //s.set_status(true);
        s.set_buffer(true);
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Delete,
    Yank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinMode {
    Normal,
    Operation(Op),
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
}

pub struct Window {
    buffer: BufferRef,
    area: Area,
    buffer_row: usize,
    buffer_col: usize,
    window_props: WindowProps,
    window_updates: WindowProps,
    cursor: Cursor,
    mode: WinMode,
}

impl Window {
    pub fn new(buffer: BufferRef) -> Self {
        Self {
            area: Area::default(),
            buffer,
            buffer_row: 0,
            buffer_col: 0,
            window_props: WindowProps::default(),
            window_updates: WindowProps::all(),
            cursor: Cursor::new(Area::default()),
            mode: WinMode::Normal,
        }
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    #[inline(always)]
    fn border_width(&self) -> u16 {
        if self.window_props.border() {
            1
        } else {
            0
        }
    }

    #[inline(always)]
    fn gutter_offset(&self) -> Pos {
        Pos(self.border_width(), self.border_width()) + self.area.pos()
    }

    #[inline(always)]
    fn gutter_width(&self) -> u16 {
        if self.window_props.gutter() {
            2
        } else {
            0
        }
    }

    #[inline(always)]
    fn gutter_area(&self) -> Area {
        self.gutter_offset().area(
            self.gutter_width(),
            self.area.h - self.border_width() * 2 - self.status_height(),
        )
    }

    #[inline(always)]
    fn linenum_offset(&self) -> Pos {
        self.gutter_offset() + Pos(self.gutter_width(), 0)
    }

    #[inline(always)]
    fn linenum_width(&self) -> u16 {
        if self.window_props.linenum() {
            4
        } else {
            0
        }
    }

    #[inline(always)]
    fn linenum_area(&self) -> Area {
        self.linenum_offset().area(
            self.linenum_width(),
            self.area.h - self.border_width() * 2 - self.status_height(),
        )
    }

    #[inline(always)]
    fn status_offset(&self) -> Pos {
        Pos(
            self.gutter_width() + self.linenum_width() + self.border_width(),
            self.area.h - self.border_width() - 1,
        )
    }

    #[inline(always)]
    fn status_height(&self) -> u16 {
        if self.window_props.status() {
            1
        } else {
            0
        }
    }

    #[inline(always)]
    fn buffer_offset(&self) -> Pos {
        self.linenum_offset() + Pos(self.linenum_width(), 0)
    }

    #[inline(always)]
    fn buffer_area(&self) -> Area {
        self.buffer_offset().area(
            self.area.w - self.border_width() * 2 - self.gutter_width() - self.linenum_width(),
            self.area.h - self.border_width() * 2 - self.status_height(),
        )
    }
}

impl Renderable for Window {
    fn area(&self) -> Area {
        self.area
    }

    fn set_area(&mut self, new_area: Area) {
        self.area = new_area;
        self.cursor.apply(Motion::None, self.buffer_area());
    }

    fn draw<W: Write>(&mut self, term: &mut W) -> Result<()> {
        let buf_read = self.buffer.read();
        if self.window_updates.border() && self.window_props.border() {
            // Draw border
        }
        if self.window_updates.gutter() && self.window_props.gutter() {
            // Draw Gutter
            let area = self.gutter_area();
            for (i, line) in area.lines().enumerate() {
                line.move_cursor(term)?;
                if i + self.buffer_row < buf_read.len() {
                    write!(term, "{:width$}", "", width = area.w as usize)?;
                } else {
                    write!(term, "{:width$}", "", width = area.w as usize)?;
                }
            }
        }
        if self.window_updates.linenum() && self.window_props.linenum() {
            // Draw LineNums
            let area = self.linenum_area();
            for (i, line) in area.lines().enumerate() {
                line.move_cursor(term)?;
                if i + self.buffer_row < buf_read.len() {
                    write!(term, "{i:width$} ", width = area.w as usize - 1)?;
                } else {
                    write!(term, "{:width$}", " ~ ", width = area.w as usize)?;
                }
            }
        }
        if self.window_updates.status() && self.window_props.status() {
            // Draw status line
        }
        if self.window_updates.buffer() && self.window_props.buffer() {
            // Draw buffer
            let area = self.buffer_area();
            for (i, line) in area.lines().enumerate() {
                line.move_cursor(term)?;
                if let Some(l) = buf_read.get_line(i + self.buffer_row) {
                    l.draw(term, area.w as usize)?;
                } else {
                    write!(term, "{:width$}", "", width = area.w as usize)?;
                }
            }
        }
        self.window_updates = WindowProps::none();
        Ok(())
    }
}

pub enum WinAction {
    None,
}

impl Command for WinAction {
    fn execute<W: Lockable>(self, editor: &mut crate::Curse<W>) {
        match self {
            Self::None => (),
        }
    }
}

impl EventReader for Window {
    type Act = WinAction;
    fn on_key(&mut self, key: KeyEvent) -> Self::Act {
        let KeyEvent { code, modifiers } = key;
        let area = self.buffer_area();
        match self.mode {
            WinMode::Normal => match code {
                KeyCode::Char('i') => {
                    self.cursor.set_shape(CursorShape::Line);
                    self.mode = WinMode::Insert;
                }
                KeyCode::Char('I') => {
                    self.cursor.apply(Motion::Col(0), area);
                    self.cursor.set_shape(CursorShape::Line);
                    self.mode = WinMode::Insert;
                }
                KeyCode::Char('a') => {
                    self.cursor.set_shape(CursorShape::Line);
                    self.cursor.apply(Motion::Relative(1, 0), area);
                    self.mode = WinMode::Insert;
                }
                KeyCode::Char('A') => {
                    self.cursor.set_shape(CursorShape::Line);
                    self.cursor.apply(Motion::Col(u16::MAX), area);
                    self.mode = WinMode::Insert;
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    self.cursor.apply(Motion::Relative(-1, 0), area);
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    self.cursor.apply(Motion::Relative(1, 0), area);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.cursor.apply(Motion::Relative(0, 1), area);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.cursor.apply(Motion::Relative(0, -1), area);
                }
                KeyCode::Char('$') | KeyCode::End => {
                    self.cursor.apply(Motion::Col(area.w), area);
                }
                KeyCode::Char('0') => {
                    self.cursor.apply(Motion::Col(0), area);
                }
                KeyCode::Char('^') | KeyCode::Home => {
                    self.cursor.apply(
                        Motion::Col(
                            self.buffer
                                .read()
                                .get_line(self.cursor.row(area))
                                .unwrap()
                                .first_char() as u16,
                        ),
                        area,
                    );
                }
                KeyCode::Char('d') => {
                    self.mode = WinMode::Operation(Op::Delete);
                }
                KeyCode::Char('y') => {
                    self.mode = WinMode::Operation(Op::Yank);
                }
                _ => todo!("Key event: {key:?}"),
            },
            WinMode::Insert => {
                if modifiers == KeyModifiers::NONE {
                    match code {
                        KeyCode::Char(c) => {
                            self.buffer.write().insert_char(
                                self.cursor.row(area),
                                self.cursor.col(area),
                                c,
                            );
                            self.cursor.apply(Motion::Relative(1, 0), area);
                            self.window_updates.set_buffer(true);
                        }
                        KeyCode::Left => {
                            self.cursor.apply(Motion::Relative(-1, 0), area);
                        }
                        KeyCode::Right => {
                            self.cursor.apply(Motion::Relative(1, 0), area);
                        }
                        KeyCode::Down => {
                            self.cursor.apply(Motion::Relative(0, 1), area);
                        }
                        KeyCode::Up => {
                            self.cursor.apply(Motion::Relative(0, -1), area);
                        }
                        KeyCode::Backspace => {
                            if self.cursor.col(area) > 0 {
                                self.cursor.apply(Motion::Relative(-1, 0), area);
                                self.buffer
                                    .write()
                                    .remove_char(self.cursor.row(area), self.cursor.col(area));
                                self.window_updates.set_buffer(true);
                            } else if self.cursor.row(area) > 0 {
                                let mut buffer = self.buffer.write();
                                self.cursor.apply(
                                    Motion::Relative(
                                        buffer.get_line(self.cursor.row(area) - 1).unwrap().len()
                                            as i16,
                                        -1,
                                    ),
                                    area,
                                );
                                buffer.join_line(self.cursor.row(area));
                                self.window_updates.set_buffer(true);
                                self.window_updates.set_linenum(true);
                                self.window_updates.set_gutter(true);
                            }
                        }
                        KeyCode::Enter => {
                            self.buffer
                                .write()
                                .split_line(self.cursor.row(area), self.cursor.col(area));
                            self.cursor.apply(Motion::Relative(0, 1), area);
                            self.cursor.apply(Motion::Col(0), area);
                            self.window_updates.set_buffer(true);
                            self.window_updates.set_linenum(true);
                            self.window_updates.set_gutter(true);
                        }
                        KeyCode::Esc => {
                            self.cursor.set_shape(CursorShape::Block);
                            self.mode = WinMode::Normal;
                        }
                        KeyCode::End => {
                            self.cursor.apply(Motion::Col(area.w), area);
                        }
                        KeyCode::Home => {
                            self.cursor.apply(
                                Motion::Col(
                                    self.buffer
                                        .read()
                                        .get_line(self.cursor.row(area))
                                        .unwrap()
                                        .first_char() as u16,
                                ),
                                area,
                            );
                        }
                        _ => todo!("Insert Key Event: {code:?}"),
                    }
                }
            }
            _ => todo!("{:?}", self.mode),
        }
        let buffer = self.buffer.read();
        if self.cursor.row(area) >= buffer.len() {
            self.cursor
                .apply(Motion::Row(buffer.len().saturating_sub(1) as u16), area);
        }
        let len = if self.mode == WinMode::Insert {
            buffer.get_line(self.cursor.row(area)).unwrap().len()
        } else {
            buffer
                .get_line(self.cursor.row(area))
                .unwrap()
                .len()
                .saturating_sub(1)
        };
        if self.cursor.col(area) > len {
            self.cursor.apply(Motion::Col(len as u16), area);
        }
        WinAction::None
    }

    fn on_mouse(&mut self, mouse: MouseEvent) -> Self::Act {
        todo!("Mouse event: {mouse:?}")
    }
}

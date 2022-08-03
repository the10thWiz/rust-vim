//
// window.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use crossterm::Result;
use log::info;

use crate::buffer::BufferRef;
use crate::cursor::CursorShape;
use crate::keymap::{Action, State};
use crate::util::Pos;
use crate::Vim;
use crate::{cursor::Motion, Area, Cursor, EventReader, Renderable};

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
    Replace,
    Visual,
    VisualLine,
    VisualBlock,
}

impl WinMode {
    pub fn get_shape(&self) -> CursorShape {
        match self {
            Self::Normal => CursorShape::Block,
            Self::Operation(_) => CursorShape::Block,
            Self::Insert => CursorShape::Line,
            Self::Replace => CursorShape::UnderScore,
            Self::Visual => CursorShape::Block,
            Self::VisualLine => CursorShape::Block,
            Self::VisualBlock => CursorShape::Block,
        }
    }

    pub fn get_message(&self) -> &'static str {
        match self {
            Self::Normal => "",
            Self::Operation(_) => "",
            Self::Insert => "-- INSERT --",
            Self::Replace => "-- REPLACE --",
            Self::Visual => "-- VISUAL --",
            Self::VisualLine => "-- VISUAL LINE --",
            Self::VisualBlock => "-- VISUAL BLOCK --",
        }
    }

    pub fn insert(&self) -> bool {
        match self {
            Self::Insert | Self::Replace => true,
            _ => false,
        }
    }
}

pub enum Scroll {
    Down,
    Up,
    Left,
    Right,
}

pub enum Dist {
    One,
    HalfScreen,
    Screen,
}

pub struct VisibileArea {
    screen_pos: Area,
    buffer_row: usize,
    buffer_col: usize,
}

pub struct Window {
    buffer: BufferRef,
    buffer_view: VisibileArea,
    window_props: WindowProps,
    window_updates: WindowProps,
    cursor: Cursor,
    mode: WinMode,
}

impl Window {
    pub fn new(buffer: BufferRef) -> Self {
        Self {
            buffer,
            buffer_view: VisibileArea {
                screen_pos: Area::default(),
                buffer_row: 0,
                buffer_col: 0,
            },
            window_props: WindowProps::default(),
            window_updates: WindowProps::all(),
            cursor: Cursor::new(),
            mode: WinMode::Normal,
        }
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    pub fn cursor_apply(&mut self, motion: Motion) -> &mut Self {
        self.cursor
            .apply(motion, &self.buffer.read(), self.mode == WinMode::Insert);
        if self.cursor.row() < self.buffer_view.buffer_row {
            self.buffer_view.buffer_row = self.cursor.row();
            self.on_scroll();
        } else if self.cursor.row()
            >= self.buffer_view.buffer_row + self.buffer_view.screen_pos.height()
        {
            self.buffer_view.buffer_row =
                self.cursor.row() - self.buffer_view.screen_pos.height() + 1;
            self.on_scroll();
        }
        if self.cursor.col() < self.buffer_view.buffer_col {
            self.buffer_view.buffer_col = self.cursor.col();
            self.on_scroll();
        } else if self.cursor.col()
            >= self.buffer_view.buffer_col + self.buffer_view.screen_pos.width()
        {
            self.buffer_view.buffer_col =
                self.cursor.col() - self.buffer_view.screen_pos.width() + 1;
            self.on_scroll();
        }
        self
    }

    fn on_scroll(&mut self) {
        self.window_updates.set_gutter(true);
        self.window_updates.set_buffer(true);
        self.window_updates.set_linenum(true);
    }

    pub fn scroll(&mut self, scroll: Scroll, dist: Dist) {
        match scroll {
            Scroll::Down => {
                self.buffer_view.buffer_row = self
                    .buffer_view
                    .buffer_row
                    .saturating_add(self.row_dist(dist))
                    .min(self.buffer.read().len().saturating_sub(1))
            }
            Scroll::Up => {
                self.buffer_view.buffer_row = self
                    .buffer_view
                    .buffer_row
                    .saturating_sub(self.row_dist(dist));
            }
            Scroll::Right => {
                self.buffer_view.buffer_col = self
                    .buffer_view
                    .buffer_col
                    .saturating_add(self.col_dist(dist))
                    .min(self.buffer.read()[self.cursor.row()].len() - 1)
            }
            Scroll::Left => {
                self.buffer_view.buffer_col = self
                    .buffer_view
                    .buffer_col
                    .saturating_sub(self.col_dist(dist));
            }
        }
        self.on_scroll();
    }

    fn row_dist(&self, dist: Dist) -> usize {
        match dist {
            Dist::One => 1,
            Dist::HalfScreen => self.buffer_area().height() / 2,
            Dist::Screen => self.buffer_area().height(),
        }
    }

    fn col_dist(&self, dist: Dist) -> usize {
        match dist {
            Dist::One => 1,
            Dist::HalfScreen => self.buffer_area().width() / 2,
            Dist::Screen => self.buffer_area().width(),
        }
    }

    pub fn mode(&self) -> WinMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: WinMode) -> &mut Self {
        self.cursor.set_shape(mode.get_shape());
        if self.mode == WinMode::Insert {
            self.cursor_apply(Motion::Left);
        }
        self.mode = mode;
        self
    }

    #[inline(always)]
    fn border_width(&self) -> usize {
        if self.window_props.border() {
            1
        } else {
            0
        }
    }

    #[inline(always)]
    fn gutter_offset(&self) -> Pos {
        Pos(self.border_width(), self.border_width()) + self.area().pos()
    }

    #[inline(always)]
    fn gutter_width(&self) -> usize {
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
            self.area().h - self.border_width() * 2 - self.status_height(),
        )
    }

    #[inline(always)]
    fn linenum_offset(&self) -> Pos {
        self.gutter_offset() + Pos(self.gutter_width(), 0)
    }

    #[inline(always)]
    fn linenum_width(&self) -> usize {
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
            self.area().h - self.border_width() * 2 - self.status_height(),
        )
    }

    #[inline(always)]
    fn status_offset(&self) -> Pos {
        Pos(
            self.gutter_width() + self.linenum_width() + self.border_width(),
            self.area().h - self.border_width() - 1,
        )
    }

    #[inline(always)]
    fn status_height(&self) -> usize {
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
    pub fn buffer_area(&self) -> Area {
        self.buffer_offset().area(
            self.area().w - self.border_width() * 2 - self.gutter_width() - self.linenum_width(),
            self.area().h - self.border_width() * 2 - self.status_height(),
        )
    }

    pub fn get_state(&self) -> State {
        match self.mode {
            WinMode::Normal => State::Normal,
            WinMode::Operation(_) => State::Operator,
            WinMode::Insert | WinMode::Replace => State::Insert,
            WinMode::Visual | WinMode::VisualLine | WinMode::VisualBlock => State::Visual,
        }
    }

    pub fn buffer(&self) -> &BufferRef {
        &self.buffer
    }
}

impl Renderable for Window {
    fn area(&self) -> Area {
        self.buffer_view.screen_pos
    }

    fn set_area(&mut self, new_area: Area) {
        self.buffer_view.screen_pos = new_area;
        //self.cursor.apply(Motion::None, self.buffer_area());
    }

    fn cursor_pos(&self) -> Cursor {
        Cursor::from_params(
            self.cursor().col() - self.buffer_view.buffer_col + self.buffer_area().x,
            self.cursor().row() - self.buffer_view.buffer_row + self.buffer_area().y,
            self.cursor().shape(),
        )
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
                if i + self.buffer_view.buffer_row < buf_read.len() {
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
                let row = i + self.buffer_view.buffer_row;
                if row < buf_read.len() {
                    write!(term, "{row:width$} ", width = area.w as usize - 1)?;
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
                if let Some(l) = buf_read.get_line(i + self.buffer_view.buffer_row) {
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
    SetMessage(&'static str),
}

impl Action for WinAction {
    fn run(&self, editor: &mut Vim) {
        match self {
            Self::None => (),
            Self::SetMessage(m) => editor.message(m.to_string()),
        }
    }
}

impl EventReader for Window {
    type Act = WinAction;
    fn on_key(&mut self, key: KeyEvent) -> Self::Act {
        let KeyEvent { code, modifiers } = key;
        let area = self.buffer_area();
        match code {
            KeyCode::Char(c) => {
                if self.mode.insert() && modifiers & !KeyModifiers::SHIFT == KeyModifiers::NONE {
                    if self.mode == WinMode::Insert {
                        self.buffer
                            .write()
                            .insert_char(self.cursor.row(), self.cursor.col(), c);
                    } else if self.mode == WinMode::Replace {
                        self.buffer
                            .write()
                            .replace_char(self.cursor.row(), self.cursor.col(), c);
                    }
                    self.cursor_apply(Motion::Right);
                    self.window_updates.set_buffer(true);
                }
            }
            KeyCode::Backspace => {
                if self.mode.insert() {
                    if self.cursor.col() > 0 {
                        self.cursor_apply(Motion::Left);
                        self.buffer
                            .write()
                            .remove_char(self.cursor.row(), self.cursor.col());
                        self.window_updates.set_buffer(true);
                    } else if self.cursor.row() > 0 {
                        self.cursor_apply(Motion::Up);
                        self.cursor_apply(Motion::End);
                        self.buffer().write().join_line(self.cursor.row());
                        self.window_updates.set_buffer(true);
                        self.window_updates.set_linenum(true);
                        self.window_updates.set_gutter(true);
                    }
                } else if self.cursor.col() == 0 {
                    self.cursor_apply(Motion::Up);
                    self.cursor_apply(Motion::End);
                } else {
                    self.cursor_apply(Motion::Left);
                }
            }
            KeyCode::Delete => {
                if self.cursor.col() < self.buffer.read()[self.cursor.row()].len() {
                    self.buffer
                        .write()
                        .remove_char(self.cursor.row(), self.cursor.col());
                    self.window_updates.set_buffer(true);
                } else if self.cursor.row() + 1 < self.buffer.read().len() {
                    self.buffer().write().join_line(self.cursor.row());
                    self.window_updates.set_buffer(true);
                    self.window_updates.set_linenum(true);
                    self.window_updates.set_gutter(true);
                }
            }
            KeyCode::Enter => {
                if self.mode.insert() {
                    self.buffer
                        .write()
                        .split_line(self.cursor.row(), self.cursor.col());
                    self.cursor_apply(Motion::Down);
                    self.cursor_apply(Motion::SetCol(0));
                    self.window_updates.set_buffer(true);
                    self.window_updates.set_linenum(true);
                    self.window_updates.set_gutter(true);
                } else {
                    self.cursor_apply(Motion::Down);
                }
            }
            KeyCode::Esc => {
                self.set_mode(WinMode::Normal);
                return WinAction::SetMessage("");
            }
            KeyCode::End => {
                self.cursor_apply(Motion::SetCol(area.w));
            }
            KeyCode::Home => {
                self.cursor_apply(Motion::Start);
            }
            KeyCode::Insert => {
                if self.mode == WinMode::Insert {
                    self.set_mode(WinMode::Replace);
                } else if self.mode == WinMode::Replace {
                    self.set_mode(WinMode::Insert);
                } else {
                    self.set_mode(WinMode::Insert);
                }
            }
            _ => (),
        }
        WinAction::None
    }

    fn on_mouse(&mut self, mouse: MouseEvent) -> Self::Act {
        info!("Mouse event: {mouse:?}");
        WinAction::None
    }
}

//
// cursor.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use crate::{buffer::BufferRead, Area, Result, util::Pos};
use crossterm::{
    cursor::{MoveTo, SetCursorShape},
    QueueableCommand,
};
use std::io::Write;

pub use crossterm::cursor::CursorShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    SetCol(usize),
    SetRow(usize),
    Up,
    Down,
    Left,
    Right,
    End,
    Start,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    x: usize,
    y: usize,
    ty: CursorShape,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            ty: CursorShape::Block,
        }
    }

    pub fn from_params(x: usize, y: usize, ty: CursorShape) -> Self {
        Self { x, y, ty }
    }

    pub(crate) fn invalid() -> Self {
        Self {
            x: !0,
            y: !0,
            ty: CursorShape::Block,
        }
    }

    pub fn row(&self) -> usize {
        self.y
    }

    pub fn col(&self) -> usize {
        self.x
    }

    pub fn apply(&mut self, motion: Motion, buffer: &BufferRead, insert: bool) {
        match motion {
            Motion::SetRow(r) => self.y = r.min(buffer.len() - 1),
            Motion::SetCol(c) => {
                self.x = c.min(
                    buffer[self.y]
                        .len()
                        .saturating_sub(if insert { 0 } else { 1 }),
                )
            }
            Motion::Up => self.y = self.y.saturating_sub(1),
            Motion::Down => self.y = self.y.saturating_add(1).min(buffer.len() - 1),
            Motion::Left => self.x = buffer[self.y].prev(self.x),
            Motion::Right => self.x = buffer[self.y].next(self.x, insert),
            Motion::End => self.x = buffer[self.y].len(),
            Motion::Start => self.x = buffer[self.y].first_char(),
        }
    }

    pub fn shape(&self) -> CursorShape {
        self.ty
    }

    pub fn set_shape(&mut self, shape: CursorShape) {
        self.ty = shape;
    }

    pub fn draw<W: Write>(&self, mut term: W) -> Result<()> {
        term.queue(MoveTo(self.x as u16, self.y as u16))?;
        term.queue(SetCursorShape(self.ty))?;
        Ok(())
    }

    pub fn pos(&self) -> Pos {
        Pos(self.x, self.y)
    }
}

//
// cursor.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::io::Write;
use crossterm::{QueueableCommand, cursor::{MoveTo, SetCursorShape}};
use crate::{Area, Result};

pub use crossterm::cursor::CursorShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Relative(i16, i16),
    Absolute(u16, u16),
    Col(u16),
    Row(u16),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pos: (u16, u16),
    ty: CursorShape,
}

impl Cursor {
    pub fn new(area: Area) -> Self {
        Self {
            pos: (area.x, area.y),
            ty: CursorShape::Block,
        }
    }

    pub(crate) fn invalid() -> Self {
        Self {
            pos: (!0, !0),
            ty: CursorShape::Block,
        }
    }

    pub fn row(&self, area: Area) -> usize {
        (self.pos.1 - area.y) as usize
    }

    pub fn col(&self, area: Area) -> usize {
        (self.pos.0 - area.x) as usize
    }

    pub fn apply(&mut self, motion: Motion, area: Area) {
        let (c, r) = match motion {
            Motion::Relative(c, r) => (self.pos.0 as i16 + c, self.pos.1 as i16 + r),
            Motion::Absolute(c, r) => (c as i16, r as i16),
            Motion::Row(r) => (self.pos.0 as i16, (r + area.y) as i16),
            Motion::Col(c) => ((c + area.x) as i16, self.pos.1 as i16),
            Motion::None => (self.pos.0 as i16, self.pos.1 as i16),
        };
        if r < area.y as i16 {
            self.pos.1 = area.y;
        } else if r >= area.y as i16 + area.h as i16 {
            self.pos.1 = area.y + area.h - 1;
        } else {
            self.pos.1 = r as u16;
        }
        if c < area.x as i16 {
            self.pos.0 = area.x;
        } else if c >= area.x as i16 + area.w as i16 {
            self.pos.0 = area.x + area.w - 1;
        } else {
            self.pos.0 = c as u16;
        }
    }

    pub fn set_shape(&mut self, shape: CursorShape) {
        self.ty = shape;
    }

    pub fn draw<W: Write>(&self, mut term: W) -> Result<()> {
        term.queue(MoveTo(self.pos.0, self.pos.1))?;
        term.queue(SetCursorShape(self.ty))?;
        Ok(())
    }
}



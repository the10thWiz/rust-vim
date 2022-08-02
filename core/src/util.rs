//
// util.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::{io::Write, ops::{Add, AddAssign}};

use crossterm::{QueueableCommand, cursor::MoveTo};

use crate::Result;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Area {
    pub fn pos(&self) -> Pos {
        Pos(self.x, self.y)
    }

    pub fn height(&self) -> u16 {
        self.h
    }

    pub fn width(&self) -> u16 {
        self.w
    }

    pub fn lines<'s>(&'s self) -> LineIter<'s> {
        LineIter {
            area: self,
            line: self.y,
        }
    }
}

pub struct LineIter<'s> {
    area: &'s Area,
    line: u16,
}

impl<'s> Iterator for LineIter<'s> {
    type Item = Pos;
    fn next(&mut self) -> Option<Self::Item> {
        if self.line < self.area.y + self.area.h {
            let ret = self.line;
            self.line += 1;
            Some(Pos(self.area.x, ret))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos(pub u16, pub u16);

impl Pos {
    pub fn area(&self, w: u16, h: u16) -> Area {
        Area {
            x: self.0,
            y: self.1,
            w, h,
        }
    }

    pub fn in_area(&self, area: Area) -> Self {
        Self(self.0 + area.x, self.1 + area.y)
    }

    pub fn move_cursor<W: Write>(&self, term: &mut W) -> Result<()> {
        term.queue(MoveTo(self.0, self.1))?;
        Ok(())
    }
}

impl Add for Pos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign for Pos {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}


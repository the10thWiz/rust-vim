//
// util.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::{io::Write, ops::{Add, AddAssign}, fmt::Display};

use crossterm::{QueueableCommand, cursor::MoveTo, event::{KeyEvent, KeyCode, KeyModifiers}};

use crate::Result;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Area {
    pub fn pos(&self) -> Pos {
        Pos(self.x, self.y)
    }

    pub fn height(&self) -> usize {
        self.h
    }

    pub fn width(&self) -> usize {
        self.w
    }

    pub fn top(&self, h: usize) -> Self {
        Self {
            x: self.x,
            y: self.y,
            w: self.w,
            h,
        }
    }

    pub fn bottom(&self, h: usize) -> Self {
        Self {
            x: self.x,
            y: self.y + self.h - h,
            w: self.w,
            h,
        }
    }

    pub fn left(&self, w: usize) -> Self {
        Self {
            x: self.x,
            y: self.y,
            w,
            h: self.h,
        }
    }

    pub fn right(&self, w: usize) -> Self {
        Self {
            x: self.x + self.w - w,
            y: self.y,
            w,
            h: self.h,
        }
    }

    pub fn lines<'s>(&'s self) -> LineIter<'s> {
        LineIter {
            area: self,
            line: self.y,
        }
    }

    const SPACES: [u8; 100] = [b' '; 100];
    pub fn clear<W: Write>(&self, term: &mut W) -> Result<()> {
        for l in self.lines() {
            l.move_cursor(term)?;
            Self::clear_line(term, self.w)?;
        }
        Ok(())
    }

    pub fn clear_line<W: Write>(term: &mut W, mut width: usize) -> Result<()> {
        while width > 0 {
            width -= term.write(&Self::SPACES[0..width.min(Self::SPACES.len())])?;
        }
        Ok(())
    }
}

pub struct LineIter<'s> {
    area: &'s Area,
    line: usize,
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
pub struct Pos(pub usize, pub usize);

impl Pos {
    pub fn area(&self, w: usize, h: usize) -> Area {
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
        term.queue(MoveTo(self.0 as u16, self.1 as u16))?;
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

pub struct KeyDisplay(pub KeyEvent);

impl KeyDisplay {
    fn fmt_code(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.code {
            KeyCode::Backspace => write!(f, "<Bs>"),
            KeyCode::Enter => write!(f, "<Ret>"),
            KeyCode::Left => write!(f, "<Left>"),
            KeyCode::Right => write!(f, "<Right>"),
            KeyCode::Up => write!(f, "<Up>"),
            KeyCode::Down => write!(f, "<Down>"),
            KeyCode::Home => write!(f, "<Home>"),
            KeyCode::End => write!(f, "<End>"),
            KeyCode::PageUp => write!(f, "<PgUp>"),
            KeyCode::PageDown => write!(f, "<PgDn>"),
            KeyCode::Tab => write!(f, "<Tab>"),
            KeyCode::BackTab => write!(f, "<S-Tab>"),
            KeyCode::Delete => write!(f, "<Del>"),
            KeyCode::Insert => write!(f, "<Ins>"),
            KeyCode::F(n) => write!(f, "<F{}>", n),
            KeyCode::Char(c) => write!(f, "{}", c),
            KeyCode::Null => write!(f, "<0>"),
            KeyCode::Esc => write!(f, "<Esc>"),
        }
    }
}

impl Display for KeyDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = self.0.modifiers;
        if m.contains(KeyModifiers::CONTROL) && m.contains(KeyModifiers::ALT) {
            write!(f, "<M-^")?;
            self.fmt_code(f)?;
            write!(f, ">")
        } else if m.contains(KeyModifiers::CONTROL) {
            write!(f, "^")?;
            self.fmt_code(f)
        } else if m.contains(KeyModifiers::ALT) {
            write!(f, "<M-")?;
            self.fmt_code(f)?;
            write!(f, ">")
        } else {
            self.fmt_code(f)
        }
    }
}


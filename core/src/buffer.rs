//
// buffer.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::{
    fs::File,
    io::{BufRead, BufReader, Write, self},
    ops::{Deref, DerefMut, Index, IndexMut},
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crossterm::style::ContentStyle;

use crate::{util::Area, Result};

pub struct Line {
    text: String,
    style: Vec<(usize, ContentStyle)>,
}

impl Line {
    fn empty() -> Self {
        Self {
            text: String::new(),
            style: vec![(0, ContentStyle::default())],
        }
    }

    fn new(text: String) -> Self {
        Self {
            style: vec![(text.len(), ContentStyle::default())],
            text,
        }
    }

    pub fn draw<W: Write>(&self, term: &mut W, width: usize) -> Result<()> {
        write!(term, "{:width$}", self.text)?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn first_char(&self) -> usize {
        self.text
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(self.text.len())
    }

    fn update(&mut self) {
        self.style.last_mut().unwrap().0 = self.text.len();
    }

    pub fn prev(&self, pos: usize) -> usize {
        self.text.floor_char_boundary(pos.saturating_sub(1))
    }
    pub fn next(&self, pos: usize, past_end: bool) -> usize {
        if pos.saturating_add(1) >= self.text.len() {
            if past_end {
                self.text.len()
            } else {
                self.text.floor_char_boundary(self.text.len() - 1)
            }
        } else {
            self.text.ceil_char_boundary(pos.saturating_add(1))
        }
    }
}

pub struct Buffer {
    data: Vec<Line>,
    filename: Option<PathBuf>,
}

impl Buffer {
    pub fn empty() -> Self {
        Self {
            data: vec![Line::empty()],
            filename: None,
        }
    }

    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        Ok(Self {
            data: BufReader::new(File::open(&path)?)
                .lines()
                .map(|l| Ok(Line::new(l?)))
                .collect::<Result<Vec<Line>>>()?,
            filename: Some(path),
        })
    }

    pub fn write_file(&mut self) -> Result<()> {
        let mut file = File::create(self.filename.as_ref().ok_or(io::Error::new(io::ErrorKind::NotFound, "File not found"))?)?;
        for line in self.data.iter() {
            file.write_all(line.text.as_bytes())?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    pub fn get_line(&self, line: usize) -> Option<&Line> {
        self.data.get(line)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn append_line(&mut self, text: String) {
        self.data.push(Line::new(text));
    }

    pub fn insert_line(&mut self, line: usize, text: String) {
        self.data.insert(line, Line::new(text));
    }

    pub fn insert_char(&mut self, line: usize, col: usize, ch: char) {
        self.data[line].text.insert(col, ch);
        self.data[line].update();
    }

    pub fn replace_char(&mut self, line: usize, col: usize, ch: char) {
        let line = &mut self.data[line];
        if col < line.text.len() {
            line.text.remove(col);
        }
        line.text.insert(col, ch);
        line.update();
    }

    pub fn remove_char(&mut self, line: usize, col: usize) {
        self.data[line].text.remove(col);
        self.data[line].update();
    }

    pub fn split_line(&mut self, line: usize, col: usize) {
        let text = self.data[line].text.split_off(col);
        self.data.insert(line + 1, Line::new(text));
        self.data[line].update();
    }

    pub fn join_line(&mut self, line: usize) {
        let next = self.data.remove(line + 1);
        self.data[line].text += next.text.as_str();
        self.data[line].update();
    }
}

impl Index<usize> for Buffer {
    type Output = Line;

    fn index(&self, line: usize) -> &Self::Output {
        &self.data[line]
    }
}

impl IndexMut<usize> for Buffer {
    fn index_mut(&mut self, line: usize) -> &mut Self::Output {
        &mut self.data[line]
    }
}

pub struct BufferRef {
    inner: Arc<RwLock<Buffer>>,
}

impl BufferRef {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Buffer::empty())),
        }
    }

    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self> {
        Buffer::from_file(path).map(|b| Self {
            inner: Arc::new(RwLock::new(b)),
        })
    }

    pub fn read<'s>(&'s self) -> BufferRead<'s> {
        BufferRead {
            inner: self.inner.read().unwrap(),
        }
    }

    pub fn write<'s>(&'s self) -> BufferWrite<'s> {
        BufferWrite {
            inner: self.inner.write().unwrap(),
        }
    }
}

impl Clone for BufferRef {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub struct BufferRead<'s> {
    inner: RwLockReadGuard<'s, Buffer>,
}

impl<'s> Deref for BufferRead<'s> {
    type Target = Buffer;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

pub struct BufferWrite<'s> {
    inner: RwLockWriteGuard<'s, Buffer>,
}

impl<'s> Deref for BufferWrite<'s> {
    type Target = Buffer;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'s> DerefMut for BufferWrite<'s> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

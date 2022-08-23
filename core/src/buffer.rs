//
// buffer.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::{
    fmt::Display,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    ops::{Deref, DerefMut, Index, IndexMut},
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crossterm::style::ContentStyle;
use vimscript::{IdProcuder, Id};

use crate::{Result, options::{BufOptions, Opts}};

pub trait BufferSelect {
    fn select(&self, buffer: &Buffer) -> bool;
}

#[derive(Debug, Default)]
pub struct Signs {
    lst: Vec<(char, ContentStyle, isize)>,
}

impl Display for Signs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (view, style, _) in self.lst.iter().take(2) {
            write!(f, "{}", style.apply(view))?;
        }
        Ok(())
    }
}

pub struct Line {
    text: String,
    style: Vec<(usize, ContentStyle)>,
    signs: Signs,
}

impl Line {
    fn empty() -> Self {
        Self::new(String::new())
    }

    fn new(text: String) -> Self {
        Self {
            style: vec![(text.len(), ContentStyle::default())],
            text,
            signs: Signs::default(),
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

    pub fn signs(&self) -> &Signs {
        &self.signs
    }
}

pub struct Buffer {
    data: Vec<Line>,
    filename: Option<PathBuf>,
    options: BufOptions,
}

impl Buffer {
    pub fn empty() -> Self {
        Self {
            data: vec![Line::empty()],
            filename: None,
            options: BufOptions::new(),
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
            options: BufOptions::new(),
        })
    }

    pub fn options(&self) -> &BufOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut BufOptions {
        &mut self.options
    }

    pub fn write_file(&mut self) -> Result<()> {
        let mut file = File::create(
            self.filename
                .as_ref()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File not found"))?,
        )?;
        for line in self.data.iter() {
            file.write_all(line.text.as_bytes())?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    pub fn title(&self) -> &str {
        match &self.filename {
            Some(path) => path
                .iter()
                .last()
                .map_or("/", |o| o.to_str().unwrap_or("[INVALID PATH]")),
            None => "[Scratch Buffer]",
        }
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
    id: Id,
    inner: Arc<RwLock<Buffer>>,
}

impl BufferRef {
    pub fn empty(id: &mut IdProcuder) -> Self {
        Self {
            id: id.get(),
            inner: Arc::new(RwLock::new(Buffer::empty())),
        }
    }

    pub fn from_file(id: &mut IdProcuder, path: impl Into<PathBuf>) -> Result<Self> {
        Buffer::from_file(path).map(|b| Self {
            id: id.get(),
            inner: Arc::new(RwLock::new(b)),
        })
    }

    pub fn read(&self) -> BufferRead<'_> {
        BufferRead {
            inner: self.inner.read().unwrap(),
        }
    }

    pub fn write(&self) -> BufferWrite<'_> {
        BufferWrite {
            inner: self.inner.write().unwrap(),
        }
    }

    pub fn with_read<T>(&self, f: impl FnOnce(&BufferRead<'_>) -> T) -> T {
        let lock = self.read();
        let ret = f(&lock);
        drop(lock);
        ret
    }

    pub fn with_write<T>(&self, f: impl FnOnce(&mut BufferWrite<'_>) -> T) -> T {
        let mut lock = self.write();
        let ret = f(&mut lock);
        drop(lock);
        ret
    }

    pub fn id(&self) -> Id {
        self.id
    }
}

impl Clone for BufferRef {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
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

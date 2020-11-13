use crate::buffer::Buffer;
use std::io::Write;
use std::sync::Arc;
use terminal::{error::Result, Action, Color, Terminal};

pub struct Area {
    r_min: u16,
    r_max: u16,
    c_min: u16,
    c_max: u16,
}
impl Area {
    pub fn new(r_min: u16, c_min: u16, r_max: u16, c_max: u16) -> Self {
        Self {
            r_min,
            r_max,
            c_min,
            c_max,
        }
    }
}

/// Always (Col, Row)
pub enum Motion {
    Relative(i16, i16),
    WindowPos(i16, i16),
    FilePos(i16, i16),
}

/// Window holds the window, and
/// associated information, such as buffers and
/// metadata
pub struct Window {
    area: Area,
    cur_buffer: Option<Arc<Buffer>>,
    window_range: (u16, u16),
    cursor: (u16, u16),
    gutter_width: u16,
    selection: Option<(u16, u16)>,
}

impl Window {
    pub fn new(area: Area) -> Self {
        Self {
            area,
            cur_buffer: None,
            window_range: (0, 0),
            cursor: (0, 0),
            gutter_width: 3,
            selection: None,
        }
    }
    pub fn draw<W: Write>(&self, terminal: &mut Terminal<W>) -> Result<()> {
        terminal.batch(Action::SetBackgroundColor(Color::Red))?;

        terminal.batch(Action::MoveCursorTo(
            self.area.c_min + 1 + self.gutter_width,
            1,
        ))?;
        write!(terminal, "c: {:?}", self.cursor)?;
        for i in self.area.r_min..self.area.r_max {
            terminal.batch(Action::MoveCursorTo(self.area.c_min, i))?;
            write!(terminal, "{: >2} ", i)?;
        }
        Ok(())
    }
    pub fn resize(&mut self, area: Area) {
        self.area = area;
    }
    pub fn get_cursor(&self) -> (u16, u16) {
        (self.cursor.0, self.cursor.1 - self.window_range.0)
    }
    pub fn get_screen_cursor(&self) -> (u16, u16) {
        (self.cursor.0 + self.gutter_width, self.cursor.1)
    }
    fn set_cursor(&mut self, mut c: i16, mut r: i16) {
        if c > (self.area.c_max - self.gutter_width - self.area.c_min - 1) as i16 {
            c = (self.area.c_max - self.area.c_min - 1) as i16;
        } else if c < 0 {
            c = 0;
        }
        if r > (self.area.r_max - self.area.r_min - 1) as i16 {
            r = (self.area.r_max - self.area.r_min - 1) as i16;
        } else if r < 0 {
            r = 0;
        }
        self.cursor = (c as u16, r as u16);
    }
    pub fn move_cursor(&mut self, motion: Motion) {
        match motion {
            Motion::Relative(c, r) => {
                self.set_cursor(self.cursor.0 as i16 + c, self.cursor.1 as i16 + r)
            }
            Motion::FilePos(c, r) => unimplemented!(),
            Motion::WindowPos(c, r) => unimplemented!(),
        }
    }
    pub fn visual_range(&self) -> (u16, u16) {
        self.window_range
    }
}

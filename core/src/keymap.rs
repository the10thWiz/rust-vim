//
// keymap.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crossterm::{
    cursor::CursorShape,
    event::{KeyCode, KeyEvent, KeyModifiers},
};
use enum_map::{Enum, EnumMap};

use crate::{
    cursor::Motion,
    window::{Op, WinMode, Window},
    Vim,
};

pub trait Action {
    fn run(&self, state: &mut Vim);
}

impl<F> Action for F
where
    F: Fn(&mut Vim),
    F: 'static,
{
    fn run(&self, state: &mut Vim) {
        self(state)
    }
}

pub enum MapAction {
    Act(Arc<dyn Action>),
    Wait,
    None,
}

enum KeyMapAction {
    Action(Arc<dyn Action>),
    Chord(HashMap<KeyEvent, KeyMapAction>, Option<Arc<dyn Action>>),
}

impl Debug for KeyMapAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Action(_) => write!(f, "Action(dyn Action)"),
            Self::Chord(map, a) => write!(
                f,
                "Chord({:?}, {})",
                map,
                if a.is_some() {
                    "Some(dyn Action)"
                } else {
                    "None"
                }
            ),
        }
    }
}

#[derive(Debug, Default)]
pub struct KeyMap {
    map: HashMap<KeyEvent, KeyMapAction>,
    state: Vec<KeyEvent>,
}

impl KeyMap {
    pub fn clear(&mut self) {
        self.state.clear();
    }

    pub fn on_key(&mut self, k: KeyEvent) -> MapAction {
        self.state.push(k);
        match self.get_action(self.state.as_ref()) {
            Some(KeyMapAction::Action(a)) => {
                let ret = MapAction::Act(Arc::clone(a));
                self.state.clear();
                ret
            }
            Some(KeyMapAction::Chord(_, _)) => MapAction::Wait,
            None => self.default_action(),
        }
    }

    fn get_action<'s>(&'s self, path: &[KeyEvent]) -> Option<&'s KeyMapAction> {
        let mut cur = &self.map;
        for event in path {
            match cur.get(event)? {
                a @ KeyMapAction::Action(_) => return Some(a),
                KeyMapAction::Chord(map, _a) => cur = map,
            }
        }
        None
    }

    fn default_action(&mut self) -> MapAction {
        while !self.state.is_empty() {
            self.state.pop();
            match self.get_action(self.state.as_ref()) {
                Some(KeyMapAction::Chord(_, Some(a))) => {
                    let ret = MapAction::Act(Arc::clone(a));
                    self.state.clear();
                    return ret;
                }
                _ => (),
            }
        }
        MapAction::None
    }
}

#[derive(Debug, Enum, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Normal,
    Insert,
    Visual,
    Operator,
    Cli,
}

impl Default for State {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Default)]
pub struct MapSet {
    map: EnumMap<State, KeyMap>,
    last: State,
}

macro_rules! keys {
    ($map:ident, State::$name:ident => { $($rem:tt)* }) => {
        $map.register_bindings(
            State::Normal,
            keys!([]; $($rem)*),
        );
    };
    ([$($tt:tt)*]; $c:tt $($mod:ident)* => {$($inner:tt)*} $(, $($rem:tt)*)?) => {
        [
            $($tt)*
            (
                keys!(@keycode $c $($mod)*),
                KeyMapAction::Chord({
                    keys!([]; $($inner)*).into_iter().collect()
                }, None),
            ),
        ]
    };
    ([$($tt:tt)*]; $($c:tt $($mod:ident)*)|* => |$s:ident| $e:expr $(, $($rem:tt)*)?) => {
        keys!([
         $($tt)*
         $(
             (
                 keys!(@keycode $c $($mod)*),
                 KeyMapAction::Action(Arc::new(|$s: &mut Vim| {$e}) as Arc<dyn Action>),
            ),
                 )*
        ]; $($($rem)*)?)
    };
    ([$($tt:tt)*];) => {vec![$($tt)*]};
    (@keycode $c:literal $($mod:ident)*) => {
        KeyEvent::new(KeyCode::Char($c), KeyModifiers::empty() $(| keys!(@modkey $mod))*)
    };
    (@keycode F($c:literal) $($mod:ident)*) => {
        KeyEvent::new(KeyCode::F($c), KeyModifiers::empty() $(| keys!(@modkey $mod))*)
    };
    (@keycode $c:ident $($mod:ident)*) => {
        KeyEvent::new(KeyCode::$c, KeyModifiers::empty() $(| keys!(@modkey $mod))*)
    };
    (@modkey C) => {KeyModifiers::CONTROL};
    (@modkey S) => {KeyModifiers::SHIFT};
    (@modkey A) => {KeyModifiers::ALT};
}

impl MapSet {
    pub fn global() -> Self {
        let mut s = Self::default();
        keys!(s, State::Normal => {
            'i' => |v| {
                v.get_focus_mut().set_mode(WinMode::Insert);
            },
            'I' => |v| {
                v.get_focus_mut().cursor_apply(Motion::Col(0)).set_mode(WinMode::Insert);
            },
            'a' => |v| {
                v.get_focus_mut().cursor_apply(Motion::Relative(1, 0)).set_mode(WinMode::Insert);
            },
            'A' => |v| {
                v.get_focus_mut().cursor_apply(Motion::Col(u16::MAX)).set_mode(WinMode::Insert);
            },
            'h' | Left => |v| {
                v.get_focus_mut().cursor_apply(Motion::Relative(-1, 0));
            },
            'l' | Right => |v| {
                v.get_focus_mut().cursor_apply(Motion::Relative(1, 0));
            },
            'j' | Down => |v| {
                v.get_focus_mut().cursor_apply(Motion::Relative(0, 1));
            },
            'k' | Up => |v| {
                v.get_focus_mut().cursor_apply(Motion::Relative(0, -1));
            },
            '$' | End => |v| {
                let win = v.get_focus_mut();
                win.cursor_apply(Motion::Col(win.buffer_area().w));
            },
            '0' => |v| {
                v.get_focus_mut().cursor_apply(Motion::Col(0));
            },
            '^' | Home => |v| {
                let win = v.get_focus_mut();
                let col = win.buffer()
                            .read()
                            .get_line(win.cursor().row(win.buffer_area()))
                            .unwrap()
                            .first_char() as u16;
                win.cursor_apply(Motion::Col(col));
            },
            'd' => |v| {
                v.get_focus_mut().set_mode(WinMode::Operation(Op::Delete));
            },
            'y' => |v| {
                v.get_focus_mut().set_mode(WinMode::Operation(Op::Yank));
            },
            'w' C => {
                'h' => |_v| todo!("Window Movement"),
                'j' => |_v| todo!("Window Movement"),
                'k' => |_v| todo!("Window Movement"),
                'l' => |_v| todo!("Window Movement"),
            },
        });
        s
    }

    //pub fn window() -> Self {
        //let mut s = Self::default();
        //s
    //}

    fn register_bindings(
        &mut self,
        state: State,
        bindings: impl IntoIterator<Item = (KeyEvent, KeyMapAction)>,
    ) {
        for (k, a) in bindings {
            self.map[state].map.insert(k, a);
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, state: State) -> MapAction {
        if self.last != state {
            self.map[self.last].clear();
        }
        self.last = state;
        self.map[state].on_key(k)
    }
}

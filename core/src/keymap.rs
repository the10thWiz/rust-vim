//
// keymap.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::{collections::HashMap, fmt::Debug, sync::Arc, io::Write};

use crossterm::{event::{KeyCode, KeyEvent, KeyModifiers}, Result};
use enum_map::{Enum, EnumMap};

use crate::{
    cli::Cli,
    cursor::Motion,
    window::{Dist, Op, Scroll, WinMode},
    Vim, util::KeyDisplay,
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
    Act(usize, Arc<dyn Action>),
    Wait,
    None,
}

#[derive(Clone)]
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

impl KeyMapAction {
    fn insert(&mut self, k: KeyEvent, a: KeyMapAction) {
        match self {
            Self::Chord(map, _) => { map.insert(k, a); },
            Self::Action(old) => {
                let tmp = Arc::clone(old);
                *self = Self::Chord(HashMap::new(), Some(tmp));
                self.insert(k, a);
            }
        }
    }

    fn get(&self, k: &KeyEvent) -> Option<&KeyMapAction> {
        match self {
            Self::Chord(map, _) => map.get(k),
            Self::Action(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct KeyMap {
    map: KeyMapAction,
    state: Vec<KeyEvent>,
    rep: usize,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            map: KeyMapAction::Chord(HashMap::new(), None),
            state: vec![],
            rep: 0,
        }
    }
}

impl KeyMap {
    pub fn clear(&mut self) {
        self.state.clear();
        self.rep = 0;
    }

    pub fn on_key(&mut self, k: KeyEvent) -> MapAction {
        if let KeyCode::Char(c) = k.code {
            if let Some(d) = c.to_digit(10).filter(|&d| d != 0 || self.rep != 0) {
                self.rep = self.rep * 10 + d as usize;
                return MapAction::Wait;
            }
        }
        self.state.push(k);
        //debug!("Key press: {k:?}");
        //debug!("Action: {:?}", self.get_action(self.state.as_ref()));
        match self.get_action(self.state.as_ref()) {
            Some(KeyMapAction::Action(a)) => {
                let ret = MapAction::Act(self.rep.max(1), Arc::clone(a));
                self.clear();
                ret
            }
            Some(KeyMapAction::Chord(_, _)) => MapAction::Wait,
            None => self.default_action(),
        }
    }

    fn get_action<'s>(&'s self, path: &[KeyEvent]) -> Option<&'s KeyMapAction> {
        let mut cur = &self.map;
        for event in path {
            match cur {
                a @ KeyMapAction::Action(_) => return Some(a),
                map @ KeyMapAction::Chord(_, _) => cur = map.get(event)?,
            }
        }
        Some(cur)
    }

    fn default_action(&mut self) -> MapAction {
        while !self.state.is_empty() {
            self.state.pop();
            match self.get_action(self.state.as_ref()) {
                Some(KeyMapAction::Chord(_, Some(a))) => {
                    let ret = MapAction::Act(self.rep.max(1), Arc::clone(a));
                    self.clear();
                    return ret;
                }
                _ => (),
            }
        }
        MapAction::None
    }

    pub fn draw<W: Write>(&self, term: &mut W) -> Result<()> {
        for k in self.state.iter() {
            write!(term, "{}", KeyDisplay(*k))?;
        }
        Ok(())
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
            Esc => |_v| (),
            'i' => |v| {
                v.set_mode(WinMode::Insert);
            },
            'I' => |v| {
                v.set_mode(WinMode::Insert).cursor_apply(Motion::SetCol(0));
            },
            'a' => |v| {
                v.set_mode(WinMode::Insert).cursor_apply(Motion::Right);
            },
            'A' => |v| {
                v.set_mode(WinMode::Insert).cursor_apply(Motion::End);
            },
            'v' => |v| {
                v.set_mode(WinMode::Visual);
            },
            'V' => |v| {
                v.set_mode(WinMode::VisualLine);
            },
            'h' | Left => |v| {
                v.get_focus_mut().cursor_apply(Motion::Left);
            },
            'l' | Right => |v| {
                v.get_focus_mut().cursor_apply(Motion::Right);
            },
            'j' | Down => |v| {
                v.get_focus_mut().cursor_apply(Motion::Down);
            },
            'k' | Up => |v| {
                v.get_focus_mut().cursor_apply(Motion::Up);
            },
            '$' | End => |v| {
                v.get_focus_mut().cursor_apply(Motion::End);
            },
            '0' => |v| {
                v.get_focus_mut().cursor_apply(Motion::SetCol(0));
            },
            '^' | Home => |v| {
                let win = v.get_focus_mut();
                let col = win.buffer()
                            .read()
                            .get_line(win.cursor().row())
                            .unwrap()
                            .first_char();
                win.cursor_apply(Motion::SetCol(col));
            },
            'd' => |v| {
                v.set_mode(WinMode::Operation(Op::Delete));
            },
            'y' => |v| {
                v.set_mode(WinMode::Operation(Op::Yank));
            },
            'r' => |v| {
                v.set_mode(WinMode::Operation(Op::Replace));
            },
            'R' => |v| {
                v.set_mode(WinMode::Replace);
            },
            ':' => |v| {
                v.start_cli(Cli::Command);
            },
            'e' C => |v| {
                v.get_focus_mut().scroll(Scroll::Down, Dist::One);
            },
            'Y' C => |v| {
                v.get_focus_mut().scroll(Scroll::Up, Dist::One);
            },
            'w' C => {
                'h' => |v| v.move_focus(Scroll::Left),
                'j' => |v| v.move_focus(Scroll::Down),
                'k' => |v| v.move_focus(Scroll::Up),
                'l' => |v| v.move_focus(Scroll::Right),
            },
        });
        let arrow_keys = s.clone_bindings(
            State::Normal,
            [
                keys!(@keycode Up),
                keys!(@keycode Down),
                keys!(@keycode Left),
                keys!(@keycode Right),
                keys!(@keycode Home),
                keys!(@keycode End),
            ],
        );
        s.register_bindings(State::Insert, arrow_keys.iter().cloned());
        s.register_bindings(State::Visual, arrow_keys.iter().cloned());
        let hjkl_keys = s.clone_bindings(
            State::Normal,
            [
                keys!(@keycode 'h'),
                keys!(@keycode 'j'),
                keys!(@keycode 'k'),
                keys!(@keycode 'l'),
                keys!(@keycode '$'),
                keys!(@keycode '^'),
                keys!(@keycode '0'),
                keys!(@keycode 'e' C),
                keys!(@keycode 'Y' C),
            ],
        );
        s.register_bindings(State::Visual, hjkl_keys.iter().cloned());
        let win_keys = s.clone_bindings(State::Normal, [keys!(@keycode 'w' C)]);
        s.register_bindings(State::Insert, win_keys.iter().cloned());
        s.register_bindings(State::Visual, win_keys.iter().cloned());
        s.register_bindings(State::Operator, win_keys.iter().cloned());
        s
    }

    fn register_bindings(
        &mut self,
        state: State,
        bindings: impl IntoIterator<Item = (KeyEvent, KeyMapAction)>,
    ) {
        for (k, a) in bindings {
            self.map[state].map.insert(k, a);
        }
    }

    fn clone_bindings(
        &self,
        state: State,
        iter: impl IntoIterator<Item = KeyEvent>,
    ) -> Vec<(KeyEvent, KeyMapAction)> {
        let mut ret = vec![];
        for k in iter {
            if let Some(act) = self.map[state].map.get(&k) {
                ret.push((k, act.clone()));
            }
        }
        ret
    }

    pub fn on_key(&mut self, k: KeyEvent, state: State) -> MapAction {
        if self.last != state {
            self.map[self.last].clear();
        }
        self.last = state;
        self.map[state].on_key(k)
    }

    pub fn draw<W: Write>(&self, term: &mut W, state: State) -> Result<()> {
        self.map[state].draw(term)
    }
}

use crate::keymap::{Action, KeyMappings};
use crate::{
    window::{Motion, Window},
    EditorState,
};
use terminal::{KeyCode, KeyEvent, KeyModifiers};

fn ch(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())
}
fn code(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::empty())
}

pub fn normal_map(map: &mut KeyMappings) {
    let movement = Action::chord()
        .add(
            ch('k'),
            Action::st(&|s| s.active_window().move_cursor(Motion::Relative(0, -1))),
        )
        .add(
            ch('j'),
            Action::st(&|s| s.active_window().move_cursor(Motion::Relative(0, 1))),
        )
        .add(
            ch('h'),
            Action::st(&|s| s.active_window().move_cursor(Motion::Relative(-1, 0))),
        )
        .add(
            ch('l'),
            Action::st(&|s| s.active_window().move_cursor(Motion::Relative(1, 0))),
        )
        .dup(code(KeyCode::Up), ch('k'))
        .dup(code(KeyCode::Down), ch('j'))
        .dup(code(KeyCode::Right), ch('l'))
        .dup(code(KeyCode::Left), ch('h'));
    map.add_basic_map(&movement);
}

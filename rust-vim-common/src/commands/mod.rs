use crate::keymap::KeyMappings;
mod movement;

pub fn normal_map(map: &mut KeyMappings) {
    movement::normal_map(map);
}

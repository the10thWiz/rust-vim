use super::EditorState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use terminal::KeyEvent;

pub struct ActionBuilder(HashMap<KeyEvent, Arc<Action>>);
impl ActionBuilder {
    pub fn add(mut self, key: KeyEvent, action: Arc<Action>) -> Self {
        self.0.insert(key, action);
        self
    }
    pub fn dup(mut self, old: KeyEvent, new: KeyEvent) -> Self {
        if let Some(a) = self.0.get(&old).cloned() {
            self.0.insert(new, a);
        }
        self
    }
    pub fn build(self) -> Arc<Action> {
        Arc::new(Action::Chord(self.0))
    }
}

pub enum Action {
    Boxed(Box<dyn Fn(&mut EditorState)>),
    Static(&'static dyn Fn(&mut EditorState)),
    Chord(HashMap<KeyEvent, Arc<Action>>),
    NoOp(),
}

impl Action {
    pub fn execute(self: Arc<Self>, state: &mut EditorState) {
        match &*self {
            Self::Static(f) => f(state),
            Self::Boxed(f) => f(state),
            Self::Chord(_) => (),
            Self::NoOp() => (),
        }
    }
    pub fn add_key(self: Arc<Self>, key: KeyEvent) -> Option<Arc<Self>> {
        match &*self {
            Self::Chord(map) => map.get(&key).cloned(),
            Self::Boxed(_) | Self::Static(_) => None,
            Self::NoOp() => None,
        }
    }
    pub fn st(f: &'static dyn Fn(&mut EditorState)) -> Arc<Self> {
        Arc::new(Self::Static(f))
    }
    pub fn bx(f: Box<dyn Fn(&mut EditorState)>) -> Arc<Self> {
        Arc::new(Self::Boxed(f))
    }
    pub fn chord() -> ActionBuilder {
        ActionBuilder(HashMap::new())
    }
}

/**
 * KeyMappings
 *
 * Key mappings allow easy adaptation of keybindings
 * and allow rvi to detect duplicate keybindings between
 * plugins
 *
 * Key mappings map from native Curses/Keyboard codes to
 * a string. Namespaces are delinitated with a colon,
 * so `l:a` is the keybinding `a` in the namespace `l`
 *
 * There are several default namespaces:
 * - `l`: Letters
 * - `c`: control codes
 * - `s`: special characters (i.e. arrows, return, end, etc)
 * - `u`: unknown, identified as a numeric code, "{:X}"
 *
 * At this point, it should not be nessecary to add new namespaces
 */
pub struct KeyMappings {
    basic_map: HashMap<KeyEvent, Arc<Action>>,
    plugin_map: HashMap<KeyEvent, Arc<Action>>,
    running_action: Mutex<Option<Arc<Action>>>,
}

impl KeyMappings {
    pub fn new() -> Self {
        Self {
            basic_map: HashMap::new(),
            plugin_map: HashMap::new(),
            running_action: Mutex::new(None),
        }
    }
    pub fn add_basic_binding(&mut self, key: KeyEvent, action: Arc<Action>) {
        self.basic_map.insert(key, action);
    }
    pub fn add_basic_map(&mut self, action: &ActionBuilder) {
        for (&key, action) in action.0.iter() {
            self.basic_map.insert(key, action.clone());
        }
    }
    pub fn add_plugin_binding(&mut self, key: KeyEvent, action: Arc<Action>) {
        self.plugin_map.insert(key, action);
    }
    pub fn on_key(&self, key: KeyEvent) -> Arc<Action> {
        // Add key to running action
        //   else execute action
        //   else execute plugin action
        let mut running_action = self.running_action.lock().expect("Lock Issue");
        if let Some(action) = &*running_action {
            *running_action = action.clone().add_key(key);
            Arc::new(Action::NoOp())
        } else if let Some(action) = self.basic_map.get(&key) {
            *running_action = None;
            action.clone()
        } else if let Some(action) = self.plugin_map.get(&key) {
            *running_action = None;
            action.clone()
        } else {
            *running_action = None;
            Arc::new(Action::NoOp())
        }
    }
}

mod channelmap {
    use std::sync::mpsc::*;
    use terminal::KeyEvent;
    pub enum Event {
        /// A keypress event
        Keypress(KeyEvent),
        /// The result of an intermediate press
        Intermediate(),
    }
    pub enum Response {
        /// All actions have been completed
        ///
        /// More specifically, the host is
        /// free to move onto other input
        Complete(),
        /// The next keypress should be sent
        /// to this stream, regardless of what
        /// key is pressed
        NextAny(),
        /// The next keypress should be sent
        /// to another stream, after which a
        /// Intermediate Event should will be
        /// sent to this stream
        NextOther(),
        /// No more keys should be processed
        /// for this stream
        NextDone(),
    }
    pub struct Keymap {
        map: (),
        action_stack: (),
    }
    impl Keymap {
        /// Adds the channel as a binding for the keys provided
        pub fn add_binding(keys: (), channel: (Sender<Event>, Receiver<Response>)) {}
        pub fn dispatch_key(key: KeyEvent) {
        }
    }
}

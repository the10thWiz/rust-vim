//
// namespace.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::collections::HashMap;

type Result<T> = std::result::Result<T, NamespaceError>;
#[derive(Debug)]
pub enum NamespaceError {
    NamespaceNotDefined(Namespace),
    UnknownNamespace,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Id(usize);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Namespace {
    Global,
    Buffer,
    Window,
    Script,
    Local,
    Builtin,
}

impl Namespace {
    pub fn from_name(s: &str) -> Result<Self> {
        if s.starts_with("g:") {
            Ok(Self::Global)
        } else if s.starts_with("b:") {
            Ok(Self::Buffer)
        } else if s.starts_with("w:") {
            Ok(Self::Window)
        } else if s.starts_with("s:") {
            Ok(Self::Script)
        } else if s.starts_with("v:") {
            Ok(Self::Builtin)
        } else if s.contains(':') {
            Err(NamespaceError::UnknownNamespace)
        } else {
            Ok(Self::Local)
        }
    }
}

#[derive(Debug)]
pub struct NameSpaced<T> {
    global: HashMap<String, T>,
    buffer: HashMap<Id, HashMap<String, T>>,
    window: HashMap<Id, HashMap<String, T>>,
    script: HashMap<Id, HashMap<String, T>>,
    local: Vec<HashMap<String, T>>,
    builtin: HashMap<String, T>,
    buffer_id: Option<Id>,
    window_id: Option<Id>,
    script_id: Option<Id>,
}

impl<T> Default for NameSpaced<T> {
    fn default() -> Self {
        Self {
            global: HashMap::new(),
            buffer: HashMap::new(),
            window: HashMap::new(),
            script: HashMap::new(),
            local: Vec::new(),
            builtin: HashMap::new(),
            buffer_id: None,
            window_id: None,
            script_id: None,
        }
    }
}

impl<T> NameSpaced<T> {
    fn get_mut(&mut self, namesapce: Namespace) -> Result<&mut HashMap<String, T>> {
        Ok(match namesapce {
            Namespace::Global => &mut self.global,
            Namespace::Local => self
                .local
                .last_mut()
                .ok_or(NamespaceError::NamespaceNotDefined(Namespace::Local))?,
            Namespace::Buffer => self
                .buffer
                .entry(
                    self.buffer_id
                        .ok_or(NamespaceError::NamespaceNotDefined(Namespace::Buffer))?,
                )
                .or_default(),
            Namespace::Window => self
                .buffer
                .entry(
                    self.window_id
                        .ok_or(NamespaceError::NamespaceNotDefined(Namespace::Window))?,
                )
                .or_default(),
            Namespace::Script => self
                .buffer
                .entry(
                    self.script_id
                        .ok_or(NamespaceError::NamespaceNotDefined(Namespace::Script))?,
                )
                .or_default(),
            Namespace::Builtin => &mut self.builtin,
        })
    }

    pub fn insert(&mut self, name: impl Into<String>, val: T) -> Result<Option<T>> {
        let name = name.into();
        Ok(self
            .get_mut(Namespace::from_name(name.as_str())?)?
            .insert(name, val))
    }

    pub fn remove(&mut self, name: impl AsRef<str>) -> Result<Option<T>> {
        let name = name.as_ref();
        Ok(self.get_mut(Namespace::from_name(name)?)?.remove(name))
    }

    pub fn insert_builtin(&mut self, name: impl Into<String>, val: T) -> Option<T> {
        self.builtin.insert(name.into(), val)
    }

    pub fn get(&self, name: impl AsRef<str>) -> Result<Option<&T>> {
        let name = name.as_ref();
        Ok(match Namespace::from_name(name)? {
            Namespace::Global => self.global.get(name),
            Namespace::Buffer => self
                .buffer
                .get(
                    &self
                        .buffer_id
                        .ok_or(NamespaceError::NamespaceNotDefined(Namespace::Buffer))?,
                )
                .and_then(|m| m.get(name)),
            Namespace::Window => self
                .window
                .get(
                    &self
                        .buffer_id
                        .ok_or(NamespaceError::NamespaceNotDefined(Namespace::Buffer))?,
                )
                .and_then(|m| m.get(name)),
            Namespace::Script => self
                .script
                .get(
                    &self
                        .buffer_id
                        .ok_or(NamespaceError::NamespaceNotDefined(Namespace::Buffer))?,
                )
                .and_then(|m| m.get(name)),
            Namespace::Local => self.local.iter().rev().find_map(|m| m.get(name)).or_else(|| self.builtin.get(name)),
            Namespace::Builtin => self.builtin.get(name),
        })
    }

    pub fn set_window(&mut self, id: impl Into<Option<Id>>) {
        self.window_id = id.into();
    }

    pub fn set_buffer(&mut self, id: impl Into<Option<Id>>) {
        self.buffer_id = id.into();
    }

    pub fn set_script(&mut self, id: impl Into<Option<Id>>) {
        self.script_id = id.into();
    }

    pub fn enter_local(&mut self) {
        self.local.push(HashMap::new());
    }
    pub fn leave_local(&mut self) {
        self.local.pop();
    }
}

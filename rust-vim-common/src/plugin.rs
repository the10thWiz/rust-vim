//
// plugin.rs
// Copyright (C) 2020 matt <matt@mattlaptop>
// Distributed under terms of the MIT license.
//
use std::collections::HashMap;
use std::thread::{self, JoinHandle};

pub trait Plugin {
    fn init(&mut self) -> ();
    fn handle(&mut self) -> ();
}

pub struct PluginHost {
    plugins: HashMap<String, JoinHandle<()>>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    pub fn add(&mut self, id: String, plugin: Box<dyn Plugin>) {
        // Starts the plugins on another thread...
        let plugin_id = id.clone();
        let join_handle = thread::spawn(move || {});
        if let Some(_) = self.plugins.insert(id, join_handle) {
            panic!("Id {} already has a plugin loaded", id);
        }
    }
}

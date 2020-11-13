use log::{error, info, warn};
pub type Result<S> = std::result::Result<S, PluginError>;

pub enum PluginError {
    HungUp(),
    InvalidAction(),
}

use message_plugins::{Message, Plugin};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};

struct Ch<T> {
    id: String,
    send: Sender<Message<T>>,
    //recv: Option<Receiver<Message<T>>>,
    handle: JoinHandle<Option<u8>>,
}

impl<T> Ch<T> {
    fn send(&self, message: Message<T>) -> bool {
        if let Err(e) = self.send.send(message) {
            error!("Plugin `{}` ended before host", self.id);
            true
        } else {
            false
        }
    }
}

pub struct Host<T> {
    plugins: HashMap<String, Ch<T>>,
}

impl<T: Sync + Send + 'static> Host<T> {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    pub fn attach(&mut self, id: impl Into<String>, mut plugin: impl Plugin<T>) {
        let id = id.into();
        if self.plugins.contains_key(&id) {
            warn!("Plugin {} has already been loaded", id);
        } else {
            let (send, rx) = channel();
            let handle = spawn(move || {
                while let Ok(message) = rx.recv() {
                    if let Some(status) = plugin.handle_message(message) {
                        return Some(status);
                    }
                }
                None
            });
            self.plugins.insert(id.clone(), Ch { id, send, handle });
        }
    }
    pub fn dettach(&mut self, id: String) {
        if let Some(ch) = self.plugins.remove(&id) {
            match ch.handle.join() {
                Ok(Some(code)) => warn!("Plugin: {} ended with code {}", id, code),
                Ok(None) => info!("Plugin: {} ended without a  code", id),
                Err(e) => error!("Plugin: {} ended with error {:?}", id, e),
            }
        }
    }
    pub fn send_all(&self, message: impl Into<Message<T>>) {
        let message = message.into();
        for ch in self.plugins.values() {
            ch.send(message.clone());
        }
    }
    pub fn send(&self, id: impl AsRef<str>, message: impl Into<Message<T>>) {
        if let Some(ch) = self.plugins.get(id.as_ref()) {
            ch.send(message.into());
        } else {
            warn!("Plugin {} is not loaded", id.as_ref());
        }
    }
    pub fn end(self, exit_message: impl Into<Message<T>>) {
        self.send_all(exit_message);
        for (id, ch) in self.plugins {
            match ch.handle.join() {
                Ok(Some(code)) => warn!("Plugin: {} ended with code {}", id, code),
                Ok(None) => info!("Plugin: {} ended without a  code", id),
                Err(e) => error!("Plugin: {} ended with error {:?}", id, e),
            }
        }
    }
}

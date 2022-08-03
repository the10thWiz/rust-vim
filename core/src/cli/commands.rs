//
// commands.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use crate::Vim;
use std::sync::Arc;

use super::{CliCommand, CliState};

fn multi<'a>(
    reg: &mut CliState,
    iter: impl IntoIterator<Item = &'a str>,
    f: impl CliCommand + 'static,
) {
    let cmd: Arc<dyn CliCommand> = Arc::new(f);
    for name in iter {
        reg.register_command(name, Arc::clone(&cmd));
    }
}

pub fn default(reg: &mut CliState) {
    multi(reg, ["q", "quit"], |_a: &str, v: &mut Vim| v.exit());
    multi(reg, ["w", "write"], |_a: &str, v: &mut Vim| {
        let res = v.get_focus().buffer().write().write_file();
        v.err(res);
    });
}

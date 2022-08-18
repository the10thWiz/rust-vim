//
// commands.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use vimscript::{CmdRange, VimScriptCtx, Command};

use crate::VimInner;
use std::sync::Arc;

struct Cmd<F>(F);
impl<F: Fn(CmdRange<'_>, bool, &str, &mut VimScriptCtx<VimInner>, &mut VimInner)> Command<VimInner>
    for Cmd<F>
{
    fn execute(
        &self,
        range: CmdRange<'_>,
        bang: bool,
        commands: &str,
        ctx: &mut VimScriptCtx<VimInner>,
        state: &mut VimInner,
    ) {
        self.0(range, bang, commands, ctx, state)
    }
}

fn multi<'a>(
    reg: &mut VimScriptCtx<VimInner>,
    iter: impl IntoIterator<Item = &'a str>,
    f: impl Fn(CmdRange<'_>, bool, &str, &mut VimScriptCtx<VimInner>, &mut VimInner) + 'static,
) {
    let cmd: Arc<dyn Command<VimInner>> = Arc::new(Cmd(f));
    for name in iter {
        reg.command(name, Arc::clone(&cmd));
    }
}

pub fn default(reg: &mut VimScriptCtx<VimInner>) {
    multi(reg, ["q", "quit"], |_range, _bang, _args, _ctx, v| v.exit());
    multi(reg, ["w", "write"], |_range, _bang, _args, _ctx, v| {
        let res = v.get_focus().buffer().write().write_file();
        v.err(res);
    });
}

use log::{error, info, warn};
use std::io::Write;
use std::time::Duration;
use terminal::*;

use rust_vim_common::EditorState;

#[allow(unused_must_use)]
fn panic_cleanup(info: &std::panic::PanicInfo) {
    let terminal = terminal::stdout();
    terminal.act(Action::DisableRawMode);
    terminal.act(Action::DisableRawMode);
    terminal.batch(Action::DisableMouseCapture);
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        error!("Error: {}", s);
    }
    if let Some(loc) = info.location() {
        error!(
            "A Panic occured at: {}:{}:{}",
            loc.file(),
            loc.line(),
            loc.column()
        );
    } else {
        error!("A Panic occured somewhere");
    }
}

#[allow(unused_must_use)]
fn main() {
    env_logger::init();
    info!("Starting Init");
    let mut terminal = terminal::stdout();
    std::panic::set_hook(Box::new(panic_cleanup));
    match run(&mut terminal) {
        _ => (),
    }
    terminal.batch(Action::DisableRawMode);
    terminal.batch(Action::DisableRawMode);
    terminal.batch(Action::DisableMouseCapture);
    terminal.flush_batch();
}

fn run<W: Write>(terminal: &mut Terminal<W>) -> error::Result<()> {
    terminal.batch(Action::EnableRawMode)?;
    terminal.batch(Action::EnterAlternateScreen)?;
    terminal.batch(Action::EnableMouseCapture)?;

    let size = if let Retrieved::TerminalSize(col, row) = terminal.get(Value::TerminalSize)? {
        (col, row)
    } else {
        (0, 0)
    };
    let mut state = EditorState::init(size);

    draw(&mut state, terminal, true)?;

    while !state.is_done() {
        if let Ok(Retrieved::Event(Some(ev))) =
            terminal.get(Value::Event(Some(Duration::from_millis(10))))
        {
            match ev {
                Event::Resize => draw(&mut state, terminal, true)?,
                Event::Key(key) => {
                    state.on_key(key);
                    draw(&mut state, terminal, true)?;
                }
                Event::Mouse(mouse) => (),
                Event::Unknown => (),
            }
        } else {
            draw(&mut state, terminal, false)?;
        }
    }
    Ok(())
}

/// Draws the screen
///
/// draw_always: when true, state.draw() will always be called
fn draw<W: Write>(
    state: &mut EditorState,
    terminal: &mut Terminal<W>,
    draw_always: bool,
) -> error::Result<()> {
    if let Retrieved::TerminalSize(col, row) = terminal.get(Value::TerminalSize)? {
        if state.update((col, row)) || draw_always {
            state.draw(terminal)?;
        }
    } else {
        unreachable!()
    }
    Ok(())
}

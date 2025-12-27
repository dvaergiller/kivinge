use super::error::Error;
use crossterm::execute;
use crossterm::terminal;
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
pub use ratatui::{prelude, widgets};
use std::io;
use std::io::stdout;
use std::ops::{Deref, DerefMut};
use std::panic;

#[derive(Debug)]
pub struct LoadedTerminal(Terminal<CrosstermBackend<io::Stdout>>);

impl Deref for LoadedTerminal {
    type Target = Terminal<CrosstermBackend<io::Stdout>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LoadedTerminal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for LoadedTerminal {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("IO Error disabling terminal raw mode");
        io::stdout()
            .execute(terminal::LeaveAlternateScreen)
            .expect("IO Error leaving alternate screen");
    }
}

pub fn load() -> Result<LoadedTerminal, Error> {
    terminal::enable_raw_mode()?;
    io::stdout().execute(terminal::EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));
    Ok(LoadedTerminal(terminal))
}

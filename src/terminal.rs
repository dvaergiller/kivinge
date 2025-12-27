use crossterm::terminal;
use crossterm::ExecutableCommand;
pub use ratatui::{prelude, widgets};
use ratatui::prelude::*;
use std::io;
use std::ops::{Deref, DerefMut};
use super::kivra::error::Error;

#[derive(Debug)]
pub struct LoadedTerminal(Terminal<CrosstermBackend<io::Stdout>>);

impl Deref for LoadedTerminal {
    type Target = Terminal<CrosstermBackend<io::Stdout>>;
    fn deref(&self) ->
        &Self::Target
    {
        &self.0
    }
}

impl DerefMut for LoadedTerminal {
    fn deref_mut(&mut self) ->
        &mut Self::Target
    {
        &mut self.0
    }
}

impl Drop for LoadedTerminal {
    fn drop(&mut self) {
        terminal::disable_raw_mode()
            .expect("IO Error disabling terminal raw mode");
        io::stdout().execute(terminal::LeaveAlternateScreen)
            .expect("IO Error leaving alternate screen");
    }
}

pub fn load() -> Result<LoadedTerminal, Error> {
    terminal::enable_raw_mode()?;
    io::stdout().execute(terminal::EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    Ok(LoadedTerminal(terminal))
}

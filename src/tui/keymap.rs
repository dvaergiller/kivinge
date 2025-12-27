use crossterm::event::{Event, KeyCode};

use crate::error::Error;

pub enum KeyCommand {
    Up,
    Down,
    Select,
    Back,
    Quit,
    Key(KeyCode),
    Unknown,
}

pub fn read_key() -> Result<KeyCommand, Error> {
    match crossterm::event::read()? {
        Event::Key(key) => match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('p') => {
                Ok(KeyCommand::Up)
            }

            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('n') => {
                Ok(KeyCommand::Down)
            }

            KeyCode::Enter
            | KeyCode::Right
            | KeyCode::Char('l')
            | KeyCode::Char('f') => Ok(KeyCommand::Select),

            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('b') => {
                Ok(KeyCommand::Back)
            }

            KeyCode::Esc | KeyCode::Char('q') => Ok(KeyCommand::Quit),

            _ => Ok(KeyCommand::Key(key.code)),
        },
        _ => Ok(KeyCommand::Unknown),
    }
}

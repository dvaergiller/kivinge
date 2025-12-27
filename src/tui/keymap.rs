use crossterm::event::{Event, KeyCode};

use crate::error::Error;

pub enum KeyEvent {
    Up,
    Down,
    Select,
    Back,
    Quit,
    Key(KeyCode),
    Unknown,
}

pub fn read_key() -> Result<KeyEvent, Error> {
    match crossterm::event::read()? {
        Event::Key(key) => match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('p') => {
                Ok(KeyEvent::Up)
            }

            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('n') => {
                Ok(KeyEvent::Down)
            }

            KeyCode::Enter
            | KeyCode::Right
            | KeyCode::Char('l')
            | KeyCode::Char('f') => Ok(KeyEvent::Select),

            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('b') => {
                Ok(KeyEvent::Back)
            }

            KeyCode::Esc | KeyCode::Char('q') => Ok(KeyEvent::Quit),

            _ => Ok(KeyEvent::Key(key.code)),
        },
        _ => Ok(KeyEvent::Unknown),
    }
}

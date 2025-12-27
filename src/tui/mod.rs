use std::time::Duration;

use crossterm::event::poll;
use keymap::{read_key, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    widgets::Paragraph,
    Frame,
};
use terminal::LoadedTerminal;
use thiserror::Error;

use crate::client::session::Session;

pub mod inbox;
pub mod inbox_item;
mod keymap;
pub mod login;
pub mod qr;
pub mod terminal;

#[derive(Debug, Error)]
pub enum Error {
    #[error("QR code generation failed: {0}")]
    QRError(#[from] qrcode::types::QrError),

    #[error("IO error encountered: {0}")]
    IOError(#[from] std::io::Error),

    #[error("HTTP client error: {0}")]
    ClientError(#[from] crate::client::Error),

    #[error("app error: {0}")]
    AppError(&'static str),
}

pub trait TuiView {
    type ReturnType;

    fn update(
        &mut self,
        event: Event,
    ) -> Result<Command<Self::ReturnType>, Error>;
    fn render(&mut self, frame: &mut Frame, rect: Rect);
}

pub enum Command<Ret> {
    AwaitKey,
    AwaitTimeout(Duration),
    Return(Ret),
}

pub enum Event {
    Init,
    Key(KeyEvent),
    Timeout,
}

pub fn show<Ret>(
    view: &mut impl TuiView<ReturnType = Ret>,
    terminal: &mut LoadedTerminal,
    session: Option<&Session>,
) -> Result<Ret, Error> {
    let mut command = view.update(Event::Init)?;

    loop {
        let draw = |frame: &mut Frame| {
            let subview_rect = render_main(frame, session);
            view.render(frame, subview_rect);
        };
        terminal.draw(draw)?;

        match command {
            Command::AwaitKey => {
                let key = read_key()?;
                command = view.update(Event::Key(key))?;
            }

            Command::AwaitTimeout(duration) => {
                if poll(duration)? {
                    let key = read_key()?;
                    command = view.update(Event::Key(key))?;
                } else {
                    command = view.update(Event::Timeout)?;
                }
            }

            Command::Return(ret) => {
                return Ok(ret);
            }
        }
    }
}

fn render_main(frame: &mut Frame, session: Option<&Session>) -> Rect {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
        .split(frame.size());

    let header = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints(vec![Constraint::Fill(1), Constraint::Fill(1)])
        .split(layout[0]);

    let title =
        Paragraph::new("Kivinge")
        .bold()
        .fg(ratatui::style::Color::Black)
        .bg(ratatui::style::Color::Green);
    frame.render_widget(title, header[0]);

    let user_name =
        session.map(|s| s.user_info.name.clone()).unwrap_or_default();
    let session_header = Paragraph::new(user_name)
        .fg(ratatui::style::Color::Black)
        .bg(ratatui::style::Color::Green)
        .right_aligned();
    frame.render_widget(session_header, header[1]);
    layout[1]
}

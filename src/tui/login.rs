use crossterm::event::{poll, read, Event, KeyCode};
use ratatui::{prelude, widgets};
use std::time::Duration;

use super::{qr, terminal::LoadedTerminal};
use crate::{client::Client, error::Error, model::auth::AuthTokenResponse};

struct State {
    qr_code: String,
    next_poll_url: String,
    retry_after: u32,
}

pub fn show(
    terminal: &mut LoadedTerminal,
    client: &impl Client,
) -> Result<Option<AuthTokenResponse>, Error> {
    let config = client.get_config()?;
    let (verifier, auth_resp) = client.start_auth(&config)?;

    let mut state = State {
        qr_code: auth_resp.qr_code,
        next_poll_url: auth_resp.next_poll_url,
        retry_after: 1,
    };

    loop {
        render(terminal, &state.qr_code)?;

        if poll(Duration::from_secs(state.retry_after.into()))? {
            match read()? {
                Event::Key(key) if key.code == KeyCode::Char('q') => {
                    client.abort_auth(&state.next_poll_url)?;
                    return Ok(None);
                }
                _ => (),
            }
        }

        let check = client.check_auth(&state.next_poll_url)?;
        match check.ssn {
            None => {
                state.qr_code = check.qr_code;
                state.next_poll_url =
                    check.next_poll_url.unwrap_or(state.next_poll_url);
                state.retry_after =
                    check.retry_after.unwrap_or(state.retry_after);
            }
            Some(_) => {
                return client
                    .get_auth_token(&config, auth_resp.code, verifier)
                    .map(Some);
            }
        }
    }
}

pub fn render(
    terminal: &mut LoadedTerminal,
    qr_code: &String,
) -> Result<(), Error> {
    let qr = qr::encode(qr_code)?;

    let title = "Authenticate with BankID";
    let block = widgets::Block::default()
        .title(title)
        .title_alignment(prelude::Alignment::Center)
        .title_bottom("Press 'q' to abort login")
        .borders(widgets::Borders::NONE);
    let paragraph = widgets::Paragraph::new(qr).block(block).centered();

    let draw = |frame: &mut prelude::Frame| {
        frame.render_widget(paragraph, frame.size());
    };
    terminal.clear()?;
    terminal.draw(draw)?;
    Ok(())
}

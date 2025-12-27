use crossterm::event::{Event, KeyCode, poll, read};
use std::time::Duration;

use crate::kivra::{model, qr, request};
use crate::terminal::{self, prelude, widgets, LoadedTerminal};
use crate::kivra::error::Error;

struct State {
    qr_code: String,
    next_poll_url: String,
    retry_after: u32,
}

pub fn show(terminal: &mut terminal::LoadedTerminal, client: &request::Client) ->
    Result<Option<model::AuthTokenResponse>, Error>
{
    let config = request::get_config(client)?;
    let (verifier, auth_resp) = request::start_auth(client, &config)?;

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
                    request::abort_auth(client, &state.next_poll_url)?;
                    return Ok(None)
                }
                _ => ()
            }
        }

        let check = request::check_auth(client, &state.next_poll_url)?;
        match check.ssn {
            None => {
                state.qr_code = check.qr_code;
                state.next_poll_url = check.next_poll_url.unwrap_or(state.next_poll_url);
                state.retry_after = check.retry_after.unwrap_or(state.retry_after);
            }
            Some(_) => {
                return request::get_auth_token(client, &config, auth_resp.code, verifier).map(Some);
            }
        }
    }
}

pub fn test_render(terminal: &mut LoadedTerminal) -> Result<(), Error> {
    let qr = "bankid.aba9c84c-c6ab-420f-830d-6aaa59ce9599.3.\
              d421c4e52d81fe6ad708b6817aef3b23be7a306d800324b871b0303c041e62f2";
    render(terminal, &qr.to_string())?;
    read()?;
    Ok(())
}

fn render(terminal: &mut LoadedTerminal, qr_code: &String) -> Result<(), Error> {
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

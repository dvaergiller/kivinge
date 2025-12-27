use std::thread::sleep;

use kivinge::kivra::{request, qr, session, model::*, error::Error};
use kivinge::{terminal, terminal::prelude, terminal::widgets};

fn main() {
    run().unwrap_or_else(|err| println!("{}", err));
}

fn run() -> Result<(), Error> {
    let client = request::client();
    let config = request::get_config(&client)?;
    let mut terminal = terminal::load()?;
    let session = load_session_or_login(&mut terminal, &client, &config)?;
    let inbox = request::get_inbox_listing(&client,
                                           &session.user_info.kivra_user_id,
                                           &session.access_token)?;

    inbox.iter().for_each(
        |entry| println!("{}: {}", entry.sender_name, entry.subject));

    Ok(())
}

fn load_session_or_login(terminal: &mut terminal::LoadedTerminal,
                         client: &request::Client,
                         config: &Config) ->
    Result<session::Session, Error>
{
    let loaded = session::try_load()?;
    if let Some(session) = loaded {
        return Ok(session);
    }

    let auth_response = login(terminal, &client, &config)?;
    let session = session::make(auth_response.access_token,
                                auth_response.id_token)?;
    session::save(&session)?;
    Ok(session)
}

fn login(terminal: &mut terminal::LoadedTerminal,
         client: &request::Client,
         config: &Config) ->
    Result<AuthTokenResponse, Error>
{
    let (verifier, auth_resp) = request::start_auth(&client, &config)?;
    let qrcode = qr::encode(&auth_resp.qr_code)?;

    terminal.clear()?;
    terminal.draw(|f| ui_show_login_qr_code(f, &qrcode))?;

    loop {
        let mut status = request::check_auth(&client, &auth_resp.next_poll_url)?;

        match (status.retry_after, &status.next_poll_url, &status.ssn) {
            (Some(retry_after), Some(next_poll_url), _) => {
                sleep(std::time::Duration::from_secs(retry_after.into()));
                status = request::check_auth(&client, &next_poll_url)?;
                let qrcode = qr::encode(&status.qr_code)?;
                terminal.draw(|f| ui_show_login_qr_code(f, &qrcode))?;
            }
            (_, _, Some(_)) => {
                return request::get_auth_token(client,
                                               &config,
                                               &auth_resp,
                                               verifier);
            }
            (_, _, _) => {
                return Err(Error::AppError("I dont know".to_string()));
            }
        }
    }
}

fn ui_show_login_qr_code(frame: &mut prelude::Frame, qr_code: &String) {
    let title = "Authenticate with BankID";
    let block = widgets::Block::default()
        .title(title)
        .borders(widgets::Borders::ALL);
    let paragraph = widgets::Paragraph::new(qr_code.clone()).block(block);
    frame.render_widget(paragraph, frame.size());
}

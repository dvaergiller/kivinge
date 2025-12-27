use crossterm::terminal;
use crossterm::ExecutableCommand;
use std::thread::sleep;
use std::io;
use ratatui::{prelude::*, widgets::*};

use kivinge::kivra::error::Error;
use kivinge::kivra::request;
use kivinge::kivra::qr;
use kivinge::kivra::model::*;

fn main() {
    let resp = run();
    println!("{:?}", resp);
}

fn run() -> Result<AuthTokenResponse, Error> {
    let client = request::client();
    let config = request::get_config(&client)?;

    terminal::enable_raw_mode()?;
    io::stdout().execute(terminal::EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    terminal.clear()?;
    let auth_response = login_view(&mut terminal, &client, &config)?;
    let access_token = &auth_response.access_token;
    let inbox_listing = get_inbox_listing(client, user_id, access_token)?;
    terminal::disable_raw_mode()?;
    io::stdout().execute(terminal::LeaveAlternateScreen)?;
    Ok(auth_response)
}

fn ui_show_login_qr_code(frame: &mut Frame, qr_code: &String) {
    let title = "Authenticate with BankID";
    let block = Block::default().title(title).borders(Borders::ALL);
    let paragraph = Paragraph::new(qr_code.clone()).block(block);
    frame.render_widget(paragraph, frame.size());
}

fn login_view<B: Backend>(terminal: &mut Terminal<B>,
                          client: &request::Client,
                          config: &Config)
                          -> Result<AuthTokenResponse, Error>
{
    let (verifier, auth_response) = request::start_auth(&client, &config)?;
    let qrcode = qr::encode(&auth_response.qr_code)?;
    terminal.draw(|f| ui_show_login_qr_code(f, &qrcode))?;

    loop {
        let mut status = request::check_auth(&client, &auth_response.next_poll_url)?;

        match (status.retry_after, &status.next_poll_url, &status.ssn) {
            (Some(retry_after), Some(next_poll_url), _) => {
                sleep(std::time::Duration::from_secs(retry_after.into()));
                status = request::check_auth(&client, &next_poll_url)?;
                let qrcode = qr::encode(&status.qr_code)?;
                terminal.draw(|f| ui_show_login_qr_code(f, &qrcode))?;
            }
            (_, _, Some(_)) => {
                return Ok(request::get_auth_token(client, &config, &auth_response, verifier)?);
            }
            (_, _, _) => {
                return Err(Error::AppError("I dont know".to_string()));
            }
        }
    }
}

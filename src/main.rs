use clap::{Parser, Subcommand};
use std::thread::sleep;

use kivinge::kivra::{request, qr, session, model::*, error::Error};
use kivinge::{terminal, terminal::prelude, terminal::widgets};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand, Debug)]
enum Command {
    Login,
    Inbox,
    Logout,
}

fn main() {
    let cli_args = CliArgs::parse();
    match run(cli_args) {
        Ok(()) => (),
        Err(err) => println!("Error: {err}")
    }
}

fn run(cli_args: CliArgs) -> Result<(), Error> {
    let client = request::client();
    match cli_args.command {
        Command::Login => load_session_or_login(&client).and(Ok(())),

        Command::Inbox => {
            let session = load_session_or_login(&client)?;
            let inbox = request::get_inbox_listing(&client, &session)?;
            for entry in inbox {
                println!("{} - {}", entry.sender_name, entry.subject);
            }
            Ok(())
        },

        Command::Logout => {
            let session = session::try_load()?
                .ok_or(Error::AppError("No session found".to_string()))?;
            request::revoke_auth_token(&client, session)?;
            session::delete_saved()
        },
    }
}

fn load_session_or_login(client: &request::Client) ->
    Result<session::Session, Error>
{
    let loaded = session::try_load()?;
    if let Some(session) = loaded {
        return Ok(session);
    }

    let auth_response = login(&client)?;
    let session = session::make(auth_response.access_token,
                                auth_response.id_token)?;
    session::save(&session)?;
    Ok(session)
}

fn login(client: &request::Client) ->
    Result<AuthTokenResponse, Error>
{
    let config = request::get_config(&client)?;
    let mut terminal = terminal::load()?;
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

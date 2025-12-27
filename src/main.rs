use clap::{Parser, Subcommand};

use kivinge::kivra::{error::Error, request, session};
use kivinge::{terminal, view};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[arg(long)]
    preview: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Login,
    List,
    Logout,
}

fn main() {
    let cli_args = CliArgs::parse();
    match run(cli_args) {
        Ok(()) => (),
        Err(err) => println!("Error: {err}"),
    }
}

fn run(cli_args: CliArgs) -> Result<(), Error> {
    let mut terminal = terminal::load()?;
    let client = request::client();
    match cli_args.command {
        Command::Login if cli_args.preview => view::login::test_render(&mut terminal),
        Command::Login => load_session_or_login(&client).and(Ok(())),

        Command::List => {
            let session = load_session_or_login(&client)?;
            let inbox = request::get_inbox_listing(&client, &session)?;
            for entry in inbox {
                println!("{} - {}", entry.sender_name, entry.subject);
            }
            Ok(())
        }

        Command::Logout => {
            let session =
                session::try_load()?.ok_or(Error::AppError("No session found".to_string()))?;
            request::revoke_auth_token(&client, session)?;
            session::delete_saved()
        }
    }
}

fn load_session_or_login(client: &request::Client) -> Result<session::Session, Error> {
    let loaded = session::try_load()?;
    if let Some(session) = loaded {
        return Ok(session);
    }

    let mut terminal = terminal::load()?;
    match view::login::show(&mut terminal, client)? {
        Some(auth_response) => {
            let session = session::make(auth_response.access_token, auth_response.id_token)?;
            session::save(&session)?;
            Ok(session)
        }
        None => {
            Err(Error::AppError("Login aborted".to_string()))
        }
    }
}

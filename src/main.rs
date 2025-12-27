use crossterm::event::{read, Event, KeyCode};
use std::{fs::File, io::Read};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{
    self,
    shells::{Bash, PowerShell, Zsh},
    Generator,
};
use kivinge::kivra::{client::{self, Client}, model, session};
use kivinge::{cli, error::Error, terminal, tui};

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
    #[command(about = "Generate shell completion script")]
    Completions {
        #[arg(value_enum)]
        shell: CompletionsShell,
    },
    #[command(about = "Log in to Kivra")]
    Login,
    #[command(about = "List all items in the inbox")]
    List,
    #[command(about = "View inbox item")]
    View {
        item_id: usize
    },
    #[command(about = "Log out from Kivra")]
    Logout,
}

#[derive(ValueEnum, Debug, Clone)]
enum CompletionsShell {
    Bash,
    PowerShell,
    Zsh,
}

fn main() {
    let cli_args = CliArgs::parse();
    match run(cli_args) {
        Ok(()) => (),
        Err(err) => println!("Error: {err}"),
    }
}

fn generate_completions<G: Generator>(gen: G) {
    let mut command = CliArgs::command();
    clap_complete::generate(gen, &mut command, "kivinge", &mut std::io::stdout());
}

fn run(cli_args: CliArgs) -> Result<(), Error> {
    let client = client::KivraClient::new();
    match cli_args.command {
        _ if cli_args.preview =>
            run_preview(cli_args),

        Command::Completions { shell: CompletionsShell::Bash } =>
            Ok(generate_completions(Bash)),
        Command::Completions { shell: CompletionsShell::PowerShell } =>
            Ok(generate_completions(PowerShell)),
        Command::Completions { shell: CompletionsShell::Zsh } =>
            Ok(generate_completions(Zsh)),

        Command::Login => load_session_or_login(&client).and(Ok(())),

        Command::List => {
            let session = load_session_or_login(&client)?;
            let inbox = client.get_inbox_listing(&session)?;
            cli::inbox::print(&inbox)?;
            Ok(())
        }

        Command::View { item_id: _item_id } => {
            // let session = load_session_or_login(&client)?;
            Ok(())
        }

        Command::Logout => {
            let session =
                session::try_load()?.ok_or(Error::AppError("No session found".to_string()))?;
            client.revoke_auth_token(&session)?;
            session::delete_saved()
        }
    }
}

fn run_preview(cli_args: CliArgs) -> Result<(), Error> {
    let mut terminal = terminal::load()?;
    match cli_args.command {
        Command::Login => {
            let mut qr_code = String::new();
            File::open("./test_data/qrcode")?.read_to_string(&mut qr_code)?;
            tui::login::render(&mut terminal, &qr_code)
        }

        Command::List => {
            let file = File::open("./test_data/listing")?;
            let listing: Vec<model::ContentSpec> = serde_json::from_reader(file)?;
            tui::inbox::show(&mut terminal, &listing)
        }

        Command::View { item_id } => {
            let inbox_file = File::open("./test_data/listing")?;
            let listing: Vec<model::ContentSpec> = serde_json::from_reader(inbox_file)?;
            let spec = listing.get(item_id).ok_or(Error::AppError("Does not exist".to_string()))?;
            let details_file = File::open("./test_data/item")?;
            let details: model::ContentDetails = serde_json::from_reader(details_file)?;
            tui::content::show(&mut terminal, &spec, &details)?;
            Ok(())
        }

        _ => Err(Error::AppError("There is no preview for that command".to_string())),
    }?;

    loop {
        match read()? {
            Event::Key(key) if key.code == KeyCode::Char('q') => return Ok(()),
            _ => ()
        }
    }
}

fn load_session_or_login(client: &impl Client) -> Result<session::Session, Error> {
    let loaded = session::try_load()?;
    if let Some(session) = loaded {
        return Ok(session);
    }

    let mut terminal = terminal::load()?;
    match tui::login::show(&mut terminal, client)? {
        Some(auth_response) => {
            let session = session::make(auth_response.access_token, auth_response.id_token)?;
            session::save(&session)?;
            Ok(session)
        }
        None => Err(Error::AppError("Login aborted".to_string())),
    }
}

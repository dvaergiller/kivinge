use bytes::Bytes;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{
    self,
    shells::{Bash, PowerShell, Zsh},
    Generator,
};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use kivinge::{
    cli,
    client::{self, session, Client},
    error::Error,
    model::content::{InboxEntry, InboxListing},
    tui,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[arg(long)]
    mock: bool,

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
    View { item_id: u32 },

    #[command(about = "Download attachment")]
    Download {
        item_id: u32,
        attachment_num: u32,
        #[arg(default_value = ".")]
        download_dir: PathBuf,
    },

    #[command(about = "Open attachment")]
    Open { item_id: u32, attachment_num: u32 },

    #[command(about = "Log out from Kivra")]
    Logout,

    #[command(about = "Start interactive terminal UI")]
    Tui,
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
    let client: Box<dyn Client> = if cli_args.mock {
        Box::new(client::MockClient::default())
    } else {
        Box::new(client::KivraClient::default())
    };

    match cli_args.command {
        Command::Completions { shell } => {
            match shell {
                CompletionsShell::Bash => generate_completions(Bash),
                CompletionsShell::PowerShell => generate_completions(PowerShell),
                CompletionsShell::Zsh => generate_completions(Zsh),
            }
            Ok(())
        }

        Command::Login => load_session_or_login(&client).and(Ok(())),

        Command::List => {
            let session = load_session_or_login(&client)?;
            let inbox = client.get_inbox_listing(&session)?;
            println!("{}", cli::inbox::format(inbox));
            Ok(())
        }

        Command::View { item_id } => {
            let session = load_session_or_login(&client)?;
            let inbox = client.get_inbox_listing(&session)?;
            let entry = get_entry_by_id(inbox, item_id)?;
            let details = client.get_item_details(&session, &entry.item.key)?;
            println!("{}", cli::inbox_item::format(details)?);
            Ok(())
        }

        Command::Download {
            item_id,
            attachment_num,
            download_dir,
        } => {
            let full_path = download_attachment(&client, item_id, attachment_num, download_dir)?;
            Ok(println!("{}", full_path.to_string_lossy()))
        }

        Command::Open {
            item_id,
            attachment_num,
        } => {
            let tmp_dir = std::env::temp_dir();
            let full_path = download_attachment(&client, item_id, attachment_num, tmp_dir)?;
            opener::open(full_path)?;
            Ok(())
        }

        Command::Logout => {
            let session =
                session::try_load()?.ok_or(Error::UserError("No session found".to_string()))?;
            client.revoke_auth_token(&session)?;
            session::delete_saved()
        }

        Command::Tui => {
            let session = load_session_or_login(&client)?;
            let inbox = client.get_inbox_listing(&session)?;
            let mut terminal = tui::terminal::load()?;
            tui::inbox::show(&client, &session, &mut terminal, inbox)?;
            Ok(())
        }
    }
}

fn load_session_or_login(client: &impl Client) -> Result<session::Session, Error> {
    let loaded = session::try_load()?;
    if let Some(session) = loaded {
        return Ok(session);
    }

    let mut terminal = tui::terminal::load()?;
    match tui::login::show(&mut terminal, client)? {
        Some(auth_response) => {
            let session = session::make(auth_response.access_token, auth_response.id_token)?;
            session::save(&session)?;
            Ok(session)
        }
        None => Err(Error::AppError("Login aborted".to_string())),
    }
}

fn get_entry_by_id(inbox: InboxListing, item_id: u32) -> Result<InboxEntry, Error> {
    inbox
        .into_iter()
        .find(|i| i.id == item_id)
        .ok_or(Error::UserError(format!(
            "Inbox item {item_id} does not exist"
        )))
}

fn get_attachment_body(
    client: &impl Client,
    session: &session::Session,
    item_id: u32,
    attachment_num: u32,
) -> Result<Bytes, Error> {
    let inbox = client.get_inbox_listing(session)?;
    let entry = get_entry_by_id(inbox, item_id)?;
    let details = client.get_item_details(session, &entry.item.key)?;
    let attachment = details
        .parts
        .get(attachment_num as usize)
        .ok_or(Error::UserError(format!(
            "Inbox item {item_id} has no attachment number {attachment_num}"
        )))?;
    match (&attachment.key, &attachment.body) {
        (None, None) => Err(Error::AppError(
            "Attachment has no attachment key nor inline body".to_string(),
        )),
        (Some(key), _) => client.download_attachment(session, &entry.item.key, key),
        (_, Some(body)) => Ok(Bytes::copy_from_slice(body.as_bytes())),
    }
}

fn download_attachment(
    client: &impl Client,
    item_id: u32,
    attachment_num: u32,
    download_dir: PathBuf,
) -> Result<PathBuf, Error> {
    let session = load_session_or_login(client)?;
    let inbox = client.get_inbox_listing(&session)?;
    let entry = get_entry_by_id(inbox, item_id)?;
    let details = client.get_item_details(&session, &entry.item.key)?;
    let file = get_attachment_body(client, &session, item_id, attachment_num)?;
    let filename = details.attachment_name(attachment_num as usize)?;
    let full_path = Path::new(&download_dir).join(filename);
    File::create_new(&full_path)?.write_all(&file)?;
    Ok(full_path)
}

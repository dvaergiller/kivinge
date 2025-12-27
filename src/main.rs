use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{
    self,
    shells::{Bash, PowerShell, Zsh},
    Generator,
};
use std::path::PathBuf;

use kivinge::{
    cli,
    client::{self, session, Client},
    error::Error,
    tui,
    util::{
        download_attachment, get_entry_by_id, load_session_or_login,
        open_attachment,
    },
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
    clap_complete::generate(
        gen,
        &mut command,
        "kivinge",
        &mut std::io::stdout(),
    );
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
                CompletionsShell::PowerShell => {
                    generate_completions(PowerShell)
                }
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

        Command::Download { item_id, attachment_num, download_dir } => {
            let session = load_session_or_login(&client)?;
            let inbox = client.get_inbox_listing(&session)?;
            let entry = get_entry_by_id(inbox, item_id)?;
            let full_path = download_attachment(
                &client,
                &session,
                &entry.item,
                attachment_num,
                download_dir,
            )?;
            Ok(println!("{}", full_path.to_string_lossy()))
        }

        Command::Open { item_id, attachment_num } => {
            let session = load_session_or_login(&client)?;
            let inbox = client.get_inbox_listing(&session)?;
            let entry = get_entry_by_id(inbox, item_id)?;
            open_attachment(&client, &session, &entry.item, attachment_num)
        }

        Command::Logout => {
            let session = session::try_load()?
                .ok_or(Error::UserError("No session found"))?;
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

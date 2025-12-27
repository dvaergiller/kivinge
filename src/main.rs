use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{
    self,
    shells::{Bash, PowerShell, Zsh},
    Generator,
};
use fork::Fork;
use std::{fs::File, path::PathBuf};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};

use kivinge::{
    cli,
    client::{self, session, Client},
    error::Error,
    fuse,
    model::content::InboxItem,
    tui::{self, inbox_item::ItemViewResult, terminal::LoadedTerminal},
    util::{download_attachment, get_entry_by_id, open_attachment},
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

    #[command(about = "Mount inbox as FUSE filesystem")]
    Mount {
        mountpoint: PathBuf,
        #[arg(short = 'o', default_value = "")]
        mount_opts: String,
    },
}

#[derive(ValueEnum, Debug, Clone)]
enum CompletionsShell {
    Bash,
    PowerShell,
    Zsh,
}

fn main() -> Result<(), Error> {
    let cli_args = CliArgs::parse();
    match maybe_fork(cli_args) {
        Ok(None) => Ok(()),
        Ok(Some(output)) => {
            println!("{output}");
            Ok(())
        }
        Err(Error::ClientError(client::Error::LoginAborted)) => {
            println!("Login aborted");
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn maybe_fork(cli_args: CliArgs) -> Result<Option<String>, Error> {
    if let Command::Mount { .. } = cli_args.command {
        if let Fork::Parent(_) = fork::daemon(true, false)? {
            return Ok(None);
        }
    }
    run(cli_args)
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

fn run(cli_args: CliArgs) -> Result<Option<String>, Error> {
    let logpath = dirs::state_dir().unwrap_or(".".into()).join("kivinge.log");
    let logfile = File::options().append(true).create(true).open(logpath)?;
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(logfile.try_clone()?)
                .with_span_events(FmtSpan::ENTER),
        )
        .with(EnvFilter::from_env("LOGLEVEL"))
        .init();

    let mut client: Box<dyn Client> = if cli_args.mock {
        Box::new(client::MockClient::default())
    } else {
        Box::new(client::KivraClient::new()?)
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
            Ok(None)
        }

        Command::Login => {
            client.revoke_auth_token()?;
            client.login()?;
            Ok(Some("Login Successful".to_string()))
        }

        Command::List => {
            let inbox = client.get_inbox_listing()?;
            Ok(Some(cli::inbox::format(inbox)))
        }

        Command::View { item_id } => {
            let inbox = client.get_inbox_listing()?;
            let entry = get_entry_by_id(inbox, item_id)?;
            let details = client.get_item_details(&entry.item.key)?;
            Ok(Some(cli::inbox_item::format(details)?))
        }

        Command::Download { item_id, attachment_num, download_dir } => {
            let inbox = client.get_inbox_listing()?;
            let entry = get_entry_by_id(inbox, item_id)?;
            let full_path = download_attachment(
                &mut client,
                &entry.item,
                attachment_num,
                download_dir,
            )?;
            Ok(Some(full_path.to_string_lossy().to_string()))
        }

        Command::Open { item_id, attachment_num } => {
            let inbox = client.get_inbox_listing()?;
            let entry = get_entry_by_id(inbox, item_id)?;
            open_attachment(&mut client, &entry.item, attachment_num)?;
            Ok(None)
        }

        Command::Logout => {
            client.revoke_auth_token()?;
            session::delete_saved()?;
            Ok(Some("Session token deleted".to_string()))
        }

        Command::Tui => {
            let mut terminal = tui::terminal::load()?;
            show_inbox_tui(&mut terminal, &mut client)?;
            Ok(None)
        }

        Command::Mount { mountpoint, .. } => {
            client.get_session_or_login()?;
            fuse::mount(client, mountpoint.as_path())?;
            Ok(None)
        }
    }
}

fn show_inbox_tui(
    terminal: &mut LoadedTerminal,
    client: &mut impl Client,
) -> Result<(), Error> {
    loop {
        let user_info = client.get_session().map(|s| s.user_info);
        let mut inbox_view = tui::inbox::InboxView::make(client)?;
        let ret = tui::show(&mut inbox_view, terminal, user_info)?;
        match ret {
            Some(entry) => {
                show_inbox_item_tui(terminal, client, entry.item)?;
            }

            None => return Ok(()),
        }
    }
}

fn show_inbox_item_tui(
    terminal: &mut LoadedTerminal,
    client: &mut impl Client,
    item: InboxItem,
) -> Result<(), Error> {
    let mut entry_view = tui::inbox_item::ItemView::make(client, item.clone())?;
    loop {
        let user_info = client.get_session().map(|s| s.user_info);
        let ret = tui::show(&mut entry_view, terminal, user_info)?;
        match ret {
            ItemViewResult::Close => return Ok(()),
            ItemViewResult::MarkRead => {
                client.mark_as_read(&item.key)?;
            }
            ItemViewResult::Open(attachment_num) => {
                open_attachment(client, &item, attachment_num)?;
            }
        }
    }
}

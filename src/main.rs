use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{self, shells::{Bash, PowerShell, Zsh}, Generator};
use kivinge::kivra::{request, session};
use kivinge::{error::Error, terminal, cli, tui};

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
    let client = request::client();
    match cli_args.command {
        Command::Login if cli_args.preview => {
            let mut terminal = terminal::load()?;
            tui::login::test_render(&mut terminal)
        }

        Command::List if cli_args.preview => {
            let mut terminal = terminal::load()?;
            tui::login::test_render(&mut terminal)
        }

        Command::Completions { shell: CompletionsShell::Bash } =>
            Ok(generate_completions(Bash)),
        Command::Completions { shell: CompletionsShell::PowerShell } =>
            Ok(generate_completions(PowerShell)),
        Command::Completions { shell: CompletionsShell::Zsh } =>
            Ok(generate_completions(Zsh)),

        Command::Login => load_session_or_login(&client).and(Ok(())),

        Command::List => {
            let session = load_session_or_login(&client)?;
            let inbox = request::get_inbox_listing(&client, &session)?;
            cli::inbox::print(&inbox)?;
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
    match tui::login::show(&mut terminal, client)? {
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

rust_i18n::i18n!("locales", fallback = "en");

mod actions;
mod app;
mod git;
mod github;
mod gpg;
mod i18n;
mod models;
mod process;
mod release;
mod storage;
mod ui;

use clap::{Parser, Subcommand};
use crossterm::{cursor::Hide, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use std::io;

#[derive(Parser)]
#[command(name = "ghcap", version, about = "Cross-platform GitHub commit/push workflow TUI")]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Ask the running ghcap instance to stop Commit.
    Commitstop,
    /// Ask the running ghcap instance to stop Push.
    Pushstop,
    /// Ask the running ghcap instance to stop Commit & Push.
    Commitpushstop,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ghcap: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if let Some(command) = cli.command {
        let name = match command {
            CliCommand::Commitstop => "commitstop",
            CliCommand::Pushstop => "pushstop",
            CliCommand::Commitpushstop => "commitpushstop",
        };
        std::process::exit(storage::send_stop(name));
    }

    let config = storage::load_config()?;
    storage::save_config(&config)?;
    storage::init_instance()?;

    let setup = enable_raw_mode().and_then(|_| execute!(io::stdout(), EnterAlternateScreen, Hide));
    if let Err(error) = setup {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        storage::clear_instance();
        anyhow::bail!("terminal setup failed: {error}");
    }

    let result = match ui::terminal() {
        Ok(mut terminal) => app::run(config, &mut terminal).map_err(anyhow::Error::from),
        Err(error) => Err(anyhow::Error::from(error)),
    };

    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
    storage::clear_instance();
    result
}

mod actions;
mod app;
mod git;
mod github;
mod gpg;
mod i18n;
mod models;
mod storage;
mod ui;

use crossterm::{cursor::Hide, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use std::io;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(command) = args.get(1) {
        let code = match command.as_str() {
            "commitstop" | "pushstop" | "commitpushstop" => storage::send_stop(command),
            "--version" | "-V" => { println!("ghcap 0.13.0"); 0 }
            "--help" | "-h" => { println!("ghcap 0.13.0\nUsage: ghcap [commitstop|pushstop|commitpushstop]"); 0 }
            _ => { eprintln!("Unknown command: {command}"); 2 }
        };
        std::process::exit(code);
    }

    let config = match storage::load_config() {
        Ok(c) => c,
        Err(e) => { eprintln!("ghcap: cannot load configuration: {e}"); return; }
    };
    if let Err(e) = storage::save_config(&config) {
        eprintln!("ghcap: cannot save configuration: {e}"); return;
    }
    if let Err(e) = storage::init_instance() {
        eprintln!("ghcap: cannot initialize runtime state: {e}"); return;
    }

    let setup = enable_raw_mode().and_then(|_| execute!(io::stdout(), EnterAlternateScreen, Hide));
    if let Err(e) = setup {
        storage::clear_instance();
        eprintln!("ghcap: terminal setup failed: {e}");
        return;
    }

    let result = match ui::terminal() {
        Ok(mut terminal) => app::run(config, &mut terminal),
        Err(e) => Err(e),
    };

    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
    storage::clear_instance();

    if let Err(e) = result { eprintln!("ghcap error: {e}"); }
}

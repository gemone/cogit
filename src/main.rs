use std::io;
use std::path::PathBuf;

use clap::Parser;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod config;
mod gitops;
mod panels;
mod vimkeys;

use app::App;
use gitops::Repo;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'C', long = "repo", default_value = ".")]
    repo: PathBuf,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let repo = Repo::open(&cli.repo)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = (|| -> anyhow::Result<()> {
        let mut app = App::new(repo)?;
        app.run(&mut terminal)?;
        Ok(())
    })();

    // Always restore terminal
    let mut restore = || -> anyhow::Result<()> {
        disable_raw_mode()?;
        terminal.backend_mut().execute(LeaveAlternateScreen)?;
        Ok(())
    };

    if let Err(e) = restore() {
        eprintln!("Failed to restore terminal: {}", e);
    }

    result
}

use clap::Parser;
use std::path::PathBuf;

mod config;
mod gitops;

use gitops::repo::Repo;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'C', long = "repo", default_value = ".")]
    repo: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let repo = Repo::open(&cli.repo)?;

    if let Some(head) = repo.head_shorthand() {
        println!("HEAD: {}", head);
    } else {
        println!("HEAD: (detached)");
    }

    let status = repo.status()?;
    println!("Status entries: {}", status.len());

    let _config = config::CogitConfig::load()?;

    Ok(())
}

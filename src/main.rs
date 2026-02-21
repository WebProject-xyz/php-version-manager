mod cli;
mod commands;
mod fs;
mod interactive;
mod network;
mod shell;
mod update;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use colored::Colorize;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let _pvm_dir = fs::get_pvm_dir()?;
    let versions_dir = fs::get_versions_dir()?;
    std::fs::create_dir_all(&versions_dir)?;

    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        cmd.call().await?;
    } else if let Err(e) = interactive::run_root_menu().await {
        eprintln!("{} Error: {}", "✗".red(), e);
        std::process::exit(1);
    }

    Ok(())
}

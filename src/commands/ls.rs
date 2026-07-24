use crate::commands::use_cmd::{ActivateOpts, activate, pick_installed_version};
use crate::fs;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::io::IsTerminal;

/// List all locally installed PHP versions (interactively switch on a terminal)
#[derive(Parser, Debug)]
pub struct Ls;

impl Ls {
    pub async fn call(self) -> Result<()> {
        let items = fs::get_aliased_versions()?;
        if items.is_empty() {
            println!("No PHP versions installed.");
            return Ok(());
        }

        // Scripts and pipes get the plain list; a terminal gets a picker that
        // switches the shell to the selected version (Esc just leaves).
        if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
            let current = fs::get_current_version();
            for item in items {
                let pkgs_str = item.packages.join(", ");
                if item.version == current {
                    println!(
                        "* {} {} [{}]",
                        item.display.cyan().bold(),
                        "(current)".cyan(),
                        pkgs_str.cyan()
                    );
                } else {
                    println!("  {} [{}]", item.display, pkgs_str.cyan());
                }
            }
            return Ok(());
        }

        let current = fs::get_current_version();
        match pick_installed_version("Installed PHP versions — Enter switches, Esc exits")? {
            Some(version) if version == current => {
                eprintln!("{} PHP {} is already active.", "✓".green(), version.bold());
                Ok(())
            }
            Some(version) => activate(version, ActivateOpts::picker(false)).await,
            None => Ok(()),
        }
    }
}

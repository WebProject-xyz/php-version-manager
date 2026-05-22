use crate::fs;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::io::IsTerminal;

use dialoguer::{Select, theme::ColorfulTheme};

/// Uninstall a specific PHP version
#[derive(Parser, Debug)]
pub struct Uninstall {
    /// The version to uninstall
    pub version: Option<String>,

    /// Auto-approve the uninstallation without prompting
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

impl Uninstall {
    pub async fn call(self) -> Result<()> {
        let version = match self.version {
            Some(ref v) => fs::resolve_local_version(v)?,
            None => {
                let items = fs::get_aliased_versions()?;
                if items.is_empty() {
                    println!("{} No PHP versions are currently installed.", "💡".yellow());
                    return Ok(());
                }

                let displays: Vec<String> = items.iter().map(|i| i.display.clone()).collect();
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select a PHP version to uninstall")
                    .default(0)
                    .items(&displays)
                    .interact_opt()?;

                match selection {
                    Some(idx) => items[idx].version.clone(),
                    None => {
                        println!("{} Operation cancelled.", "✗".red());
                        return Ok(());
                    }
                }
            }
        };

        let dest = fs::get_versions_dir()?.join(&version);
        if !dest.exists() {
            anyhow::bail!("PHP {} is not installed locally.", version);
        }

        let current = fs::get_current_version();
        if version == current {
            println!(
                "{} Warning: You are uninstalling the currently active PHP version ({})",
                "⚠".yellow(),
                version
            );
        }

        let is_tty = std::io::stdin().is_terminal();
        if !self.yes && is_tty {
            let prompt = format!("Are you sure you want to uninstall PHP {}?", version);
            let confirmed = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt.bold().to_string())
                .default(true)
                .interact_opt()?
                .unwrap_or(false);

            if !confirmed {
                println!("{} Operation cancelled.", "✗".red());
                return Ok(());
            }
        }

        println!("{} Removing PHP {}...", "↻".blue(), version);
        match std::fs::remove_dir_all(&dest) {
            Ok(_) => {
                println!("{} Successfully uninstalled PHP {}", "✓".green(), version);
            }
            Err(e) => {
                anyhow::bail!("Failed to uninstall PHP {}: {}", version, e);
            }
        }

        Ok(())
    }
}

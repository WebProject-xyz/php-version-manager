use crate::fs;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use dialoguer::{Select, theme::ColorfulTheme};

/// Uninstall a specific PHP version
#[derive(Parser, Debug)]
pub struct Uninstall {
    /// The version to uninstall
    pub version: Option<String>,
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

        if !fs::is_version_installed(&version)? {
            anyhow::bail!("PHP {} is not installed.", version);
        }

        let current = fs::get_current_version();
        if version == current {
            println!(
                "{} Warning: You are uninstalling the currently active PHP version ({})",
                "⚠".yellow(),
                version
            );
        }

        let dest = fs::get_versions_dir()?.join(&version);

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

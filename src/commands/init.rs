use crate::network;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use std::collections::HashSet;

/// Initialize a .php-version file in the current directory
#[derive(Parser, Debug)]
pub struct Init;

impl Init {
    pub async fn call(self) -> Result<()> {
        println!("{} Fetching remotely available PHP versions...", "↻".blue());
        let all_versions = network::get_available_versions().await?;

        // Extract just the major.minor (e.g., "8.4" or "7.4")
        let mut major_minors = HashSet::new();
        let mut options = Vec::new();

        for (v, _) in all_versions.iter().rev() {
            // Start from newest
            let parts: Vec<&str> = v.split('.').collect();
            if parts.len() >= 2 {
                let mm = format!("{}.{}", parts[0], parts[1]);
                if major_minors.insert(mm.clone()) {
                    options.push(mm);
                }
            }
        }

        if options.is_empty() {
            anyhow::bail!("Could not retrieve PHP versions.");
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a PHP version for this directory")
            .default(0)
            .items(&options)
            .interact_opt()?;

        let selected = match selection {
            Some(idx) => &options[idx],
            None => {
                println!("{} Operation cancelled.", "✗".red());
                return Ok(());
            }
        };

        std::fs::write(".php-version", selected)?;
        println!("{} Wrote {} to .php-version", "✓".green(), selected.bold());

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Do you want to run `pvm use {}` now?", selected))
            .default(true)
            .interact_opt()?
            .unwrap_or(false)
        {
            // Call use programmatically
            let use_cmd = crate::commands::use_cmd::Use {
                version: Some(selected.clone()),
            };
            use_cmd.call().await?;
        }

        Ok(())
    }
}

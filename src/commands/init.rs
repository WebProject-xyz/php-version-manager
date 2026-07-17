use crate::constants::PHP_VERSION_FILE;
use crate::{fs, network, prompt};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Select, theme::ColorfulTheme};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::Path;

/// Initialize a .php-version file in the current directory
#[derive(Parser, Debug)]
pub struct Init;

impl Init {
    pub async fn call(self) -> Result<()> {
        // The whole flow is interactive (overwrite confirm + picker), so
        // fail early with a hint instead of a cryptic dialoguer error.
        if !std::io::stdin().is_terminal() {
            anyhow::bail!(
                "pvm init requires a terminal; write {} yourself instead.",
                PHP_VERSION_FILE
            );
        }

        if Path::new(PHP_VERSION_FILE).exists() {
            let existing = std::fs::read_to_string(PHP_VERSION_FILE)?;
            let question = format!(
                "A {} file already exists ({}). Overwrite?",
                PHP_VERSION_FILE,
                existing.trim().yellow()
            );
            if !prompt::confirm(&question, false, false)? {
                println!("{} Operation cancelled.", "✗".red());
                return Ok(());
            }
        }

        // Distinct major.minor lines: locally installed first (newest first,
        // marked), then the remote-only ones (newest first).
        let mut seen = HashSet::new();
        let mut options = Vec::new(); // what gets written to .php-version
        let mut displays = Vec::new(); // what the picker shows

        for v in fs::list_installed_versions()?.iter().rev() {
            if let Some(mm) = major_minor(v)
                && seen.insert(mm.clone())
            {
                displays.push(format!("{} {}", mm, "(installed)".green()));
                options.push(mm);
            }
        }

        println!("{} Fetching remotely available PHP versions...", "↻".blue());
        match network::get_available_versions().await {
            Ok(remote) => {
                for (v, _) in remote.iter().rev() {
                    // Start from newest
                    if let Some(mm) = major_minor(v)
                        && seen.insert(mm.clone())
                    {
                        displays.push(mm.clone());
                        options.push(mm);
                    }
                }
            }
            Err(e) => {
                println!(
                    "{} Could not fetch remote versions ({}). Showing installed versions only.",
                    "💡".yellow(),
                    e
                );
            }
        }

        if options.is_empty() {
            anyhow::bail!("Could not retrieve PHP versions.");
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a PHP version for this directory")
            .default(0)
            .items(&displays)
            .interact_opt()?;

        let selected = match selection {
            Some(idx) => &options[idx],
            None => {
                println!("{} Operation cancelled.", "✗".red());
                return Ok(());
            }
        };

        std::fs::write(PHP_VERSION_FILE, selected)?;
        println!(
            "{} Wrote {} to {}",
            "✓".green(),
            selected.bold(),
            PHP_VERSION_FILE
        );

        let question = format!("Do you want to run 'pvm use {}' now?", selected);
        if prompt::confirm(&question, true, false)? {
            // Call use programmatically
            let use_cmd = crate::commands::use_cmd::Use {
                version: Some(selected.clone()),
                silent: false,
                yes: false,
            };
            use_cmd.call().await?;
        }

        Ok(())
    }
}

/// "8.4.18" -> "8.4"; None when there is no minor component.
fn major_minor(v: &str) -> Option<String> {
    let mut parts = v.split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => Some(format!("{}.{}", major, minor)),
        _ => None,
    }
}

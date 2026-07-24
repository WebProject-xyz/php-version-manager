use crate::fs;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::IsTerminal;

/// Set the default PHP version for new shells (evaluated by 'pvm env')
#[derive(Parser, Debug)]
pub struct DefaultCmd {
    /// The version to use as default, "system" to clear (omit for interactive list)
    pub version: Option<String>,
}

impl DefaultCmd {
    pub async fn call(self) -> Result<()> {
        let version = match self.version {
            Some(ref v) if v == "system" => {
                fs::clear_default_version()?;
                println!(
                    "{} Default cleared. New shells start on the system PHP.",
                    "✓".green()
                );
                return Ok(());
            }
            Some(ref v) => fs::resolve_local_version(v)?,
            None => {
                if !std::io::stdin().is_terminal() {
                    match fs::get_default_version()? {
                        Some(v) => println!("{}", v),
                        None => println!("none"),
                    }
                    return Ok(());
                }

                let items = fs::get_aliased_versions()?;
                if items.is_empty() {
                    eprintln!("{} No PHP versions are currently installed.", "💡".yellow());
                    return Ok(());
                }

                let current_default = fs::get_default_version()?.unwrap_or_default();
                let displays: Vec<String> = items
                    .iter()
                    .map(|i| {
                        if i.version == current_default {
                            format!("{} {}", i.display, "(default)".cyan())
                        } else {
                            i.display.clone()
                        }
                    })
                    .collect();

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select the default PHP version for new shells")
                    .default(0)
                    .items(&displays)
                    .interact_opt()?;

                match selection {
                    Some(idx) => items[idx].version.clone(),
                    None => {
                        eprintln!("{} Operation cancelled.", "✗".red());
                        return Ok(());
                    }
                }
            }
        };

        fs::set_default_version(&version)?;
        println!(
            "{} Default version set to {}. New shells pick it up via 'pvm env'.",
            "✓".green(),
            version.bold()
        );
        Ok(())
    }
}

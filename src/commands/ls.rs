use crate::fs;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

/// List all locally installed PHP versions
#[derive(Parser, Debug)]
pub struct Ls;

impl Ls {
    pub async fn call(self) -> Result<()> {
        let items = fs::get_aliased_versions()?;
        let current = fs::get_current_version();

        if items.is_empty() {
            println!("No PHP versions installed.");
        } else {
            for item in items {
                let pkgs_str = item.packages.join(", ");
                if item.version == current {
                    println!("* {} {} [{}]", item.display.cyan().bold(), "(current)".cyan(), pkgs_str.cyan());
                } else {
                    println!("  {} [{}]", item.display, pkgs_str.cyan());
                }
            }
        }

        Ok(())
    }
}

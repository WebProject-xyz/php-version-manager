use crate::{fs, network};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

/// List all remote available PHP versions from static-php-cli
#[derive(Parser, Debug)]
pub struct LsRemote {
    /// Optional version prefix to filter (e.g., '8.2', '8', '8.4.1')
    pub version_prefix: Option<String>,
}

impl LsRemote {
    pub async fn call(self) -> Result<()> {
        let mut versions_info = network::get_available_versions().await?;

        if let Some(prefix) = &self.version_prefix {
            let prefix_dot = format!("{}.", prefix);
            versions_info.retain(|(v, _)| v == prefix || v.starts_with(&prefix_dot));
        }

        if versions_info.is_empty() {
            println!("{} No remote versions found.", "💡".yellow());
            return Ok(());
        }

        let installed = fs::list_installed_versions().unwrap_or_default();

        // Ascending order: the newest versions print last, right above the prompt.
        for (v, pkgs) in &versions_info {
            let pkgs_str = pkgs.join(", ");
            if installed.contains(v) {
                println!(
                    "{} {} {} [{}]",
                    "✓".green(),
                    v,
                    "(installed)".dimmed(),
                    pkgs_str.cyan()
                );
            } else {
                println!("  {} [{}]", v, pkgs_str.cyan());
            }
        }

        Ok(())
    }
}

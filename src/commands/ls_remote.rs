use crate::{fs, network};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Select, theme::ColorfulTheme};

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
            versions_info.retain(|(v, _)| v.starts_with(prefix) || v == prefix);
        }

        if versions_info.is_empty() {
            println!("{} No remote versions found.", "💡".yellow());
            return Ok(());
        }

        let installed = fs::list_installed_versions().unwrap_or_default();

        let mut display_items = Vec::new();
        let mut target_versions = Vec::new(); // Parallel array tying display index to actual version string

        // Build "Quick Select" aliases
        let mut minors = std::collections::BTreeMap::new();
        let mut highest_overall = None;

        for (v, _) in &versions_info {
            highest_overall = Some(v.clone());
            let parts: Vec<&str> = v.split('.').collect();
            if parts.len() >= 2 {
                let minor = format!("{}.{}", parts[0], parts[1]);
                minors.insert(minor, v.clone()); // BTreeMap iterates ascending, keeps latest
            }
        }

        if let Some(highest) = highest_overall {
            display_items.push(format!("latest ({})", highest).bold().to_string());
            target_versions.push(highest);
        }

        // Add them in reverse order (newest minor first) for the quick select
        for (minor, highest_patch) in minors.iter().rev() {
            display_items.push(
                format!("{} ({})", minor, highest_patch)
                    .bold()
                    .cyan()
                    .to_string(),
            );
            target_versions.push(highest_patch.clone());
        }

        display_items.push("---".dimmed().to_string());
        target_versions.push("".to_string()); // Unselectable divider

        // Build the rest of the flat list
        for (v, pkgs) in versions_info.iter().rev() {
            let pkgs_str = pkgs.join(", ");
            if installed.contains(v) {
                display_items.push(format!("{} {} {} [{}]", "✓".green(), v, "(installed)".dimmed(), pkgs_str.cyan()));
            } else {
                display_items.push(format!("  {} [{}]", v, pkgs_str.cyan()));
            }
            target_versions.push(v.clone());
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a PHP version to install")
            .default(0)
            .items(&display_items)
            .interact_opt()?;

        let selected = match selection {
            Some(idx) => {
                let target = &target_versions[idx];
                if target.is_empty() {
                    // They clicked the divider
                    println!("{} Invalid selection.", "✗".red());
                    return Ok(());
                }
                target
            }
            None => {
                println!("{} Operation cancelled.", "✗".red());
                return Ok(());
            }
        };

        if !installed.contains(selected) {
            crate::commands::install::execute_install(selected).await?;
        } else {
            println!(
                "{} PHP {} is already installed.",
                "✓".green(),
                selected.bold()
            );
        }

        Ok(())
    }
}

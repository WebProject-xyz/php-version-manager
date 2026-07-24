use crate::fs;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::collections::BTreeMap;

/// Remove superseded patch versions
#[derive(Parser, Debug)]
pub struct Prune {
    /// Auto-approve the removal without prompting
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

impl Prune {
    pub async fn call(self) -> Result<()> {
        let installed = fs::list_installed_versions()?;
        let current = fs::get_current_version();

        // Keeper per major.minor line: the newest patch. The installed list
        // is semver-sorted ascending, so the last insert per minor wins.
        let mut keepers: BTreeMap<String, String> = BTreeMap::new();
        for version in &installed {
            if let Some(minor) = fs::minor_of(version) {
                keepers.insert(minor, version.clone());
            }
        }

        // Candidates: every installed patch that is not its minor's keeper,
        // except the currently active version.
        let candidates: Vec<(String, String)> = installed
            .iter()
            .filter_map(|version| {
                let keeper = keepers.get(&fs::minor_of(version)?)?;
                (version != keeper && *version != current)
                    .then(|| (version.clone(), keeper.clone()))
            })
            .collect();

        if candidates.is_empty() {
            println!("{} Nothing to prune.", "✓".green());
            return Ok(());
        }

        println!("The following superseded version(s) will be removed:");
        for (version, keeper) in &candidates {
            println!("  {} (superseded by {})", version, keeper.bold());
        }

        let question = format!("Remove {} superseded version(s)?", candidates.len());
        if !crate::prompt::confirm(&question.bold().to_string(), true, self.yes)? {
            println!("{} Operation cancelled.", "✗".red());
            return Ok(());
        }

        let versions_dir = fs::get_versions_dir()?;
        for (version, _) in &candidates {
            let dest = versions_dir.join(version);
            std::fs::remove_dir_all(&dest)
                .map_err(|e| anyhow::anyhow!("Failed to remove PHP {}: {}", version, e))?;
            println!("{} Removed PHP {}", "✓".green(), version);
        }

        // If the persisted default version was pruned, re-point it to the
        // keeper of the same minor line.
        if let Some(default) = fs::get_default_version()?
            && let Some((_, keeper)) = candidates.iter().find(|(v, _)| *v == default)
        {
            fs::set_default_version(keeper)?;
            println!(
                "{} Default version {} was pruned; re-pointed to {}.",
                "💡".yellow(),
                default,
                keeper.bold()
            );
        }

        Ok(())
    }
}

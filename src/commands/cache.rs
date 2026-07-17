use crate::fs;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

/// Manage the remote version cache
#[derive(Parser, Debug)]
pub struct Cache {
    #[command(subcommand)]
    action: CacheAction,
}

#[derive(Subcommand, Debug)]
enum CacheAction {
    /// Delete the cached remote version index
    Clear,
}

impl Cache {
    pub async fn call(self) -> Result<()> {
        match self.action {
            CacheAction::Clear => clear(),
        }
    }
}

fn clear() -> Result<()> {
    let pvm_dir = fs::get_pvm_dir()?;
    let mut removed: usize = 0;

    match std::fs::read_dir(&pvm_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if name.starts_with("remote_cache")
                    && name.ends_with(".json")
                    && entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                {
                    std::fs::remove_file(entry.path())
                        .with_context(|| format!("Failed to remove cache file '{}'", name))?;
                    removed += 1;
                }
            }
        }
        // No PVM dir yet means there is nothing cached — that is fine.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(e).context(format!(
                "Failed to read PVM directory '{}'",
                pvm_dir.display()
            ));
        }
    }

    println!(
        "{} Removed {} cached remote version index file{}",
        "✓".green(),
        removed,
        if removed == 1 { "" } else { "s" }
    );
    Ok(())
}

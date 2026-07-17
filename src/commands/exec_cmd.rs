use crate::fs;
use anyhow::{Context, Result};
use clap::Parser;

/// Run a command under a specific PHP version
#[derive(Parser, Debug)]
pub struct Exec {
    /// The PHP version to run under
    version: String,

    /// The command and arguments to execute
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    command: Vec<String>,
}

impl Exec {
    pub async fn call(self) -> Result<()> {
        let version = fs::resolve_local_version(&self.version)?;
        let bin_dir = fs::get_version_bin_dir(&version)?;

        // Prepend the version's bin dir so its binaries shadow any other PHP
        // already on PATH, for this single child process only.
        let existing = std::env::var_os("PATH").unwrap_or_default();
        let paths = std::iter::once(bin_dir).chain(std::env::split_paths(&existing));
        let new_path = std::env::join_paths(paths).context("Failed to construct PATH")?;

        let status = std::process::Command::new(&self.command[0])
            .args(&self.command[1..])
            .env("PATH", new_path)
            .status()
            .with_context(|| format!("Failed to execute '{}'", self.command[0]))?;

        std::process::exit(status.code().unwrap_or(1));
    }
}

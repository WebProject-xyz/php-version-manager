use crate::constants::{MULTISHELL_PATH_VAR, PHP_VERSION_FILE};
use crate::{fs, shell, update};
use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use std::path::Path;

/// Change PHP version
#[derive(Parser, Debug)]
pub struct Use {
    /// The version to use (omit for interactive list)
    pub version: Option<String>,

    /// Skip interactive prompts when the requested version is missing (used by shell hooks).
    #[arg(long, hide = true)]
    pub silent: bool,
}

impl Use {
    pub async fn call(self) -> Result<()> {
        let mut version = match self.version {
            Some(ref v) => match fs::try_resolve_local_version(v)? {
                Some(resolved) => resolved,
                None => {
                    if self.silent {
                        return Ok(());
                    }

                    let prompt = format!(
                        "PHP {} is not installed locally. Do you want to install it now?",
                        v.bold()
                    );
                    let install_now = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt(&prompt)
                        .default(true)
                        .interact_opt()?
                        .unwrap_or(false);

                    if !install_now {
                        eprintln!("{} Operation cancelled.", "✗".red());
                        return Ok(());
                    }

                    crate::commands::install::execute_install(v).await?;
                    return Ok(());
                }
            },
            None => {
                let items = fs::get_aliased_versions()?;
                if items.is_empty() {
                    eprintln!("{} No PHP versions are currently installed.", "💡".yellow());
                    return Ok(());
                }

                let displays: Vec<String> = items.iter().map(|i| i.display.clone()).collect();
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select a locally installed PHP version to use")
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

        if let Ok(Some(newer_version)) = update::check_for_updates(&version).await {
            let prompt = format!(
                "{} A new patch version is available: {} ➜ {}. Do you want to install and use it now?",
                "💡".yellow(),
                version.dimmed(),
                newer_version.green().bold()
            );

            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&prompt)
                .default(true)
                .interact_opt()?
                .unwrap_or(false)
            {
                let install_cmd = crate::commands::install::Install {
                    version: Some(newer_version.clone()),
                };
                install_cmd.call().await?;
                version = newer_version;
            }
        }

        if !fs::is_version_installed(&version)? {
            anyhow::bail!(
                "PHP {} is not installed. Run 'pvm install {}' first.",
                version,
                version
            );
        }

        // Smart prompt logic
        if Path::new(PHP_VERSION_FILE).exists()
            && let Ok(current_file_ver) = std::fs::read_to_string(PHP_VERSION_FILE)
            && current_file_ver.trim() != version
        {
            let prompt = format!(
                "A {} file is present ({}). Do you want to apply this change to the directory?",
                PHP_VERSION_FILE,
                current_file_ver.trim().yellow()
            );
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&prompt)
                .default(false)
                .interact_opt()?
                .unwrap_or(false)
            {
                std::fs::write(PHP_VERSION_FILE, &version)
                    .with_context(|| format!("Failed to update {}", PHP_VERSION_FILE))?;
                eprintln!(
                    "{} Updated {} to {}",
                    "✓".green(),
                    PHP_VERSION_FILE,
                    version.bold()
                );
            }
        }

        let bin_dir = fs::get_version_bin_dir(&version)?;
        let s = shell::detect_shell();

        // These evaluate in the user's shell hook via wrapper
        let export_str1 = s.set_env_var(MULTISHELL_PATH_VAR, &bin_dir.to_string_lossy());
        let export_str2 = s.path(&bin_dir);

        let env_file = fs::get_env_update_path(None)?;
        fs::write_env_file_locked(&env_file, &format!("{}\n{}", export_str1, export_str2))?;

        // Note: process-global env is intentionally NOT mutated here. std::env::set_var
        // is unsound in a multi-threaded tokio runtime, and the wrapper sources env_file
        // into the parent shell on exit, so subsequent pvm invocations see the new PATH.

        Ok(())
    }
}

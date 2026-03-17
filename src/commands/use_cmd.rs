use crate::constants::{MULTISHELL_PATH_VAR, PHP_VERSION_FILE};
use crate::{fs, shell, update};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use fs4::fs_std::FileExt;
use std::io::Write;
use std::path::Path;

/// Change PHP version
#[derive(Parser, Debug)]
pub struct Use {
    /// The version to use (omit for interactive list)
    pub version: Option<String>,
}

impl Use {
    pub async fn call(self) -> Result<()> {
        let mut version = match self.version {
            Some(ref v) => fs::resolve_local_version(v)?,
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
        if Path::new(PHP_VERSION_FILE).exists() {
            if let Ok(current_file_ver) = std::fs::read_to_string(PHP_VERSION_FILE) {
                if current_file_ver.trim() != version {
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
                        match std::fs::write(PHP_VERSION_FILE, &version) {
                            Ok(_) => eprintln!(
                                "{} Updated {} to {}",
                                "✓".green(),
                                PHP_VERSION_FILE,
                                version.bold()
                            ),
                            Err(e) => eprintln!(
                                "{} Failed to update {}: {}",
                                "✗".red(),
                                PHP_VERSION_FILE,
                                e
                            ),
                        }
                    }
                }
            }
        }

        let bin_dir = fs::get_version_bin_dir(&version)?;
        let s = shell::detect_shell();

        // These evaluate in the user's shell hook via wrapper
        let export_str1 = s.set_env_var(MULTISHELL_PATH_VAR, &bin_dir.to_string_lossy());
        let export_str2 = s.path(&bin_dir);

        let env_file = fs::get_env_update_path(None)?;

        // Atomic write with advisory lock
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&env_file)?;
        file.lock_exclusive()?;
        let mut writer = std::io::BufWriter::new(&file);
        writeln!(writer, "{}", export_str1)?;
        writeln!(writer, "{}", export_str2)?;
        writer.flush()?;
        fs4::fs_std::FileExt::unlock(&file)?;

        // Also update the current Rust binary's environment so spawned subs (or interactive loop) see it
        unsafe {
            std::env::set_var(MULTISHELL_PATH_VAR, &bin_dir);
            if let Some(path) = std::env::var_os("PATH") {
                let mut new_path = std::ffi::OsString::new();
                new_path.push(&bin_dir);
                #[cfg(windows)]
                new_path.push(";");
                #[cfg(not(windows))]
                new_path.push(":");
                new_path.push(&path);
                std::env::set_var("PATH", new_path);
            }
        }

        Ok(())
    }
}

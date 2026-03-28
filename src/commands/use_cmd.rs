use crate::{fs, shell, update};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
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
        if Path::new(".php-version").exists()
            && let Ok(current_file_ver) = std::fs::read_to_string(".php-version")
            && current_file_ver.trim() != version
        {
            let prompt = format!(
                "A .php-version file is present ({}). Do you want to apply this change to the directory?",
                current_file_ver.trim().yellow()
            );
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&prompt)
                .default(false)
                .interact_opt()?
                .unwrap_or(false)
            {
                std::fs::write(".php-version", &version).ok();
                eprintln!("{} Updated .php-version to {}", "✓".green(), version.bold());
            }
        }

        let bin_dir = fs::get_version_bin_dir(&version)?;
        let php_ini_path = fs::get_version_php_ini_path(&version).ok();
        let s = shell::detect_shell();

        // These evaluate in the user's shell hook via wrapper
        let export_str1 = s.set_env_var("PVM_MULTISHELL_PATH", &bin_dir.to_string_lossy());
        let export_str2 = s.path(&bin_dir);
        let export_str3 = php_ini_path
            .as_ref()
            .filter(|p| p.exists())
            .map(|p| s.set_env_var("PHPRC", &p.to_string_lossy()));

        let pvm_dir = fs::get_pvm_dir()?;
        let env_file = pvm_dir.join(".env_update");
        let mut exports = format!("{}\n{}", export_str1, export_str2);
        if let Some(line) = export_str3 {
            exports.push('\n');
            exports.push_str(&line);
        }
        std::fs::write(&env_file, exports).ok();

        // Also update the current Rust binary's environment so spawned subs (or interactive loop) see it
        unsafe {
            std::env::set_var("PVM_MULTISHELL_PATH", &bin_dir);
            if let Some(path) = std::env::var_os("PATH") {
                let mut new_path = std::ffi::OsString::new();
                new_path.push(&bin_dir);
                new_path.push(":");
                new_path.push(&path);
                std::env::set_var("PATH", new_path);
            }
            if let Some(php_ini) = php_ini_path
                && php_ini.exists()
            {
                std::env::set_var("PHPRC", php_ini);
            }
        }

        Ok(())
    }
}

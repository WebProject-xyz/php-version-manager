use crate::constants::MULTISHELL_PATH_VAR;
use crate::{fs, network};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

/// Install a specific PHP version
#[derive(Parser, Debug)]
pub struct Install {
    /// The version to install, or "latest"
    pub version: Option<String>,
}

pub async fn execute_install(version: &str) -> Result<()> {
    let versions_dir = fs::get_versions_dir()?;
    std::fs::create_dir_all(&versions_dir)?;

    println!(
        "{} Resolving latest patch for PHP {}...",
        "↻".blue(),
        version
    );
    let resolved_version = network::resolve_version(version).await?;

    if fs::is_version_installed(&resolved_version)? {
        if version == resolved_version {
            println!("{} PHP {} is already installed.", "✓".green(), version);
        } else {
            println!(
                "{} PHP {} (resolved to {}) is already installed.",
                "✓".green(),
                version,
                resolved_version
            );
        }
        return Ok(());
    }

    println!(
        "{} Fetching PHP {} (resolved to {})...",
        "↻".blue(),
        version,
        resolved_version
    );
    let dest = versions_dir.join(&resolved_version);
    std::fs::create_dir_all(&dest)?;

    match network::download_and_extract(&resolved_version, &dest).await {
        Ok(_) => {
            println!(
                "{} Successfully installed PHP {} as {}",
                "✓".green(),
                version,
                resolved_version
            );

            // Ask user if they want to use it right away
            let theme = dialoguer::theme::ColorfulTheme::default();
            let use_now = dialoguer::Confirm::with_theme(&theme)
                .with_prompt(
                    format!("Do you want to use PHP {} now?", resolved_version)
                        .bold()
                        .to_string(),
                )
                .default(true)
                .interact_opt()
                .unwrap_or(Some(false))
                .unwrap_or(false);

            if use_now {
                let v = crate::fs::resolve_local_version(&resolved_version)?;
                let bin_dir = crate::fs::get_version_bin_dir(&v)?;
                let s = crate::shell::detect_shell();
                let export_str1 = s.set_env_var(MULTISHELL_PATH_VAR, &bin_dir.to_string_lossy());
                let export_str2 = s.path(&bin_dir);

                let env_file = crate::fs::get_env_update_path(None)?;
                crate::fs::write_env_file_locked(
                    &env_file,
                    &format!("{}\n{}", export_str1, export_str2),
                )?;

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
                println!("{} Switched to PHP {}", "✓".green(), v.bold());
            } else {
                println!(
                    "{} To use this version later, run `{}`",
                    "💡".yellow(),
                    format!("pvm use {}", version).bold()
                );
            }
        }
        Err(e) => {
            std::fs::remove_dir_all(&dest).ok();
            anyhow::bail!("Failed to install PHP {}: {}", version, e);
        }
    }

    Ok(())
}

impl Install {
    pub async fn call(self) -> Result<()> {
        match self.version {
            Some(v) => execute_install(&v).await,
            None => {
                let ls_cmd = crate::commands::ls_remote::LsRemote {
                    version_prefix: None,
                };
                ls_cmd.call().await
            }
        }
    }
}

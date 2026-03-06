use crate::{fs, network};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

const DEFAULT_PHP_INI: &str = "; php.ini managed by pvm\n";

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

    // Derive the minor version (e.g. "8.3.30" -> "8.3") to select the correct
    // tarball from GitHub Releases while still installing into a patch-level
    // directory locally.
    let mut parts = resolved_version.split('.').take(2);
    let major = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid resolved version {}", resolved_version))?;
    let minor = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid resolved version {}", resolved_version))?;
    let download_minor = format!("{}.{}", major, minor);

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

    match network::download_and_extract(&download_minor, &resolved_version, &dest).await {
        Ok(_) => {
            if let Ok(php_ini_path) = crate::fs::get_version_php_ini_path(&resolved_version)
                && !php_ini_path.exists()
            {
                if let Some(parent) = php_ini_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(&php_ini_path, DEFAULT_PHP_INI).ok();
            }

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
                match crate::fs::resolve_local_version(&resolved_version) {
                    Ok(v) => {
                        if let Ok(bin_dir) = crate::fs::get_version_bin_dir(&v) {
                            let s = crate::shell::detect_shell();
                            let export_str1 =
                                s.set_env_var("PVM_MULTISHELL_PATH", &bin_dir.to_string_lossy());
                            let export_str2 = s.path(&bin_dir);
                            let php_ini_path = crate::fs::get_version_php_ini_path(&v).ok();
                            let export_str3 = php_ini_path
                                .as_ref()
                                .filter(|p| p.exists())
                                .map(|p| s.set_env_var("PHPRC", &p.to_string_lossy()));

                            if let Ok(pvm_dir) = crate::fs::get_pvm_dir() {
                                let env_file = pvm_dir.join(".env_update");
                                let mut exports = format!("{}\n{}", export_str1, export_str2);
                                if let Some(line) = export_str3 {
                                    exports.push('\n');
                                    exports.push_str(&line);
                                }
                                std::fs::write(&env_file, exports).ok();
                            }

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
                            println!("{} Switched to PHP {}", "✓".green(), v.bold());
                        }
                    }
                    Err(e) => eprintln!("{} Failed to resolve installed version: {}", "✗".red(), e),
                }
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

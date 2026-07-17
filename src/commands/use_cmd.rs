use crate::constants::{MULTISHELL_PATH_VAR, PHP_VERSION_FILE};
use crate::{fs, prompt, shell, update};
use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::IsTerminal;
use std::path::Path;

/// Change PHP version
#[derive(Parser, Debug)]
pub struct Use {
    /// The version to use (omit for interactive list)
    pub version: Option<String>,

    /// Skip interactive prompts when the requested version is missing (used by shell hooks).
    #[arg(long, hide = true)]
    pub silent: bool,

    /// Auto-approve prompts (install missing versions, patch updates)
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

impl Use {
    pub async fn call(self) -> Result<()> {
        // "system" is not a real version: deactivate pvm for this shell instead.
        if self.version.as_deref() == Some("system") {
            let s = shell::detect_shell();
            let env_file = fs::get_env_update_path()?;
            fs::write_env_file_locked(&env_file, &s.deactivate(&fs::get_versions_dir()?))?;
            eprintln!("{} Switched to system PHP", "✓".green());
            return Ok(());
        }

        let mut version = match self.version {
            Some(ref v) => match fs::try_resolve_local_version(v)? {
                Some(resolved) => resolved,
                None => {
                    if self.silent {
                        return Ok(());
                    }

                    let question = format!(
                        "PHP {} is not installed locally. Do you want to install it now?",
                        v.bold()
                    );
                    if !prompt::confirm(&question, true, self.yes)? {
                        eprintln!("{} Operation cancelled.", "✗".red());
                        return Ok(());
                    }

                    // Skip install's own "use now?" prompt — we fall through to
                    // the activation path below with the freshly installed version.
                    match crate::commands::install::execute_install_with(v, false, &[], self.yes)
                        .await?
                    {
                        Some(installed) => installed,
                        None => return Ok(()),
                    }
                }
            },
            None => {
                let mut resolved_version = None;
                if let Ok(content) = std::fs::read_to_string(PHP_VERSION_FILE) {
                    let trimmed = content.trim().to_string();
                    if !trimmed.is_empty() {
                        match fs::try_resolve_local_version(&trimmed)? {
                            Some(resolved) => {
                                resolved_version = Some(resolved);
                            }
                            None => {
                                if !self.silent {
                                    let question = format!(
                                        "PHP {} (from {}) is not installed locally. Do you want to install it now?",
                                        trimmed.bold(),
                                        PHP_VERSION_FILE.bold()
                                    );
                                    if prompt::confirm(&question, true, self.yes)? {
                                        if let Some(installed) =
                                            crate::commands::install::execute_install_with(
                                                &trimmed,
                                                false,
                                                &[],
                                                self.yes,
                                            )
                                            .await?
                                        {
                                            resolved_version = Some(installed);
                                        }
                                    } else {
                                        eprintln!("{} Operation cancelled.", "✗".red());
                                        return Ok(());
                                    }
                                } else {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }

                if let Some(resolved) = resolved_version {
                    resolved
                } else {
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
            }
        };

        // Patch-update offers are an interactive-only feature: scripts must not
        // trigger surprise downloads, so the check is skipped without a terminal.
        if std::io::stdin().is_terminal()
            && let Ok(Some(newer_version)) = update::check_for_updates(&version).await
        {
            let question = format!(
                "{} A new patch version is available: {} ➜ {}. Do you want to install and use it now?",
                "💡".yellow(),
                version.dimmed(),
                newer_version.green().bold()
            );

            if prompt::confirm(&question, true, self.yes)? {
                // Carry over the packages of the version being replaced so the
                // upgrade does not re-ask what is already known.
                let installed_pkgs = fs::get_installed_packages(&version);
                if crate::commands::install::execute_install_with(
                    &newer_version,
                    false,
                    &installed_pkgs,
                    self.yes,
                )
                .await?
                .is_some()
                {
                    version = newer_version;
                }
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
            let question = format!(
                "A {} file is present ({}). Do you want to apply this change to the directory?",
                PHP_VERSION_FILE,
                current_file_ver.trim().yellow()
            );
            // Deliberately not covered by --yes: writing .php-version is a
            // side effect the user should opt into explicitly.
            if prompt::confirm(&question, false, false)? {
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

        let env_file = fs::get_env_update_path()?;
        fs::write_env_file_locked(&env_file, &format!("{}\n{}", export_str1, export_str2))?;

        // Note: process-global env is intentionally NOT mutated here. std::env::set_var
        // is unsound in a multi-threaded tokio runtime, and the wrapper sources env_file
        // into the parent shell on exit, so subsequent pvm invocations see the new PATH.

        Ok(())
    }
}

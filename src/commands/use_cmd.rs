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

        let version = match self.version {
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
                    match pick_installed_version("Select a locally installed PHP version to use")? {
                        Some(v) => return activate(v, ActivateOpts::picker(self.yes)).await,
                        None => return Ok(()),
                    }
                }
            }
        };

        activate(
            version,
            ActivateOpts {
                offer_save: false,
                assume_yes: self.yes,
                quiet: self.silent,
            },
        )
        .await
    }
}

/// Show the installed-version picker. Ok(None) means empty list or cancel
/// (both already reported to the user).
pub(crate) fn pick_installed_version(prompt_text: &str) -> Result<Option<String>> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("No version given. Usage: pvm use <version>");
    }

    let items = fs::get_aliased_versions()?;
    if items.is_empty() {
        eprintln!("{} No PHP versions are currently installed.", "💡".yellow());
        return Ok(None);
    }

    let current = fs::get_current_version();
    let displays: Vec<String> = items
        .iter()
        .map(|i| {
            let pkgs = i.packages.join(", ");
            if i.version == current {
                format!("{} {} [{}]", i.display, "(current)".cyan(), pkgs.cyan())
            } else {
                format!("{} [{}]", i.display, pkgs.cyan())
            }
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt_text)
        .default(0)
        .items(&displays)
        .interact_opt()?;

    match selection {
        Some(idx) => Ok(Some(items[idx].version.clone())),
        None => {
            eprintln!("{} Operation cancelled.", "✗".red());
            Ok(None)
        }
    }
}

pub(crate) struct ActivateOpts {
    /// Version came from an interactive picker: offer to persist it in .php-version.
    pub offer_save: bool,
    pub assume_yes: bool,
    /// Suppress the success message (shell cd-hook runs on every prompt).
    pub quiet: bool,
}

impl ActivateOpts {
    pub(crate) fn picker(assume_yes: bool) -> Self {
        Self {
            offer_save: true,
            assume_yes,
            quiet: false,
        }
    }
}

/// Shared activation tail used by 'pvm use' and the interactive 'pvm ls':
/// patch-update offer, .php-version handling and the env-file write.
pub(crate) async fn activate(mut version: String, opts: ActivateOpts) -> Result<()> {
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

        if prompt::confirm(&question, true, opts.assume_yes)? {
            // Carry over the packages of the version being replaced so the
            // upgrade does not re-ask what is already known.
            let installed_pkgs = fs::get_installed_packages(&version);
            if crate::commands::install::execute_install_with(
                &newer_version,
                false,
                &installed_pkgs,
                opts.assume_yes,
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

    let file_version = std::fs::read_to_string(PHP_VERSION_FILE)
        .ok()
        .map(|c| c.trim().to_string());

    if opts.offer_save {
        // Picker flows: offer to persist the choice for this directory.
        if file_version.as_deref() != Some(version.as_str()) {
            let question = match &file_version {
                Some(old) => format!(
                    "Save {} to {} (currently {})?",
                    version.bold(),
                    PHP_VERSION_FILE,
                    old.yellow()
                ),
                None => format!("Save {} to {}?", version.bold(), PHP_VERSION_FILE),
            };
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
    } else if Path::new(PHP_VERSION_FILE).exists()
        && let Some(ref current_file_ver) = file_version
        && current_file_ver != &version
    {
        let question = format!(
            "A {} file is present ({}). Do you want to apply this change to the directory?",
            PHP_VERSION_FILE,
            current_file_ver.yellow()
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

    if !opts.quiet {
        eprintln!("{} Now using PHP {}", "✓".green(), version.bold());
    }

    Ok(())
}

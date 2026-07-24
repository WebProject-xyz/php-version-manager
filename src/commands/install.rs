use crate::constants::MULTISHELL_PATH_VAR;
use crate::{fs, network};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::IsTerminal;

/// Install a specific PHP version
#[derive(Parser, Debug)]
pub struct Install {
    /// The version to install, or "latest"
    pub version: Option<String>,

    /// Packages to install without prompting (comma-separated: cli,fpm,micro)
    #[arg(long, value_delimiter = ',')]
    pub packages: Vec<String>,

    /// Auto-approve prompts
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

/// `prompt_activation` controls the trailing "Do you want to use PHP X now?" prompt and the
/// resulting env-file write. Callers like `pvm use` set it to `false` because they will fall
/// through to their own activation path with the returned resolved version.
/// `packages` skips the interactive package selection; `assume_yes` auto-approves prompts.
pub async fn execute_install_with(
    version: &str,
    prompt_activation: bool,
    packages: &[String],
    assume_yes: bool,
) -> Result<Option<String>> {
    let versions_dir = fs::get_versions_dir()?;
    std::fs::create_dir_all(&versions_dir)?;

    println!(
        "{} Resolving latest patch for PHP {}...",
        "↻".blue(),
        version
    );
    let resolved_version = network::resolve_version(version).await?;

    let available_versions = network::get_available_versions().await?;
    let available_packages = available_versions
        .iter()
        .find(|(v, _)| v == &resolved_version)
        .map(|(_, pkgs)| pkgs.clone())
        .unwrap_or_default();

    if available_packages.is_empty() {
        anyhow::bail!("No packages found for PHP {}", resolved_version);
    }

    let already_installed = fs::get_installed_packages(&resolved_version);
    if !already_installed.is_empty() {
        println!(
            "{} PHP {} is already installed [{}]",
            "💡".yellow(),
            resolved_version.bold(),
            already_installed.join(", ")
        );
        // Idempotent no-op for scripts: nothing was explicitly requested and
        // the default cli package is already present.
        if packages.is_empty()
            && !std::io::stdin().is_terminal()
            && already_installed.iter().any(|p| p == "cli")
        {
            return Ok(Some(resolved_version));
        }
    }

    let selected_packages = select_packages(
        &resolved_version,
        &available_packages,
        packages,
        &already_installed,
        assume_yes,
    )?;
    let Some(selected_packages) = selected_packages else {
        println!("{} No packages selected. Operation cancelled.", "✗".red());
        return Ok(None);
    };

    let dest = versions_dir.join(&resolved_version);
    let dest_existed = dest.exists();
    std::fs::create_dir_all(&dest)?;

    for package in &selected_packages {
        println!(
            "{} Fetching PHP {} ({}) package...",
            "↻".blue(),
            resolved_version,
            package
        );
        if let Err(e) = network::download_and_extract(&resolved_version, package, &dest).await {
            // Only wipe the dest if it didn't exist before this install attempt;
            // a pre-existing install must not be destroyed by a follow-up failure.
            if !dest_existed {
                std::fs::remove_dir_all(&dest).ok();
            }
            anyhow::bail!(
                "Failed to install PHP {} (package {}): {}",
                resolved_version,
                package,
                e
            );
        }
    }

    println!(
        "{} Successfully installed PHP {} [{}] as {}",
        "✓".green(),
        version,
        selected_packages.join(", "),
        resolved_version
    );

    // Only the cli package places a `php` binary on PATH; without it, switching is meaningless.
    let cli_selected = selected_packages.iter().any(|p| p == "cli");

    if !prompt_activation {
        if !cli_selected {
            println!(
                "{} The 'cli' package was not selected; this version cannot be activated via PATH.",
                "💡".yellow()
            );
            return Ok(None);
        }
        return Ok(Some(resolved_version));
    }

    // Without a terminal (and without -y) do not auto-activate: nothing evals
    // the env file, so a "Switched" message would lie.
    let use_now = cli_selected
        && (assume_yes || std::io::stdin().is_terminal())
        && crate::prompt::confirm(
            &format!("Do you want to use PHP {} now?", resolved_version)
                .bold()
                .to_string(),
            true,
            assume_yes,
        )?;

    if use_now {
        let v = crate::fs::resolve_local_version(&resolved_version)?;
        let bin_dir = crate::fs::get_version_bin_dir(&v)?;
        let s = crate::shell::detect_shell();
        let export_str1 = s.set_env_var(MULTISHELL_PATH_VAR, &bin_dir.to_string_lossy());
        let export_str2 = s.path(&bin_dir);

        let env_file = crate::fs::get_env_update_path()?;
        crate::fs::write_env_file_locked(&env_file, &format!("{}\n{}", export_str1, export_str2))?;

        // Note: process-global env is intentionally NOT mutated here. std::env::set_var
        // is unsound in a multi-threaded tokio runtime, and the wrapper sources env_file
        // into the parent shell on exit, so subsequent pvm invocations see the new PATH.
        println!("{} Switched to PHP {}", "✓".green(), v.bold());
        Ok(Some(resolved_version))
    } else if !cli_selected {
        println!(
            "{} The 'cli' package was not selected; this version cannot be activated via PATH.",
            "💡".yellow()
        );
        Ok(None)
    } else {
        println!(
            "{} To use this version later, run `{}`",
            "💡".yellow(),
            format!("pvm use {}", version).bold()
        );
        Ok(Some(resolved_version))
    }
}

/// Decide which packages to install: an explicit list is validated against
/// what the remote offers (and may reinstall); without one, a non-interactive
/// stdin defaults to "cli" and a terminal gets the MultiSelect, preselecting
/// whatever is not yet installed. Returns None on empty selection.
fn select_packages(
    resolved_version: &str,
    available_packages: &[String],
    requested: &[String],
    already_installed: &[String],
    assume_yes: bool,
) -> Result<Option<Vec<String>>> {
    if !requested.is_empty() {
        for pkg in requested {
            if !available_packages.contains(pkg) {
                anyhow::bail!(
                    "Package '{}' is not available for PHP {} (available: {})",
                    pkg,
                    resolved_version,
                    available_packages.join(", ")
                );
            }
        }
        return Ok(Some(requested.to_vec()));
    }

    // -y skips the MultiSelect exactly like a missing terminal does.
    if assume_yes || !std::io::stdin().is_terminal() {
        let default_pkg = "cli".to_string();
        if !available_packages.contains(&default_pkg) {
            anyhow::bail!(
                "Cannot auto-select packages: the default 'cli' package is not available for PHP {} (available: {}). Use --packages.",
                resolved_version,
                available_packages.join(", ")
            );
        }
        return Ok(Some(vec![default_pkg]));
    }

    let theme = dialoguer::theme::ColorfulTheme::default();
    let selections = dialoguer::MultiSelect::with_theme(&theme)
        .with_prompt(format!(
            "Select packages to install for PHP {}",
            resolved_version
        ))
        .items(available_packages)
        .defaults(
            &available_packages
                .iter()
                .map(|p| {
                    if already_installed.is_empty() {
                        p == "cli"
                    } else {
                        // Preselect what is missing so "add fpm later" is one Enter away.
                        !already_installed.contains(p)
                    }
                })
                .collect::<Vec<_>>(),
        )
        .interact()?;

    if selections.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        selections
            .into_iter()
            .map(|i| available_packages[i].clone())
            .collect(),
    ))
}

/// Interactive remote-version picker used by `pvm install` without a version:
/// quick-select aliases (latest, newest patch per minor) above the full list.
async fn pick_and_install(packages: &[String], assume_yes: bool) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("No version given. Usage: pvm install <version>");
    }

    let versions_info = network::get_available_versions().await?;
    if versions_info.is_empty() {
        println!("{} No remote versions found.", "💡".yellow());
        return Ok(());
    }

    let installed = fs::list_installed_versions().unwrap_or_default();

    let mut display_items = Vec::new();
    let mut target_versions = Vec::new(); // Parallel array tying display index to actual version string

    // Build "Quick Select" aliases
    let mut minors = std::collections::BTreeMap::new();
    let mut highest_overall = None;

    for (v, _) in &versions_info {
        highest_overall = Some(v.clone());
        if let Some(minor) = fs::minor_of(v) {
            minors.insert(minor, v.clone()); // BTreeMap iterates ascending, keeps latest
        }
    }

    if let Some(highest) = highest_overall {
        display_items.push(format!("latest ({})", highest).bold().to_string());
        target_versions.push(highest);
    }

    // Add them in reverse order (newest minor first) for the quick select
    for (minor, highest_patch) in minors.iter().rev() {
        display_items.push(
            format!("{} ({})", minor, highest_patch)
                .bold()
                .cyan()
                .to_string(),
        );
        target_versions.push(highest_patch.clone());
    }

    display_items.push("---".dimmed().to_string());
    target_versions.push("".to_string()); // Unselectable divider

    // Build the rest of the flat list
    for (v, pkgs) in versions_info.iter().rev() {
        let pkgs_str = pkgs.join(", ");
        if installed.contains(v) {
            display_items.push(format!(
                "{} {} {} [{}]",
                "✓".green(),
                v,
                "(installed)".dimmed(),
                pkgs_str.cyan()
            ));
        } else {
            display_items.push(format!("  {} [{}]", v, pkgs_str.cyan()));
        }
        target_versions.push(v.clone());
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a PHP version to install")
        .default(0)
        .items(&display_items)
        .interact_opt()?;

    let selected = match selection {
        Some(idx) => {
            let target = &target_versions[idx];
            if target.is_empty() {
                // They clicked the divider
                println!("{} Invalid selection.", "✗".red());
                return Ok(());
            }
            target
        }
        None => {
            println!("{} Operation cancelled.", "✗".red());
            return Ok(());
        }
    };

    // Explicit --packages must not be swallowed by the short-circuit: it is
    // the way to add packages to an already-installed version.
    if !installed.contains(selected) || !packages.is_empty() {
        execute_install_with(selected, true, packages, assume_yes).await?;
    } else {
        println!(
            "{} PHP {} is already installed.",
            "✓".green(),
            selected.bold()
        );
    }

    Ok(())
}

impl Install {
    pub async fn call(self) -> Result<()> {
        match self.version {
            Some(v) => execute_install_with(&v, true, &self.packages, self.yes)
                .await
                .map(|_| ()),
            None => pick_and_install(&self.packages, self.yes).await,
        }
    }
}

use anyhow::{Context, Result};
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::io::IsTerminal;

/// Ask a yes/no question. `assume_yes` short-circuits to true; a
/// non-interactive stdin returns `default` so scripts never hang on a
/// hidden prompt. Esc/q counts as "no".
pub fn confirm(prompt: &str, default: bool, assume_yes: bool) -> Result<bool> {
    if assume_yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Ok(default);
    }
    Ok(Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact_opt()
        .context("Failed to read confirmation from terminal")?
        .unwrap_or(false))
}

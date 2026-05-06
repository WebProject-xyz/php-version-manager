use crate::network;
use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use serde::Deserialize;
use std::io::Write;

const GITHUB_REPO: &str = "WebProject-xyz/php-version-manager";

/// Check for and apply pvm updates
#[derive(Parser, Debug)]
pub struct SelfUpdate {
    /// Apply the update if a newer version is available (asks for confirmation, default yes)
    #[arg(long)]
    pub apply: bool,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
}

fn current_version() -> Result<semver::Version> {
    let raw = env!("PVM_VERSION");
    let token = raw.split_whitespace().next().unwrap_or(raw);
    let trimmed = token.trim_start_matches('v');
    // git-describe extras (e.g. "-2-gabc") are not valid semver; drop them.
    let core = trimmed.split('-').next().unwrap_or(trimmed);
    semver::Version::parse(core)
        .or_else(|_| semver::Version::parse(env!("CARGO_PKG_VERSION")))
        .with_context(|| {
            format!(
                "Failed to parse current pvm version '{}' from '{}'",
                core, raw
            )
        })
}

fn parse_remote_version(tag: &str) -> Result<semver::Version> {
    let trimmed = tag.trim_start_matches('v');
    semver::Version::parse(trimmed)
        .with_context(|| format!("Failed to parse remote release tag '{}'", tag))
}

async fn fetch_latest_release() -> Result<GhRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );
    let release = network::http_client()?
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("Failed to query GitHub releases API")?
        .error_for_status()
        .context("GitHub releases API returned an error")?
        .json::<GhRelease>()
        .await
        .context("Failed to parse GitHub release JSON")?;
    Ok(release)
}

async fn download_and_replace(tag: &str) -> Result<()> {
    let target = network::get_target_triple()?;
    let asset = format!("pvm-{}.tar.gz", target);
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        GITHUB_REPO, tag, asset
    );

    let current_exe =
        std::env::current_exe().context("Failed to determine current executable path")?;
    let exe_dir = current_exe
        .parent()
        .context("Current executable has no parent directory")?;

    println!("{} Downloading {}...", "↻".blue(), url);

    let response = network::http_client()?
        .get(&url)
        .send()
        .await
        .context("Failed to download release archive")?
        .error_for_status()
        .context("Server returned an error when downloading release")?;

    let pb = network::build_download_progress_bar(response.content_length())?;
    let tmp = network::stream_to_tempfile(response, &pb).await?;
    pb.finish_and_clear();

    // Stage the new binary in the same directory as the current exe so the rename is atomic
    // (cross-filesystem renames would fail otherwise).
    let mut staged = tempfile::Builder::new()
        .prefix(".pvm-update-")
        .tempfile_in(exe_dir)
        .context("Failed to create staging file next to current executable")?;

    {
        let tar = flate2::read::GzDecoder::new(tmp);
        let mut archive = tar::Archive::new(tar);
        let mut found = false;
        for entry in archive
            .entries()
            .context("Failed to read archive entries")?
        {
            let mut entry = entry.context("Failed to read archive entry")?;
            let path = entry.path().context("Invalid entry path")?.into_owned();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "pvm" {
                std::io::copy(&mut entry, staged.as_file_mut())
                    .context("Failed to extract pvm binary from archive")?;
                staged
                    .as_file_mut()
                    .flush()
                    .context("Failed to flush staged binary")?;
                found = true;
                break;
            }
        }
        if !found {
            anyhow::bail!("Release archive did not contain a `pvm` binary");
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(staged.path(), std::fs::Permissions::from_mode(0o755))
            .context("Failed to set permissions on staged binary")?;
    }

    // Atomic rename. On Unix this is safe even while the current binary is executing — the
    // kernel keeps the old inode alive through open fds until the process exits.
    staged
        .persist(&current_exe)
        .with_context(|| format!("Failed to replace current executable {:?}", current_exe))?;

    Ok(())
}

impl SelfUpdate {
    pub async fn call(self) -> Result<()> {
        let current = current_version()?;
        let current_str = current.to_string();
        println!("{} Current version: {}", "↻".blue(), current_str.bold());
        println!("{} Checking GitHub for the latest release...", "↻".blue());

        let release = fetch_latest_release().await?;
        let remote = parse_remote_version(&release.tag_name)?;

        if remote <= current {
            println!("{} pvm is up to date ({})", "✓".green(), current_str.bold());
            return Ok(());
        }

        let remote_str = remote.to_string();
        println!(
            "{} New version available: {} → {}",
            "💡".yellow(),
            current_str.bold(),
            remote_str.bold()
        );

        if !self.apply {
            println!(
                "{} Run `{}` to install it.",
                "💡".yellow(),
                "pvm self-update --apply".bold()
            );
            return Ok(());
        }

        let theme = dialoguer::theme::ColorfulTheme::default();
        let confirmed = dialoguer::Confirm::with_theme(&theme)
            .with_prompt(format!("Update pvm to {}?", remote_str).bold().to_string())
            .default(true)
            .interact_opt()
            .context("Failed to read update confirmation from terminal")?
            .unwrap_or(false);

        if !confirmed {
            println!("{} Update cancelled.", "✗".red());
            return Ok(());
        }

        download_and_replace(&release.tag_name).await?;
        println!(
            "{} Successfully updated pvm to {}",
            "✓".green(),
            remote_str.bold()
        );
        println!(
            "{} Restart your shell or re-run `{}` to pick up the new binary.",
            "💡".yellow(),
            "pvm env".bold()
        );

        Ok(())
    }
}

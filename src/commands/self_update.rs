use crate::network;
use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use serde::Deserialize;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

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

fn parse_pvm_version(raw: &str) -> Result<semver::Version> {
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

fn current_version() -> Result<semver::Version> {
    parse_pvm_version(env!("PVM_VERSION"))
}

fn parse_remote_version(tag: &str) -> Result<semver::Version> {
    let trimmed = tag.trim_start_matches('v');
    semver::Version::parse(trimmed)
        .with_context(|| format!("Failed to parse remote release tag '{}'", tag))
}

fn parse_sha256_digest(raw: &str) -> Result<String> {
    // Accept either bare hex or `sha256sum` output ("<hex>  <filename>"); take first token.
    let token = raw
        .split_whitespace()
        .next()
        .context("Checksum file is empty")?;
    if token.len() != 64 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("Checksum file does not contain a 64-character hex SHA-256 digest");
    }
    Ok(token.to_ascii_lowercase())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn sha256_of_file(file: &mut File) -> Result<String> {
    use sha2::{Digest, Sha256};
    file.seek(SeekFrom::Start(0))
        .context("Failed to rewind archive for hashing")?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buf)
            .context("Failed to read archive while hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    file.seek(SeekFrom::Start(0))
        .context("Failed to rewind archive after hashing")?;
    Ok(hex_encode(&hasher.finalize()))
}

async fn fetch_checksum(url: &str) -> Result<String> {
    let body = network::http_client()?
        .get(url)
        .send()
        .await
        .context("Failed to download release checksum")?
        .error_for_status()
        .context("Server returned an error when downloading release checksum")?
        .text()
        .await
        .context("Failed to read release checksum body")?;
    parse_sha256_digest(&body)
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
    let mut tmp = network::stream_to_tempfile(response, &pb).await?;
    pb.finish_and_clear();

    let checksum_url = format!("{}.sha256", url);
    println!("{} Verifying integrity ({})...", "↻".blue(), checksum_url);
    let expected = fetch_checksum(&checksum_url)
        .await
        .context("Failed to fetch SHA-256 checksum for release archive")?;
    let actual = sha256_of_file(&mut tmp)?;
    if actual != expected {
        anyhow::bail!(
            "Integrity check failed for downloaded release: expected SHA-256 {}, got {}",
            expected,
            actual
        );
    }
    println!("{} Checksum OK", "✓".green());

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

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u64, minor: u64, patch: u64) -> semver::Version {
        semver::Version::new(major, minor, patch)
    }

    #[test]
    fn parse_pvm_version_strips_leading_v() {
        assert_eq!(parse_pvm_version("v1.2.3").unwrap(), v(1, 2, 3));
    }

    #[test]
    fn parse_pvm_version_drops_git_describe_suffix() {
        assert_eq!(parse_pvm_version("1.2.3-2-gabc").unwrap(), v(1, 2, 3));
        assert_eq!(parse_pvm_version("v1.2.3-2-gabc").unwrap(), v(1, 2, 3));
    }

    #[test]
    fn parse_pvm_version_takes_first_whitespace_token() {
        // build.rs may embed "VERSION (commit ...)" forms.
        assert_eq!(
            parse_pvm_version("1.2.3 (abcd1234 2026-01-01)").unwrap(),
            v(1, 2, 3)
        );
    }

    #[test]
    fn parse_pvm_version_falls_back_to_cargo_pkg_version_on_unknown() {
        let parsed = parse_pvm_version("unknown").unwrap();
        let pkg = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(parsed, pkg);
    }

    #[test]
    fn parse_remote_version_strips_leading_v() {
        assert_eq!(parse_remote_version("v1.2.3").unwrap(), v(1, 2, 3));
        assert_eq!(parse_remote_version("1.2.3").unwrap(), v(1, 2, 3));
    }

    #[test]
    fn parse_remote_version_rejects_garbage() {
        assert!(parse_remote_version("not-a-version").is_err());
    }

    #[test]
    fn parse_sha256_digest_accepts_bare_hex() {
        let raw = "a".repeat(64);
        assert_eq!(parse_sha256_digest(&raw).unwrap(), raw);
    }

    #[test]
    fn parse_sha256_digest_accepts_sha256sum_format() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let raw = format!("{}  pvm-linux-x86_64.tar.gz\n", hex);
        assert_eq!(parse_sha256_digest(&raw).unwrap(), hex);
    }

    #[test]
    fn parse_sha256_digest_lowercases_input() {
        let upper = "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789";
        let lower = upper.to_ascii_lowercase();
        assert_eq!(parse_sha256_digest(upper).unwrap(), lower);
    }

    #[test]
    fn parse_sha256_digest_rejects_short() {
        assert!(parse_sha256_digest("deadbeef").is_err());
    }

    #[test]
    fn parse_sha256_digest_rejects_non_hex() {
        let raw = "z".repeat(64);
        assert!(parse_sha256_digest(&raw).is_err());
    }

    #[test]
    fn parse_sha256_digest_rejects_empty() {
        assert!(parse_sha256_digest("").is_err());
        assert!(parse_sha256_digest("   \n").is_err());
    }

    #[test]
    fn sha256_of_file_matches_known_vector() {
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let mut tmp = tempfile::tempfile().unwrap();
        tmp.write_all(b"abc").unwrap();
        tmp.flush().unwrap();
        let digest = sha256_of_file(&mut tmp).unwrap();
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // Hashing must rewind so the next reader sees the full content.
        let mut s = String::new();
        tmp.read_to_string(&mut s).unwrap();
        assert_eq!(s, "abc");
    }

    #[test]
    fn sha256_of_empty_file() {
        let mut tmp = tempfile::tempfile().unwrap();
        let digest = sha256_of_file(&mut tmp).unwrap();
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}

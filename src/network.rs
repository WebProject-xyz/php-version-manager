use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use scraper::{Html, Selector};
use std::path::Path;
use tar::Archive;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;

const BASE_URL: &str = "https://dl.static-php.dev/static-php-cli/common/";
const CACHE_FILE: &str = "remote_cache.json";
const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

pub async fn get_available_versions() -> Result<Vec<String>> {
    let pvm_dir = crate::fs::get_pvm_dir()?;
    let cache_path = pvm_dir.join(CACHE_FILE);

    // 1. Try to load from valid cache
    if cache_path.exists()
        && let Ok(metadata) = std::fs::metadata(&cache_path)
        && let Ok(modified) = metadata.modified()
        && let Ok(elapsed) = modified.elapsed()
        && elapsed < CACHE_DURATION
        && let Ok(mut file) = File::open(&cache_path)
    {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok()
            && let Ok(versions) = serde_json::from_str::<Vec<String>>(&contents)
        {
            return Ok(versions);
        }
    }

    // 2. Fetch from remote with a spinner
    println!(
        "{} Fetching available versions from dl.static-php.dev...",
        "↻".blue()
    );
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")?,
    );
    spinner.set_message("Fetching...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let client = Client::new();
    let res = client.get(BASE_URL).send().await?.text().await?;
    spinner.finish_and_clear();

    let document = Html::parse_document(&res);
    let selector = Selector::parse("a").unwrap();

    let mut versions = Vec::new();
    for element in document.select(&selector) {
        let href = element.value().attr("href").unwrap_or("");
        if href.contains("/php-") && href.ends_with("-cli-linux-x86_64.tar.gz") {
            let filename = href.rsplit('/').next().unwrap_or(href);
            if let Some(version) = filename
                .strip_prefix("php-")
                .and_then(|h| h.strip_suffix("-cli-linux-x86_64.tar.gz"))
            {
                versions.push(version.to_string());
            }
        } else if href.starts_with("php-")
            && href.ends_with("-cli-linux-x86_64.tar.gz")
            && let Some(version) = href
                .strip_prefix("php-")
                .and_then(|h| h.strip_suffix("-cli-linux-x86_64.tar.gz"))
        {
            versions.push(version.to_string());
        }
    }

    // Sort versions by splitting by dot and parsing as numbers
    versions.sort_by(|a, b| {
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
        a_parts.cmp(&b_parts)
    });

    // 3. Write to cache
    if let Ok(json) = serde_json::to_string(&versions) {
        std::fs::create_dir_all(&pvm_dir).ok();
        if let Ok(mut file) = File::create(&cache_path) {
            file.write_all(json.as_bytes()).ok();
        }
    }

    Ok(versions)
}

pub async fn resolve_version(requested: &str) -> Result<String> {
    let versions = get_available_versions().await?;

    if requested == "latest" {
        if let Some(latest) = versions.last() {
            return Ok(latest.clone());
        } else {
            anyhow::bail!("No versions available from remote");
        }
    }

    // Exact match
    if versions.contains(&requested.to_string()) {
        return Ok(requested.to_string());
    }

    // Look for latest patch (e.g., requested "8.3", look for "8.3.*")
    let prefix = format!("{}.", requested);
    let matching: Vec<&String> = versions.iter().filter(|v| v.starts_with(&prefix)).collect();

    // The list is already sorted ascending, so the last match is the newest
    if let Some(latest) = matching.last() {
        return Ok((*latest).clone());
    }

    anyhow::bail!(
        "Could not resolve a remotely available version matching '{}'",
        requested
    )
}

pub async fn download_and_extract(resolved_version: &str, dest: &Path) -> Result<()> {
    let url = format!(
        "{}/php-{}-cli-linux-x86_64.tar.gz",
        BASE_URL, resolved_version
    );
    let client = Client::new();
    let res = client.get(&url).send().await?.bytes().await?;

    let cursor = std::io::Cursor::new(res);
    let tar = GzDecoder::new(cursor);
    let mut archive = Archive::new(tar);

    let bin_dir = dest.join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    archive
        .unpack(&bin_dir)
        .context("Failed to unpack downloaded archive")?;

    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let p_bin = bin_dir.join("php");
        if p_bin.exists() {
            let mut perms = std::fs::metadata(&p_bin)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&p_bin, perms)?;
        }
    }

    Ok(())
}

use crate::constants::{BASE_URL, REMOTE_CACHE_FILE};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use fs4::fs_std::FileExt;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use tar::Archive;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;

const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

#[derive(Deserialize)]
struct RemoteFile {
    name: String,
    is_dir: bool,
}

fn get_target_triple() -> Result<&'static str> {
    use std::env::consts::{ARCH, OS};
    match (OS, ARCH) {
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("linux", "aarch64") => Ok("linux-aarch64"),
        ("macos", "x86_64") => Ok("macos-x86_64"),
        ("macos", "aarch64") => Ok("macos-aarch64"),
        _ => anyhow::bail!("Unsupported OS/Architecture: {}-{}", OS, ARCH),
    }
}

pub async fn get_available_versions() -> Result<Vec<String>> {
    let pvm_dir = crate::fs::get_pvm_dir()?;
    let cache_path = pvm_dir.join(REMOTE_CACHE_FILE);

    // 1. Try to load from valid cache
    if cache_path.exists()
        && let Ok(file) = File::open(&cache_path)
    {
        file.lock_shared().ok();
        let mut contents = String::new();
        let mut f = &file;
        let read_res = f.read_to_string(&mut contents);
        file.unlock().ok();

        if read_res.is_ok()
            && let Ok(metadata) = std::fs::metadata(&cache_path)
            && let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = modified.elapsed()
            && elapsed < CACHE_DURATION
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
    let json_url = format!("{}?format=json", BASE_URL);
    let res = client
        .get(json_url)
        .send()
        .await
        .context("Failed to fetch version list from remote")?
        .error_for_status()
        .context("Remote server returned an error when fetching version list")?
        .json::<Vec<RemoteFile>>()
        .await
        .context("Failed to parse remote version JSON")?;
    spinner.finish_and_clear();

    let target = get_target_triple()?;
    let suffix = format!("-cli-{}.tar.gz", target);

    let mut versions = Vec::new();
    for file in res {
        if !file.is_dir
            && file.name.starts_with("php-")
            && file.name.ends_with(&suffix)
            && let Some(version) = file
                .name
                .strip_prefix("php-")
                .and_then(|h: &str| h.strip_suffix(&suffix))
        {
            versions.push(version.to_string());
        }
    }

    crate::utils::sort_versions(&mut versions);

    // 3. Write to cache
    if let Ok(json) = serde_json::to_string(&versions) {
        std::fs::create_dir_all(&pvm_dir).ok();
        if let Ok(file) = File::create(&cache_path) {
            file.lock_exclusive().ok();
            let mut writer = std::io::BufWriter::new(&file);
            writer.write_all(json.as_bytes()).ok();
            writer.flush().ok();
            file.unlock().ok();
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
    let target = get_target_triple()?;
    let url = format!("{}php-{}-cli-{}.tar.gz", BASE_URL, resolved_version, target);
    let client = Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to download server")?
        .error_for_status()
        .context("Server returned an error for the requested PHP version")?;

    let total_size = response
        .content_length()
        .context("Failed to get content length from server")?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
        .progress_chars("#>-"));

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();

    while let Some(item) = stream.next().await {
        let chunk = item.context("Error while downloading chunk")?;
        buffer.extend_from_slice(&chunk);
        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message("Download complete");

    let cursor = std::io::Cursor::new(buffer);
    let tar = GzDecoder::new(cursor);
    let mut archive = Archive::new(tar);

    let bin_dir = dest.join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    archive.unpack(&bin_dir).context(
        "Failed to unpack downloaded archive - the file might be corrupted or incomplete",
    )?;

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

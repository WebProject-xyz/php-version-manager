use crate::constants::{BASE_URL, REMOTE_CACHE_FILE};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;

use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use tar::Archive;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Duration;

const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const USER_AGENT: &str = concat!("pvm/", env!("CARGO_PKG_VERSION"));

pub(crate) fn http_client() -> Result<Client> {
    Client::builder()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .context("Failed to initialize HTTP client")
}

#[derive(Deserialize)]
struct RemoteFile {
    name: String,
    is_dir: bool,
}

pub(crate) fn get_target_triple() -> Result<&'static str> {
    use std::env::consts::{ARCH, OS};
    match (OS, ARCH) {
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("linux", "aarch64") => Ok("linux-aarch64"),
        ("macos", "x86_64") => Ok("macos-x86_64"),
        ("macos", "aarch64") => Ok("macos-aarch64"),
        _ => anyhow::bail!("Unsupported OS/Architecture: {}-{}", OS, ARCH),
    }
}

pub(crate) fn build_download_progress_bar(total_size: Option<u64>) -> Result<ProgressBar> {
    let pb = if let Some(size) = total_size {
        let pb = ProgressBar::new(size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-"));
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template(
            "{spinner:.green} [{elapsed_precise}] {bytes} downloaded ({bytes_per_sec})",
        )?);
        pb
    };
    Ok(pb)
}

// Stream the response into a temp file to avoid materializing the whole archive in memory,
// returning a handle rewound to position 0 so the caller can feed it to a decoder.
pub(crate) async fn stream_to_tempfile(
    response: reqwest::Response,
    pb: &ProgressBar,
) -> Result<File> {
    let mut tmp = tempfile::tempfile().context("Failed to create temporary archive file")?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item.context("Error while downloading chunk")?;
        tmp.write_all(&chunk)
            .context("Failed to write archive chunk to temp file")?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }
    tmp.flush().context("Failed to flush temp archive file")?;
    tmp.seek(SeekFrom::Start(0))
        .context("Failed to rewind temp archive file")?;
    Ok(tmp)
}

pub async fn get_available_versions() -> Result<Vec<(String, Vec<String>)>> {
    let pvm_dir = crate::fs::get_pvm_dir()?;
    let target = get_target_triple()?;
    // Cache is filtered by target triple, so the filename must include it
    // to prevent cross-arch reuse when $PVM_DIR is shared (e.g. via NFS).
    let cache_path = pvm_dir.join(format!(
        "{}-{}.json",
        REMOTE_CACHE_FILE.trim_end_matches(".json"),
        target
    ));

    // 1. Try to load from valid cache
    if cache_path.exists()
        && let Ok(file) = File::open(&cache_path)
        && file.lock_shared().is_ok()
    {
        let mut contents = String::new();
        let mut f = &file;
        let read_res = f.read_to_string(&mut contents);
        file.unlock().ok();

        if read_res.is_ok()
            && let Ok(metadata) = std::fs::metadata(&cache_path)
            && let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = modified.elapsed()
            && elapsed < CACHE_DURATION
            && let Ok(versions) = serde_json::from_str::<Vec<(String, Vec<String>)>>(&contents)
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

    let client = http_client()?;
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

    let suffix = format!("-{}.tar.gz", target);

    let mut versions_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for file in res {
        if !file.is_dir
            && file.name.starts_with("php-")
            && file.name.ends_with(&suffix)
            && let Some(rest) = file
                .name
                .strip_prefix("php-")
                .and_then(|s| s.strip_suffix(&suffix))
            && let Some(idx) = rest.rfind('-')
        {
            let version = &rest[..idx];
            let package = &rest[idx + 1..];
            versions_map
                .entry(version.to_string())
                .or_default()
                .push(package.to_string());
        }
    }

    let mut versions: Vec<(String, Vec<String>)> = versions_map.into_iter().collect();

    versions.sort_by(|a, b| {
        let v1 = semver::Version::parse(&a.0).unwrap_or(semver::Version::new(0, 0, 0));
        let v2 = semver::Version::parse(&b.0).unwrap_or(semver::Version::new(0, 0, 0));
        v1.cmp(&v2)
    });

    for (_, pkgs) in &mut versions {
        pkgs.sort();
    }

    // 3. Write to cache (skip silently if lock fails — next run will retry)
    if let Ok(json) = serde_json::to_string(&versions) {
        std::fs::create_dir_all(&pvm_dir).ok();
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&cache_path)
            && file.lock().is_ok()
        {
            file.set_len(0).ok();
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
        if let Some((latest, _)) = versions.last() {
            return Ok(latest.clone());
        } else {
            anyhow::bail!("No versions available from remote");
        }
    }

    // Exact match
    if versions.iter().any(|(v, _)| v == requested) {
        return Ok(requested.to_string());
    }

    // Look for latest patch (e.g., requested "8.3", look for "8.3.*")
    let prefix = format!("{}.", requested);
    let matching: Vec<&String> = versions
        .iter()
        .filter_map(|(v, _)| {
            if v.starts_with(&prefix) {
                Some(v)
            } else {
                None
            }
        })
        .collect();

    // The list is already sorted ascending, so the last match is the newest
    if let Some(latest) = matching.last() {
        return Ok((*latest).clone());
    }

    anyhow::bail!(
        "Could not resolve a remotely available version matching '{}'",
        requested
    )
}

pub async fn download_and_extract(
    resolved_version: &str,
    package: &str,
    dest: &Path,
) -> Result<()> {
    let target = get_target_triple()?;
    let url = format!(
        "{}php-{}-{}-{}.tar.gz",
        BASE_URL, resolved_version, package, target
    );
    let client = http_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to download server")?
        .error_for_status()
        .context(format!(
            "Server returned an error for PHP {} ({})",
            resolved_version, package
        ))?;

    let pb = build_download_progress_bar(response.content_length())?;
    let tmp = stream_to_tempfile(response, &pb).await?;
    pb.finish_with_message(format!("Downloaded package {}", package));

    let tar = GzDecoder::new(tmp);
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
        if let Ok(entries) = std::fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let mut perms = std::fs::metadata(&path)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&path, perms).ok();
                }
            }
        }
    }

    Ok(())
}

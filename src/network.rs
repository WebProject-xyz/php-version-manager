use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use tar::Archive;

const PHP_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/WebProject-xyz/php-version-manager/main/php-versions.json";
const PHP_RELEASE_BASE: &str =
    "https://github.com/WebProject-xyz/php-version-manager/releases/download";
const PHP_RELEASE_TAG: &str = "latest-build";

#[derive(Debug, Deserialize)]
struct PhpVersionManifestEntry {
    pub build: String,
}

type PhpVersionManifest = BTreeMap<String, PhpVersionManifestEntry>;

async fn fetch_manifest() -> Result<PhpVersionManifest> {
    let client = Client::new();
    let url =
        std::env::var("PVM_REMOTE_MANIFEST_URL").unwrap_or_else(|_| PHP_MANIFEST_URL.to_string());

    let res = client.get(&url).send().await?;
    if !res.status().is_success() {
        anyhow::bail!(
            "Failed to fetch PHP version manifest from {} (status {})",
            url,
            res.status()
        );
    }

    let text = res.text().await?;
    let manifest: PhpVersionManifest =
        serde_json::from_str(&text).context("Failed to parse php-versions.json manifest")?;

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolves_versions_from_manifest() {
        let mut manifest = PhpVersionManifest::new();
        manifest.insert(
            "8.2".to_string(),
            PhpVersionManifestEntry {
                build: "8.2.28".to_string(),
            },
        );
        manifest.insert(
            "8.3".to_string(),
            PhpVersionManifestEntry {
                build: "8.3.30".to_string(),
            },
        );
        manifest.insert(
            "8.4".to_string(),
            PhpVersionManifestEntry {
                build: "8.4.18".to_string(),
            },
        );

        // Verify our sorting helper logic with a local manifest instance
        let mut versions: Vec<String> = manifest.values().map(|e| e.build.clone()).collect();
        versions.sort_by(|a, b| {
            let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
            let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
            a_parts.cmp(&b_parts)
        });
        assert_eq!(versions, vec!["8.2.28", "8.3.30", "8.4.18"]);

        // And verify the "latest" minor resolution logic using the same comparison
        let latest = manifest
            .iter()
            .max_by(|(a, _), (b, _)| {
                let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                a_parts.cmp(&b_parts)
            })
            .map(|(_, entry)| entry.build.clone());

        assert_eq!(latest, Some("8.4.18".to_string()));
    }
}

pub async fn get_available_versions() -> Result<Vec<String>> {
    let manifest = fetch_manifest().await?;

    let mut versions: Vec<String> = manifest.values().map(|entry| entry.build.clone()).collect();

    // Sort versions by splitting by dot and parsing as numbers
    versions.sort_by(|a, b| {
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
        a_parts.cmp(&b_parts)
    });

    Ok(versions)
}

pub async fn resolve_version(requested: &str) -> Result<String> {
    let manifest = fetch_manifest().await?;

    if requested == "latest" {
        // Pick the highest minor key and return its build
        if let Some((_minor, entry)) = manifest.iter().max_by(|(a, _), (b, _)| {
            let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
            let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
            a_parts.cmp(&b_parts)
        }) {
            return Ok(entry.build.clone());
        } else {
            anyhow::bail!("No versions available from remote manifest");
        }
    }

    // 1. Exact match on minor key (e.g. "8.3" -> "8.3.30")
    if let Some(entry) = manifest.get(requested) {
        return Ok(entry.build.clone());
    }

    // 2. Exact match on a build value (e.g. "8.3.30")
    if manifest.values().any(|entry| entry.build == requested) {
        return Ok(requested.to_string());
    }

    anyhow::bail!(
        "Could not resolve a remotely available version matching '{}' from manifest",
        requested
    )
}

pub async fn download_and_extract(minor: &str, _resolved_version: &str, dest: &Path) -> Result<()> {
    let url = format!(
        "{}/{}/php-{}-linux-x86_64.tar.gz",
        PHP_RELEASE_BASE, PHP_RELEASE_TAG, minor
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

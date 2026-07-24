use crate::constants::UPDATE_CHECK_GUARD_FILE;
use crate::{fs, network};
use anyhow::Result;

use std::io::{Read, Seek, Write};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn check_for_updates(target_version: &str) -> Result<Option<String>> {
    let mode = std::env::var("PVM_UPDATE_MODE").unwrap_or_else(|_| "notify".to_string());
    if mode == "disabled" {
        return Ok(None);
    }

    let pvm_dir = fs::get_pvm_dir()?;
    let guard_file = pvm_dir.join(UPDATE_CHECK_GUARD_FILE);

    // Acquire lock and check if 24 hours have passed
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&guard_file)?;

    file.lock()?;

    let mut contents = String::new();
    file.read_to_string(&mut contents).ok();

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if !contents.is_empty()
        && let Ok(last_check) = contents.trim().parse::<u64>()
        && now.saturating_sub(last_check) < 86400
    {
        file.unlock().ok();
        return Ok(None);
    }

    // Write the new timestamp to prevent spam on next commands
    file.set_len(0).ok();
    file.rewind().ok();
    let mut writer = std::io::BufWriter::new(&file);
    writeln!(writer, "{}", now).ok();
    writer.flush().ok();
    file.unlock().ok();

    if target_version == "system" {
        return Ok(None);
    }

    // Parse out the minor version (e.g., "8.4.1" -> "8.4")
    let Some(minor_prefix) = fs::minor_of(target_version) else {
        return Ok(None);
    };

    // Fetch remotes and resolve the newest patch for that minor line.
    // Resolution failures are ignored: the update check is best-effort.
    if let Ok(latest_matching) = network::resolve_version(&minor_prefix).await
        && latest_matching != target_version
    {
        return Ok(Some(latest_matching));
    }

    Ok(None)
}

use crate::{fs, network};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn check_for_updates(target_version: &str) -> Result<Option<String>> {
    let mode = std::env::var("PVM_UPDATE_MODE").unwrap_or_else(|_| "notify".to_string());
    if mode == "disabled" {
        return Ok(None);
    }

    let pvm_dir = fs::get_pvm_dir()?;
    let guard_file = pvm_dir.join(".update_check_guard");

    // Check if 24 hours have passed
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if guard_file.exists()
        && let Ok(contents) = std::fs::read_to_string(&guard_file)
        && let Ok(last_check) = contents.trim().parse::<u64>()
        && now - last_check < 86400
    {
        return Ok(None);
    }

    // Write the new timestamp to prevent spam on next commands
    std::fs::write(&guard_file, now.to_string()).ok();

    if target_version == "system" {
        return Ok(None);
    }

    // Parse out the minor version (e.g., "8.4.1" -> "8.4")
    let parts: Vec<&str> = target_version.split('.').collect();
    if parts.len() < 2 {
        return Ok(None);
    }
    let minor_prefix = format!("{}.{}", parts[0], parts[1]);

    // Fetch remotes and resolve the newest patch for that minor line
    if let Ok(latest_matching) = network::resolve_version(&minor_prefix).await
        && latest_matching != target_version
    {
        return Ok(Some(latest_matching));
    }

    Ok(None)
}

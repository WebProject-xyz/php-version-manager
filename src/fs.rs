use crate::constants::{MULTISHELL_PATH_VAR, PVM_DIR_VAR};
use anyhow::{Context, Result};

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct VersionItem {
    pub display: String,
    pub version: String,
}

pub fn get_pvm_dir() -> Result<PathBuf> {
    if let Ok(pvm_dir) = std::env::var(PVM_DIR_VAR) {
        return Ok(PathBuf::from(pvm_dir));
    }
    let home = dirs::data_local_dir().context("Could not find local data directory")?;
    Ok(home.join("pvm"))
}

pub fn get_versions_dir() -> Result<PathBuf> {
    Ok(get_pvm_dir()?.join("versions"))
}

pub fn get_version_bin_dir(version: &str) -> Result<PathBuf> {
    Ok(get_versions_dir()?.join(version).join("bin"))
}

pub fn is_version_installed(version: &str) -> Result<bool> {
    let version_bin = get_version_bin_dir(version)?.join("php");
    Ok(version_bin.exists())
}

pub fn list_installed_versions() -> Result<Vec<String>> {
    let versions_dir = get_versions_dir()?;
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    for entry in std::fs::read_dir(versions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Ok(name) = entry.file_name().into_string()
        {
            versions.push(name);
        }
    }

    crate::utils::sort_versions(&mut versions);
    Ok(versions)
}

pub fn get_current_version() -> String {
    if let Ok(path) = std::env::var(MULTISHELL_PATH_VAR) {
        let p = PathBuf::from(path);
        if let Some(parent) = p.parent()
            && let Some(name) = parent.file_name()
        {
            return name.to_string_lossy().into_owned();
        }
    }
    "system".to_string()
}

pub fn get_env_update_path(override_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    if let Ok(env_path) = std::env::var("PVM_ENV_UPDATE_PATH") {
        return Ok(PathBuf::from(env_path));
    }
    let pvm_dir = get_pvm_dir()?;
    let shell_pid = std::env::var("PVM_SHELL_PID").unwrap_or_default();
    let filename = if shell_pid.is_empty() {
        crate::constants::ENV_UPDATE_FILE.to_string()
    } else {
        format!("{}_{}", crate::constants::ENV_UPDATE_FILE, shell_pid)
    };
    Ok(pvm_dir.join(filename))
}

/// Safely writes content to the environment update file with an exclusive lock.
pub fn write_env_file_locked(path: &PathBuf, content: &str) -> Result<()> {
    use std::io::Write;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)?;

    file.lock()?;
    file.set_len(0)?;
    let mut writer = std::io::BufWriter::new(&file);
    writer.write_all(content.as_bytes())?;
    writer.flush()?;
    file.unlock()?;
    Ok(())
}

pub fn get_aliased_versions() -> Result<Vec<VersionItem>> {
    let mut installed = list_installed_versions()?;
    if installed.is_empty() {
        return Ok(Vec::new());
    }

    // Sort semantic versions cleanly
    installed.sort_by(|a, b| {
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
        a_parts.cmp(&b_parts)
    });

    let mut items = Vec::new();

    // Latest alias
    if let Some(highest) = installed.last() {
        items.push(VersionItem {
            display: format!("latest ({})", highest),
            version: highest.clone(),
        });
    }

    // Minor version aliases
    let mut minors = std::collections::BTreeMap::new();
    for v in &installed {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 2 {
            let minor = format!("{}.{}", parts[0], parts[1]);
            // BTreeMap keeps the latest because we iterate in ascending order, overriding previous values
            minors.insert(minor, v.clone());
        }
    }

    // Add them to the list
    for (minor, highest_patch) in minors.iter() {
        items.push(VersionItem {
            display: format!("{} ({})", minor, highest_patch),
            version: highest_patch.clone(),
        });

        // Add absolute versions for this minor in ascending order
        for v in &installed {
            if v.starts_with(&format!("{}.", minor)) {
                items.push(VersionItem {
                    display: v.clone(),
                    version: v.clone(),
                });
            }
        }
    }

    Ok(items)
}

pub fn resolve_local_version(requested: &str) -> Result<String> {
    if requested == "latest" {
        return get_aliased_versions()?
            .into_iter()
            .find(|item| item.display.starts_with("latest"))
            .map(|item| item.version)
            .ok_or_else(|| anyhow::anyhow!("No PHP versions are currently installed."));
    }

    let installed = list_installed_versions()?;
    if installed.contains(&requested.to_string()) {
        return Ok(requested.to_string());
    }

    let prefix = format!("{}.", requested);
    let matching: Vec<&String> = installed
        .iter()
        .filter(|v| v.starts_with(&prefix))
        .collect();

    if let Some(latest) = matching.last() {
        return Ok((*latest).clone());
    }

    anyhow::bail!("PHP {} is not installed locally.", requested)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_pvm_dir_with_env_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("PVM_DIR", temp_dir.path());
        }

        let pvm_dir = get_pvm_dir().unwrap();
        assert_eq!(pvm_dir, temp_dir.path());

        unsafe {
            std::env::remove_var("PVM_DIR");
        }
    }

    #[test]
    fn test_list_installed_versions() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("PVM_DIR", temp_dir.path());
        }

        let versions_dir = get_versions_dir().unwrap();
        fs::create_dir_all(versions_dir.join("8.3.1")).unwrap();
        fs::create_dir_all(versions_dir.join("8.2.14")).unwrap();
        fs::create_dir_all(versions_dir.join("8.4.0")).unwrap();

        // This is a file, should be ignored
        fs::write(versions_dir.join("ignore.txt"), "hello").unwrap();

        let versions = list_installed_versions().unwrap();

        // Ensure sorted semver
        assert_eq!(versions, vec!["8.2.14", "8.3.1", "8.4.0"]);

        unsafe {
            std::env::remove_var("PVM_DIR");
        }
    }
}

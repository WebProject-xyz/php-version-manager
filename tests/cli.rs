use predicates::prelude::*;

/// Seed the remote version cache so commands that hit the network resolve
/// entirely offline. Mirrors the cache filename scheme in network.rs.
fn seed_remote_cache(pvm_dir: &std::path::Path, versions: &[(&str, &[&str])]) {
    let data: Vec<(String, Vec<String>)> = versions
        .iter()
        .map(|(v, pkgs)| (v.to_string(), pkgs.iter().map(|p| p.to_string()).collect()))
        .collect();
    let target = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    std::fs::create_dir_all(pvm_dir).unwrap();
    std::fs::write(
        pvm_dir.join(format!("remote_cache-{}.json", target)),
        serde_json::to_string(&data).unwrap(),
    )
    .unwrap();
}

/// Create a fake installed version with the given package binaries.
fn seed_installed_version(pvm_dir: &std::path::Path, version: &str, packages: &[&str]) {
    let bin_dir = pvm_dir.join("versions").join(version).join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    for pkg in packages {
        let file = match *pkg {
            "cli" => "php",
            "fpm" => "php-fpm",
            "micro" => "micro.sfx",
            other => panic!("unknown package {}", other),
        };
        std::fs::write(bin_dir.join(file), "").unwrap();
    }
}

#[test]
fn test_install_resolve_failure_uses_cache_offline() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(temp_dir.path(), &[("8.9.7", &["cli"])]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("install").arg("7.0");
    cmd.assert().failure().stderr(predicate::str::contains(
        "Could not resolve a remotely available version matching '7.0'",
    ));
}

#[test]
fn test_ls_remote_plain_listing() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(
        temp_dir.path(),
        &[("8.9.6", &["cli", "fpm"]), ("8.9.7", &["cli"])],
    );
    seed_installed_version(temp_dir.path(), "8.9.6", &["cli"]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("ls-remote");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("8.9.7 [cli]"))
        .stdout(predicate::str::contains("8.9.6"))
        .stdout(predicate::str::contains("(installed)"));
}

#[test]
fn test_ls_remote_prefix_filter() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(temp_dir.path(), &[("8.8.1", &["cli"]), ("8.9.7", &["cli"])]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("ls-remote").arg("8.9");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("8.9.7"))
        .stdout(predicate::str::contains("8.8.1").not());
}

#[test]
fn test_install_no_version_non_tty_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(temp_dir.path(), &[("8.9.7", &["cli"])]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("install");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No version given"));
}

#[test]
fn test_install_packages_flag_rejects_unavailable() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(temp_dir.path(), &[("8.9.7", &["cli"])]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("install").arg("8.9.7").arg("--packages").arg("fpm");
    cmd.assert().failure().stderr(predicate::str::contains(
        "Package 'fpm' is not available for PHP 8.9.7",
    ));
}

#[test]
fn test_install_non_tty_without_cli_package_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(temp_dir.path(), &[("8.9.7", &["fpm"])]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("install").arg("8.9.7");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Use --packages"));
}

#[test]
fn test_install_already_installed_is_idempotent_non_tty() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(temp_dir.path(), &[("8.9.7", &["cli", "fpm"])]);
    seed_installed_version(temp_dir.path(), "8.9.7", &["cli"]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("install").arg("8.9.7");
    // Succeeds without any download attempt (the fake version would 404).
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("already installed [cli]"));
    assert!(temp_dir.path().join("versions/8.9.7/bin/php").exists());
}

#[test]
fn test_default_set_show_and_env_activation() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_installed_version(temp_dir.path(), "8.9.6", &["cli"]);

    // Set the default
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("default").arg("8.9.6");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Default version set to 8.9.6"));

    // Non-TTY 'pvm default' prints it
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("default");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("8.9.6"));

    // 'pvm env' activates it for new shells
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("env").arg("--shell=bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("export PVM_MULTISHELL_PATH="))
        .stdout(predicate::str::contains("versions/8.9.6/bin"));

    // 'pvm default system' clears it again
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("default").arg("system");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Default cleared"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("env").arg("--shell=bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PVM_MULTISHELL_PATH").not());
}

#[test]
fn test_default_rejects_missing_version() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("default").arg("9.9.9");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not installed locally"));
}

#[test]
fn test_use_system_writes_deactivation_env_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let env_file = temp_dir.path().join("custom_env_update");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_ENV_UPDATE_PATH", &env_file);
    cmd.env("SHELL", "/bin/bash");
    cmd.arg("use").arg("system");
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Switched to system PHP"));

    let content = std::fs::read_to_string(env_file).unwrap();
    assert!(content.contains("export PVM_MULTISHELL_PATH=''"));
    assert!(content.contains("grep -vF"));
    assert!(content.contains("versions"));
}

#[test]
fn test_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains(
        "Fast and simple PHP version manager",
    ));
}

#[test]
fn test_version() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("pvm"));
}

#[test]
fn test_version_short() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.arg("-v");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("pvm"));
}

#[test]
fn test_self_update_help() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("self-update").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--apply"));
}

#[test]
fn test_help_lists_self_update() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("self-update"));
}

#[test]
fn test_ls_non_tty_prints_plain_list() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_installed_version(temp_dir.path(), "8.9.6", &["cli", "fpm"]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("ls");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("latest (8.9.6)"))
        .stdout(predicate::str::contains("cli, fpm"));
}

#[test]
fn test_use_no_version_non_tty_fails_with_hint() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_installed_version(temp_dir.path(), "8.9.6", &["cli"]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_UPDATE_MODE", "disabled");
    cmd.current_dir(temp_dir.path());
    cmd.arg("use");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No version given"));
}

#[test]
fn test_init_non_tty_fails_with_hint() {
    let temp_dir = tempfile::tempdir().unwrap();
    seed_remote_cache(temp_dir.path(), &[("8.9.7", &["cli"])]);
    seed_installed_version(temp_dir.path(), "8.9.6", &["cli"]);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.current_dir(temp_dir.path());
    cmd.arg("init");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("requires a terminal"));
}

#[test]
fn test_ls_empty() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("ls");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No PHP versions installed."));
}

#[test]
fn test_env_bash() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("env").arg("--shell=bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("export PVM_DIR="))
        .stdout(predicate::str::contains("export PATH="));
}

#[test]
fn test_uninstall_not_installed() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("uninstall").arg("9.9.9");
    cmd.assert().failure().stderr(predicate::str::contains(
        "Error: PHP 9.9.9 is not installed locally.",
    ));
}

#[test]
fn test_uninstall_success() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Mock an installed version
    let bin_dir = temp_dir.path().join("versions").join("8.3.1").join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::write(bin_dir.join("php"), "").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("uninstall").arg("8.3.1");
    cmd.assert().success().stdout(predicate::str::contains(
        "Successfully uninstalled PHP 8.3.1",
    ));

    // Verify it actually deleted the folder
    assert!(!temp_dir.path().join("versions").join("8.3.1").exists());
}

#[test]
fn test_use_silent_export() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Mock an installed version
    let bin_dir = temp_dir.path().join("versions").join("8.3.1").join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::write(bin_dir.join("php"), "").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_UPDATE_MODE", "disabled");
    cmd.current_dir(temp_dir.path());
    cmd.arg("use").arg("8.3.1");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("export PVM_MULTISHELL_PATH").not());
}

#[test]
fn test_use_silent_skips_missing_version() {
    let temp_dir = tempfile::tempdir().unwrap();
    let env_file = temp_dir.path().join("custom_env_update");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_UPDATE_MODE", "disabled");
    cmd.env("PVM_ENV_UPDATE_PATH", &env_file);
    cmd.current_dir(temp_dir.path());
    cmd.arg("use").arg("--silent").arg("8.3");

    // Silent mode: missing version exits 0 with no output and no env file written.
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    assert!(
        !env_file.exists(),
        "silent mode must not write env file when version is missing"
    );
}

#[test]
fn test_uninstall_fpm_only_success() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Mock an installed version with ONLY php-fpm (no php)
    let bin_dir = temp_dir.path().join("versions").join("8.3.1").join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::write(bin_dir.join("php-fpm"), "").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.arg("uninstall").arg("8.3.1");
    cmd.assert().success().stdout(predicate::str::contains(
        "Successfully uninstalled PHP 8.3.1",
    ));

    // Verify it actually deleted the folder
    assert!(!temp_dir.path().join("versions").join("8.3.1").exists());
}

#[test]
fn test_use_php_version_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let env_file = temp_dir.path().join("custom_env_update");

    // Write .php-version
    std::fs::write(temp_dir.path().join(".php-version"), "8.3.1\n").unwrap();

    // Mock the installed version
    let bin_dir = temp_dir.path().join("versions").join("8.3.1").join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::write(bin_dir.join("php"), "").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_UPDATE_MODE", "disabled");
    cmd.env("PVM_ENV_UPDATE_PATH", &env_file);
    cmd.current_dir(temp_dir.path());
    cmd.arg("use"); // no version argument

    cmd.assert().success();

    assert!(env_file.exists());
    let env_content = std::fs::read_to_string(env_file).unwrap();
    assert!(env_content.contains("8.3.1"));
}

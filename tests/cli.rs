use predicates::prelude::*;

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
fn test_use_writes_phprc_to_env_update() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Mock an installed version with a php.ini
    let version_dir = temp_dir.path().join("versions").join("8.3.1");
    let bin_dir = version_dir.join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::write(bin_dir.join("php"), "").unwrap();
    std::fs::write(version_dir.join("php.ini"), "; test php.ini").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_UPDATE_MODE", "disabled");
    cmd.current_dir(temp_dir.path());
    cmd.arg("use").arg("8.3.1");
    cmd.assert().success();

    let env_update_path = temp_dir.path().join(".env_update");
    let env_update = std::fs::read_to_string(env_update_path).unwrap();
    assert!(
        env_update.contains("export PVM_MULTISHELL_PATH"),
        ".env_update should contain PVM_MULTISHELL_PATH"
    );
    assert!(
        env_update.contains("PHPRC="),
        ".env_update should contain PHPRC for the version-local php.ini"
    );
}

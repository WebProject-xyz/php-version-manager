use predicates::prelude::*;

#[test]
fn test_xdebug_on_and_off_modify_php_ini() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Mock an installed version with xdebug.so present
    let version_dir = temp_dir.path().join("versions").join("8.3.30");
    let bin_dir = version_dir.join("bin");
    let ext_dir = bin_dir.join("ext");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(bin_dir.join("php"), "").unwrap();
    std::fs::write(ext_dir.join("xdebug.so"), "").unwrap();

    // First, enable xdebug
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_UPDATE_MODE", "disabled");
    cmd.arg("xdebug").arg("on").arg("8.3.30");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Enabled Xdebug for PHP 8.3.30"));

    let php_ini_path = version_dir.join("php.ini");
    let php_ini_contents = std::fs::read_to_string(&php_ini_path).unwrap();
    assert!(
        php_ini_contents.contains("zend_extension="),
        "php.ini should contain a zend_extension line after enabling xdebug"
    );

    // Then, disable xdebug
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("pvm");
    cmd.env("PVM_DIR", temp_dir.path());
    cmd.env("PVM_UPDATE_MODE", "disabled");
    cmd.arg("xdebug").arg("off").arg("8.3.30");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Disabled Xdebug for PHP 8.3.30"));

    let php_ini_contents = std::fs::read_to_string(&php_ini_path).unwrap();
    assert!(
        !php_ini_contents
            .lines()
            .any(|l| l.trim_start().starts_with("zend_extension") && l.contains("xdebug")),
        "php.ini should not contain a zend_extension xdebug line after disabling xdebug"
    );
}

/// The name of the environment variable used to store the path to the PVM installation.
pub const PVM_DIR_VAR: &str = "PVM_DIR";

/// The name of the environment variable used to store the path to the currently active PHP version's bin directory.
pub const MULTISHELL_PATH_VAR: &str = "PVM_MULTISHELL_PATH";

/// The name of the file used to store the environment variables to be updated in the shell.
pub const ENV_UPDATE_FILE: &str = ".env_update";

/// The name of the file used to store the remote versions cache.
pub const REMOTE_CACHE_FILE: &str = "remote_cache.json";

/// The name of the file used as a guard for the update check.
pub const UPDATE_CHECK_GUARD_FILE: &str = ".update_check_guard";

/// The name of the file used to store the PHP version for a directory.
pub const PHP_VERSION_FILE: &str = ".php-version";

/// The base URL for fetching available PHP versions.
pub const BASE_URL: &str = "https://dl.static-php.dev/static-php-cli/common/";

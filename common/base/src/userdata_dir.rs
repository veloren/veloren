use std::path::PathBuf;
use tracing::warn;

const VELOREN_USERDATA_ENV: &str = "VELOREN_USERDATA";

// TODO: consider expanding this to a general install strategy variable that is
// also used for finding assets
// TODO: Ensure there are no NUL (\0) characters in userdata_dir (possible on
// MacOS but not Windows or Linux) as SQLite requires the database path does not
// include this character.
/// # `VELOREN_USERDATA_STRATEGY` environment variable
/// Read during compilation
/// Useful to set when compiling for distribution
/// "system" => system specific project data directory
/// "executable" => <executable dir>/userdata
/// Note: case insensitive

/// Determines common user data directory used by veloren frontends
/// The first specified in this list is used
/// 1. The VELOREN_USERDATA environment variable
/// 2. The VELOREN_USERDATA_STRATEGY environment variable
/// 3. The CARGO_MANIFEST_DIR/userdata or CARGO_MANIFEST_DIR/../userdata
/// depending on if a    workspace if being used
pub fn userdata_dir(workspace: bool, strategy: Option<&str>, manifest_dir: &str) -> PathBuf {
    // 1. The VELOREN_USERDATA environment variable
    std::env::var_os(VELOREN_USERDATA_ENV)
        .map(PathBuf::from)
        // 2. The VELOREN_USERDATA_STRATEGY environment variable
        .or_else(|| match strategy {
            // "system" => system specific project data directory
            Some(s) if s.eq_ignore_ascii_case("system") => Some(directories_next::ProjectDirs::from("net", "veloren", "veloren")
                .expect("System's $HOME directory path not found!")
                .data_dir()
                .join("userdata")
            ),
            // "executable" => <executable dir>/userdata
            Some(s) if s.eq_ignore_ascii_case("executable") => {
                let mut path = std::env::current_exe()
                    .expect("Failed to retrieve executable path!");
                path.pop();
                path.push("userdata");
                Some(path)
            },
            Some(s) => {
                warn!(
                    "Compiled with an invalid VELOREN_USERDATA_STRATEGY: \"{}\". \
                    Valid values are unset, \"system\", and \"executable\". \
                    Falling back to unset case.",
                    s,
                );
                None
            },
            _ => None,
        })
        // 3. The CARGO_MANIFEST_DIR/userdata or CARGO_MANIFEST_DIR/../userdata depending on if a
        //    workspace if being used
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(manifest_dir);
            if workspace {
                path.pop();
            }
            let exe_path = std::env::current_exe()
                .expect("Failed to retrieve executable path!");
            // If this path exists
            // and the binary path is prefixed by this path
            // put the userdata folder there
            if path.exists() && exe_path.starts_with(&path) {
                path.push("userdata");
                path
            } else {
                // otherwise warn and fallback to the executable strategy
                let project_path = path;
                let mut path = exe_path;
                path.pop();
                path.push("userdata");
                warn!(
                    "This binary is outside the project folder where it was compiled ({}) \
                    and was not compiled with VELOREN_USERDATA_STRATEGY set to \"system\" or \"executable\". \
                    Falling back the to the \"executable\" strategy (the userdata folder will be placed in the \
                    same folder as the executable: {}) \n\
                    NOTE: You can manually select a userdata folder (overriding this automatic selection) by \
                    setting the environment variable {} to the desired directory before running. \n\
                    NOTE: If you have not moved the executable this can occur when using a custom cargo \
                    target-dir that is not inside the project folder.",
                    project_path.display(),
                    path.display(),
                    VELOREN_USERDATA_ENV,
                );
                path
            }
        })
}

#[macro_export]
macro_rules! userdata_dir_workspace {
    () => {
        $crate::userdata_dir::userdata_dir(
            true,
            option_env!("VELOREN_USERDATA_STRATEGY"),
            env!("CARGO_MANIFEST_DIR"),
        )
    };
}

#[macro_export]
macro_rules! userdata_dir_no_workspace {
    () => {
        $crate::userdata_dir::userdata_dir(
            false,
            option_env!("VELOREN_USERDATA_STRATEGY"),
            env!("CARGO_MANIFEST_DIR"),
        )
    };
}

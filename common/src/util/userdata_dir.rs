use std::path::PathBuf;

const VELOREN_USERDATA_ENV: &str = "VELOREN_USERDATA";

// TODO: consider expanding this to a general install strategy variable that is
// also used for finding assets
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
            Some(_) => None, // TODO: panic? catch during compilation?
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
            // Ensure this path exists 
            // Ensure that the binary path is prefixed by this path
            if !path.exists() || !exe_path.starts_with(&path) {
                panic!("Recompile with VELOREN_USERDATA_STRATEGY set to \"system\" or \"executable\" to run the binary outside of the project folder");
            }

            path.push("userdata");
            path
        })
}

#[macro_export]
macro_rules! userdata_dir_workspace {
    () => {
        $crate::util::userdata_dir::userdata_dir(
            true,
            option_env!("VELOREN_USERDATA_STRATEGY"),
            env!("CARGO_MANIFEST_DIR"),
        )
    };
}

#[macro_export]
macro_rules! userdata_dir_no_workspace {
    () => {
        $crate::util::userdata_dir::userdata_dir(
            false,
            option_env!("VELOREN_USERDATA_STRATEGY"),
            env!("CARGO_MANIFEST_DIR"),
        )
    };
}

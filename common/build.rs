use std::process::Command;

fn main() {
    // If this env var exists, it'll be used instead
    if option_env!("VELOREN_GIT_VERSION").is_none() {
        // Get the current githash+date
        // Note: It will compare commits. As long as the commits do not diverge from the
        // server no version change will be detected.
        let version = match Command::new("git")
            .args(["log", "-n", "1", "--pretty=format:%h/%ct", "--abbrev=8"])
            .output()
        {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(version) => {
                    let hash = version.split('/').next().expect("no git hash");
                    // The non-collision guarantee isn't all that important for our purposes
                    if hash.len() != 8 {
                        format!(
                            "{}/{}",
                            hash.get(..8).expect("invalid git hash"),
                            version.split('/').nth(1).expect("no git timestamp")
                        )
                    } else {
                        version
                    }
                },
                Err(e) => panic!("failed to convert git output to UTF-8: {}", e),
            },
            Err(e) => panic!("failed to retrieve current git commit hash and date: {}", e),
        };

        // Get the current gittag
        let tag = match Command::new("git")
            .args(["describe", "--exact-match", "--tags", "HEAD"])
            .output()
        {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(tag) => tag,
                Err(e) => panic!("failed to convert git output to UTF-8: {}", e),
            },
            Err(e) => panic!("failed to retrieve current git tag: {}", e),
        };

        println!("cargo::rustc-env=VELOREN_GIT_VERSION={}/{}", &tag, &version);
    }
}

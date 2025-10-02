use std::process::Command;

fn main() {
    // If this env var exists, it'll be used instead
    if option_env!("VELOREN_GIT_VERSION").is_none() {
        // Get the current githash+date
        // Note: It will compare commits. As long as the commits do not diverge from the
        // server no version change will be detected.
        let version = match Command::new("git")
            .args([
                "log",
                "-n",
                "1",
                "--pretty=format:%h/%cd",
                "--date=format:%Y-%m-%d-%H:%M",
                "--abbrev=8",
            ])
            .output()
        {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(version) => version,
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

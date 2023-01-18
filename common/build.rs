use std::{env, fs::File, io::Write, path::Path, process::Command};

fn main() {
    // If these env variables exist then we are building on nix, use them as hash
    // and tag.
    if let (Some(hash), Some(tag)) = (option_env!("NIX_GIT_HASH"), option_env!("NIX_GIT_TAG")) {
        create_hash_file(hash);
        create_tag_file(tag);
    } else {
        // Get the current githash
        // Note: It will compare commits. As long as the commits do not diverge from the
        // server no version change will be detected.
        match Command::new("git")
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
                Ok(hash) => {
                    create_hash_file(&hash);
                },
                Err(e) => panic!("failed to convert git output to UTF-8: {}", e),
            },
            Err(e) => panic!("failed to retrieve current git commit hash: {}", e),
        }

        // Get the current githash
        // Note: It will compare commits. As long as the commits do not diverge from the
        // server no version change will be detected.
        match Command::new("git")
            .args(["describe", "--exact-match", "--tags", "HEAD"])
            .output()
        {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(tag) => {
                    create_tag_file(&tag);
                },
                Err(e) => panic!("failed to convert git output to UTF-8: {}", e),
            },
            Err(e) => panic!("failed to retrieve current git commit hash: {}", e),
        }
    }
}

fn create_hash_file(hash: &str) {
    let mut target = File::create(
        Path::new(&env::var("OUT_DIR").expect("failed to query OUT_DIR environment variable"))
            .join("githash"),
    )
    .expect("failed to create git hash file!");
    target
        .write_all(hash.trim().as_bytes())
        .expect("failed to write to file!");
}

fn create_tag_file(tag: &str) {
    let mut target = File::create(
        Path::new(&env::var("OUT_DIR").expect("failed to query OUT_DIR environment variable"))
            .join("gittag"),
    )
    .expect("failed to create git tag file!");
    target
        .write_all(tag.trim().as_bytes())
        .expect("failed to write to file!");
}

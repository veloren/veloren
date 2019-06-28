use std::process::Command;

fn main() {
    match Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(hash) => println!("cargo:rustc-env=GIT_HASH={}", hash),
            Err(e) => println!("failed to convert git output to UTF-8: {}", e),
        },
        Err(e) => println!("failed to retrieve current git commit hash: {}", e),
    }
}

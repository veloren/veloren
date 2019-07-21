use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() {
    match Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(hash) => {
                let mut target = File::create(
                    Path::new(
                        &env::var("OUT_DIR").expect("failed to query OUT_DIR environment variable"),
                    )
                    .join("githash"),
                )
                .expect("failed to create git hash file!");
                target
                    .write_all(hash.trim().as_bytes())
                    .expect("failed to write to file!");
            }
            Err(e) => panic!("failed to convert git output to UTF-8: {}", e),
        },
        Err(e) => panic!("failed to retrieve current git commit hash: {}", e),
    }
}

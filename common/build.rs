use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Get the current githash
    match Command::new("git")
        .args(&[
            "log",
            "-n",
            "1",
            "--pretty=format:%h %cd",
            "--date=format:%Y-%m-%d-%H:%M",
        ])
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
    // Check if git-lfs is working
    if std::env::var("DISABLE_GIT_LFS_CHECK").is_err() {
        let asset_path: PathBuf = ["..", "assets", "voxygen", "background", "bg_main.png"]
            .iter()
            .collect();
        let asset_file = match File::open(&asset_path) {
            Ok(file) => file,
            Err(e) => panic!(
                "failed to open asset file {}: {}",
                asset_path.to_str().unwrap(),
                e
            ),
        };
        const LFS_MARKER: &[u8] = b"version https://git-lfs.github.com/spec/";
        let mut buffer = Vec::new();
        let bytes_read = asset_file
            .take(LFS_MARKER.len() as u64)
            .read_to_end(&mut buffer)
            .expect("failed to read asset file");

        if bytes_read == LFS_MARKER.len() && buffer == LFS_MARKER {
            panic!(
                "\n\nGit Large File Storage (git-lfs) has not been set up correctly.\n\
                 Most common reasons:\n\
                 \t- git-lfs was not installed before cloning this repository\n\
                 \t- this repository was not cloned from the primary gitlab mirror.\n\
                 \t  The github mirror does not support lfs.\n\
                 See the book at https://book.veloren.net/ for details.\n\n"
            );
        }
    }
}

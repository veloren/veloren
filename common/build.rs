use regex::Regex;
use std::process::Command;

// Get the current githash+timestamp
// Note: It will compare commits. As long as the commits do not diverge from the
// server no version change will be detected.
fn get_git_hash_timestamp() -> Result<String, String> {
    let output = Command::new("git")
        .args(["log", "-n", "1", "--pretty=format:%h/%ct", "--abbrev=8"])
        .output()
        .map_err(|e| format!("Git version command couldn't be run with error: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Git version command was unsuccessful: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let hash_timestamp = String::from_utf8(output.stdout)
        .map_err(|e| format!("Git version command output isn't valid UTF-8: {}", e))?;
    let hash = hash_timestamp
        .split('/')
        .next()
        .ok_or("Git hash not found".to_string())?;
    // The non-collision guarantee isn't all that important for our purposes
    if hash.len() != 8 {
        Ok(format!(
            "{}/{}",
            hash.get(..8)
                .ok_or("Git hash not long enough".to_string())?,
            hash_timestamp
                .split('/')
                .nth(1)
                .ok_or("Git timestamp not found".to_string())?
        ))
    } else {
        Ok(hash_timestamp)
    }
}

// Get the current gittag
fn get_git_tag() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let tag = String::from_utf8(output.stdout).ok()?;

    if Regex::new(r"/^v[0-9]+\.[0-9]+\.[0-9]+$/")
        .unwrap()
        .is_match(&tag)
    {
        Some(tag)
    } else {
        None
    }
}

fn main() {
    // If this env var exists, it'll be used instead
    if option_env!("VELOREN_GIT_VERSION").is_none() {
        let hash_timestamp = match get_git_hash_timestamp() {
            Ok(hash_timestamp) => hash_timestamp,
            Err(e) => {
                println!("cargo::warning={}", e);
                println!(
                    "cargo::warning=Veloren will be compiled with git hash and timestamp set to \
                     0, which will cause version mismatch warnings where applicable, whether the \
                     version is actually mismatched or not. It is highly recommended to build the \
                     game from the cloned git repository with the git command available in order \
                     to give Veloren access to proper versioning information."
                );
                println!("cargo::rustc-env=VELOREN_GIT_VERSION=/0/0");
                return;
            },
        };

        let tag = get_git_tag().unwrap_or("".to_string());

        // Format: <git-tag?>/<git-hash>/<git-timestamp>
        println!(
            "cargo::rustc-env=VELOREN_GIT_VERSION={}/{}",
            &tag, &hash_timestamp
        );
    }
}

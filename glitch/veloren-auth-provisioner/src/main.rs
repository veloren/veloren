use authc::{AuthClient, Authority, Scheme};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf, time::Duration};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
struct GlitchValidateResponse {
    valid: Option<bool>,
    user_name: Option<String>,
    reason: Option<String>,
    error: Option<String>,
}

fn env_required(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("missing required env var {name}"))
}

fn env_default(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn slugify_username(raw: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;

    for ch in raw.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else if ch == '-' || ch == '_' || ch.is_whitespace() || ch == '.' {
            '_'
        } else {
            '_'
        };

        if mapped == '_' || mapped == '-' {
            if !last_sep {
                out.push('_');
                last_sep = true;
            }
        } else {
            out.push(mapped);
            last_sep = false;
        }
    }

    let trimmed = out.trim_matches('_').trim_matches('-').to_string();
    if trimmed.len() >= 3 { trimmed } else { "glitch".to_string() }
}

fn hex_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn derive_password(secret: &str, title_id: &str, install_id: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(b"veloren-official-auth-v1:");
    mac.update(title_id.as_bytes());
    mac.update(b":");
    mac.update(install_id.as_bytes());
    let raw = mac.finalize().into_bytes();
    format!("glt_{}", URL_SAFE_NO_PAD.encode(raw))
}

fn candidate_usernames(glitch_user_name: &str, title_id: &str, install_id: &str) -> Vec<String> {
    let slug = slugify_username(glitch_user_name);
    let hash = hex_hash(&format!("{title_id}:{install_id}"));
    let suffix10 = &hash[..10];
    let suffix14 = &hash[..14];
    let suffix24 = &hash[..24];
    let suffix30 = &hash[..30];

    let max_slug_len = 32usize.saturating_sub(1 + suffix10.len());
    let mut primary_slug = slug.chars().take(max_slug_len).collect::<String>();
    primary_slug = primary_slug.trim_matches('_').trim_matches('-').to_string();
    if primary_slug.len() < 3 {
        primary_slug = "glitch".to_string();
    }

    let mut candidates = vec![
        format!("{}-{}", primary_slug, suffix10),
        format!("glitch-{}", suffix24),
        format!("g{}", suffix30),
        format!("gl-{}", suffix14),
    ];

    candidates.retain(|u| (3..=32).contains(&u.len()) && u.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    candidates.dedup();
    candidates
}

fn validate_glitch_install(api_base: &str, title_id: &str, title_token: &str, install_id: &str) -> Result<GlitchValidateResponse, String> {
    let url = format!("{}/titles/{}/installs/{}/validate", api_base.trim_end_matches('/'), title_id, install_id);
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    let resp = client
        .post(&url)
        .bearer_auth(title_token)
        .json(&serde_json::json!({}))
        .send()
        .map_err(|e| format!("Glitch validate request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    let parsed: GlitchValidateResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Glitch validate response was not expected JSON: status={status}, error={e}, body={body}"))?;

    let valid = parsed.valid.unwrap_or(status.is_success());
    if !status.is_success() || !valid {
        let reason = parsed.reason.clone().or(parsed.error.clone()).unwrap_or(body);
        return Err(format!("Glitch install validation denied: status={status}, reason={reason}"));
    }

    Ok(parsed)
}

fn parse_auth_url(auth_base: &str) -> Result<(Scheme, Authority), String> {
    let trimmed = auth_base.trim().trim_end_matches('/');
    let (scheme_raw, rest) = trimmed
        .split_once("://")
        .ok_or_else(|| format!("invalid Veloren auth server URL '{auth_base}': expected scheme://host"))?;
    let authority_raw = rest
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("invalid Veloren auth server URL '{auth_base}': missing host"))?;

    let scheme: Scheme = scheme_raw
        .parse()
        .map_err(|e| format!("invalid Veloren auth URL scheme '{scheme_raw}': {e}"))?;
    let authority: Authority = authority_raw
        .parse()
        .map_err(|e| format!("invalid Veloren auth URL authority '{authority_raw}': {e}"))?;

    Ok((scheme, authority))
}

async fn auth_register_or_login(auth_base: &str, candidates: &[String], password: &str) -> Result<String, String> {
    let (scheme, authority) = parse_auth_url(auth_base)?;
    let auth = AuthClient::new(scheme, authority)
        .map_err(|e| format!("invalid Veloren auth server URL {auth_base}: {e}"))?;

    for username in candidates {
        let register_result = auth.register(username.as_str(), password).await;
        if let Err(register_err) = &register_result {
            eprintln!(
                "candidate username '{}' register request returned error: {}",
                username, register_err
            );
        }

        // The pinned Veloren authc client returns Ok for some non-success register
        // responses, so sign-in is the authoritative verification step.
        match auth.sign_in(username.as_str(), password).await {
            Ok(_) => return Ok(username.clone()),
            Err(login_err) => {
                eprintln!(
                    "candidate username '{}' was unavailable or unusable; register_result='{:?}'; login_error='{}'",
                    username, register_result, login_err
                );
            }
        }
    }

    Err("all deterministic Veloren account username candidates failed".to_string())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut env_file: Option<PathBuf> = None;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--env-file" => {
                i += 1;
                env_file = Some(PathBuf::from(args.get(i).ok_or("--env-file requires a path")?));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    let api_base = env_default("GLITCH_API_BASE_URL", "https://api.glitch.fun/api");
    let title_id = env_required("GLITCH_TITLE_ID")?;
    let title_token = env_required("GLITCH_TITLE_TOKEN")?;
    let install_id = env_required("GLITCH_INSTALL_ID")?;
    let auth_base = env_default("VELOREN_AUTH_SERVER_URL", "https://auth.veloren.net");
    let password_secret = env::var("VELOREN_AUTH_PASSWORD_SECRET")
        .or_else(|_| env::var("GLITCH_SHARED_PASSWORD"))
        .map_err(|_| "VELOREN_AUTH_PASSWORD_SECRET or GLITCH_SHARED_PASSWORD is required".to_string())?;

    let validation = validate_glitch_install(&api_base, &title_id, &title_token, &install_id)?;
    let glitch_user_name = validation
        .user_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&install_id);

    let password = derive_password(&password_secret, &title_id, &install_id);
    let candidates = candidate_usernames(glitch_user_name, &title_id, &install_id);
    if candidates.is_empty() {
        return Err("no legal Veloren username candidates were generated".to_string());
    }

    let username = auth_register_or_login(&auth_base, &candidates, &password).await?;

    let content = format!(
        "VELOREN_USERNAME={}\nVELOREN_PASSWORD={}\nVELOREN_GLITCH_ORIGINAL_INSTALL_ID={}\nVELOREN_GLITCH_DISPLAY_NAME={}\nVELOREN_AUTH_SERVER_URL={}\n",
        shell_quote(&username),
        shell_quote(&password),
        shell_quote(&install_id),
        shell_quote(glitch_user_name),
        shell_quote(&auth_base),
    );

    if let Some(path) = env_file {
        fs::write(&path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    } else {
        print!("{content}");
    }

    eprintln!(
        "Provisioned Veloren auth account username='{}' from Glitch user_name='{}' install_id='{}' auth_server='{}'",
        username, glitch_user_name, install_id, auth_base
    );

    Ok(())
}

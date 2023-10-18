use axum::{
    extract::{Query, State},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::DateTime;
use hyper::{Request, StatusCode};
use serde::{Deserialize, Deserializer};
use server::chat::ChatCache;
use std::str::FromStr;

/// Keep Size small, so we dont have to Clone much for each request.
#[derive(Clone)]
struct ChatToken {
    secret_token: Option<String>,
}

async fn validate_secret<B>(
    State(token): State<ChatToken>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // check if this endpoint is disabled
    let secret_token = token.secret_token.ok_or(StatusCode::METHOD_NOT_ALLOWED)?;

    pub const X_SECRET_TOKEN: &str = "X-Secret-Token";
    let session_cookie = req
        .headers()
        .get(X_SECRET_TOKEN)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if session_cookie.as_bytes() != secret_token.as_bytes() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

pub fn router(cache: ChatCache, secret_token: Option<String>) -> Router {
    let token = ChatToken { secret_token };
    Router::new()
        .route("/history", get(history))
        .layer(axum::middleware::from_fn_with_state(token, validate_secret))
        .with_state(cache)
}

#[derive(Debug, Deserialize)]
struct Params {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    /// To be used to get all messages without duplicates nor losing messages
    from_time_exclusive_rfc3339: Option<String>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: core::fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s)
            .map_err(serde::de::Error::custom)
            .map(Some),
    }
}

async fn history(
    State(cache): State<ChatCache>,
    Query(params): Query<Params>,
) -> Result<impl IntoResponse, StatusCode> {
    // first validate parameters before we take lock
    let from_time_exclusive = if let Some(rfc3339) = params.from_time_exclusive_rfc3339 {
        Some(DateTime::parse_from_rfc3339(&rfc3339).map_err(|_| StatusCode::BAD_REQUEST)?)
    } else {
        None
    };

    let messages = cache.messages.lock().await;
    let filtered: Vec<_> = match from_time_exclusive {
        Some(from_time_exclusive) => messages
            .iter()
            .filter(|msg| msg.time > from_time_exclusive)
            .cloned()
            .collect(),
        None => messages.iter().cloned().collect(),
    };
    Ok(Json(filtered))
}

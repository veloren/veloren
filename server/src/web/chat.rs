use axum::{
    extract::{Query, State},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::comp::ChatMsg;
use hyper::{Request, StatusCode};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    borrow::Cow, collections::VecDeque, mem::size_of, ops::Sub, str::FromStr, sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

/// The chat cache gets it data from the gameserver and will keep it for some
/// time It will be made available for its consumers, the REST Api
#[derive(Clone)]
pub struct ChatCache {
    messages: Arc<Mutex<VecDeque<ChatMessage>>>,
}

/// Keep Size small, so we dont have to Clone much for each request.
#[derive(Clone)]
struct ChatToken {
    secret_token: Cow<'static, str>,
}

pub struct ChatExporter {
    messages: Arc<Mutex<VecDeque<ChatMessage>>>,
    keep_duration: chrono::Duration,
}

impl ChatExporter {
    pub fn send(&self, msg: ChatMsg) {
        let time = Utc::now();
        let drop_older_than = time.sub(self.keep_duration);
        let mut messages = self.messages.blocking_lock();
        while let Some(msg) = messages.front() && msg.time < drop_older_than {
            messages.pop_front();
        }
        messages.push_back(ChatMessage { time, msg });
        const MAX_CACHE_BYTES: usize = 10_000_000; // approx. because HashMap allocates on Heap
        if messages.len() * size_of::<ChatMessage>() > MAX_CACHE_BYTES {
            let msg_count = messages.len();
            tracing::debug!(?msg_count, "shrinking cache");
            messages.shrink_to_fit();
        }
    }
}

impl ChatCache {
    pub fn new(keep_duration: Duration) -> (Self, ChatExporter) {
        let messages: Arc<Mutex<VecDeque<ChatMessage>>> = Default::default();
        let messages_clone = Arc::clone(&messages);
        let keep_duration = chrono::Duration::from_std(keep_duration).unwrap();

        (Self { messages }, ChatExporter {
            messages: messages_clone,
            keep_duration,
        })
    }
}

async fn validate_secret<B>(
    State(token): State<ChatToken>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    pub const X_SECRET_TOKEN: &str = "X-Secret-Token";
    let session_cookie = req
        .headers()
        .get(X_SECRET_TOKEN)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if session_cookie.as_bytes() != token.secret_token.as_bytes() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

pub fn router(cache: ChatCache, secret_token: String) -> Router {
    let token = ChatToken {
        secret_token: Cow::Owned(secret_token),
    };
    Router::new()
        .route("/history", get(history))
        .layer(axum::middleware::from_fn_with_state(token, validate_secret))
        .with_state(cache)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    time: DateTime<Utc>,
    msg: ChatMsg,
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

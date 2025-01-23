use crate::cli::{Message, MessageReturn};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Request, State},
    http::header::COOKIE,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use hyper::StatusCode;
use serde::Deserialize;
use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::sync::Mutex;

/// Keep Size small, so we dont have to Clone much for each request.
#[derive(Clone)]
struct UiApiToken {
    secret_token: String,
}

pub(crate) type UiRequestSender =
    tokio::sync::mpsc::Sender<(Message, tokio::sync::oneshot::Sender<MessageReturn>)>;

#[derive(Clone, Default)]
struct IpAddresses {
    users: Arc<Mutex<HashSet<IpAddr>>>,
}

async fn validate_secret(
    State(token): State<UiApiToken>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let session_cookie = req.headers().get(COOKIE).ok_or(StatusCode::UNAUTHORIZED)?;

    pub const X_SECRET_TOKEN: &str = "X-Secret-Token";
    let expected = format!("{X_SECRET_TOKEN}={}", token.secret_token);

    if session_cookie.as_bytes() != expected.as_bytes() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

/// Logs each new IP address that accesses this API authenticated
async fn log_users(
    State(ip_addresses): State<IpAddresses>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut ip_addresses = ip_addresses.users.lock().await;
    let ip_addr = addr.ip();
    if !ip_addresses.contains(&ip_addr) {
        ip_addresses.insert(ip_addr);
        let users_so_far = ip_addresses.len();
        tracing::info!(?ip_addr, ?users_so_far, "Is accessing the /ui_api endpoint");
    }
    Ok(next.run(req).await)
}

//TODO: do security audit before we extend this api with more security relevant
// functionality (e.g. account management)
pub fn router(web_ui_request_s: UiRequestSender, secret_token: String) -> Router {
    let token = UiApiToken { secret_token };
    let ip_addrs = IpAddresses::default();
    Router::new()
        .route("/players", get(players))
        .route("/logs", get(logs))
        .route("/send_global_msg", post(send_global_msg))
        .layer(axum::middleware::from_fn_with_state(ip_addrs, log_users))
        .layer(axum::middleware::from_fn_with_state(token, validate_secret))
        .with_state(web_ui_request_s)
}

async fn players(
    State(web_ui_request_s): State<UiRequestSender>,
) -> Result<impl IntoResponse, StatusCode> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let _ = web_ui_request_s.send((Message::ListPlayers, sender)).await;
    match receiver
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        MessageReturn::Players(players) => Ok(Json(players)),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn logs(
    State(web_ui_request_s): State<UiRequestSender>,
) -> Result<impl IntoResponse, StatusCode> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let _ = web_ui_request_s.send((Message::ListLogs, sender)).await;
    match receiver
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        MessageReturn::Logs(logs) => Ok(Json(logs)),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct SendWorldMsgBody {
    msg: String,
}

async fn send_global_msg(
    State(web_ui_request_s): State<UiRequestSender>,
    Json(payload): Json<SendWorldMsgBody>,
) -> Result<impl IntoResponse, StatusCode> {
    let (dummy_s, _) = tokio::sync::oneshot::channel();
    let _ = web_ui_request_s
        .send((Message::SendGlobalMsg { msg: payload.msg }, dummy_s))
        .await;
    Ok(())
}

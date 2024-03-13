use axum::{
    extract::{ConnectInfo, State},
    http::{header::SET_COOKIE, HeaderValue},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::net::SocketAddr;

pub mod api;

/// Keep Size small, so we dont have to Clone much for each request.
#[derive(Clone)]
struct UiApiToken {
    secret_token: String,
}

pub fn router(secret_token: String) -> Router {
    let token = UiApiToken { secret_token };
    Router::new().route("/", get(ui)).with_state(token)
}

async fn ui(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(token): State<UiApiToken>,
) -> impl IntoResponse {
    if !addr.ip().is_loopback() {
        return Html(
            r#"<!DOCTYPE html>
<html>
<body>
Ui is only accissable from 127.0.0.1
</body>
</html>
        "#
            .to_string(),
        )
        .into_response();
    }

    let js = include_str!("./ui.js");
    let css = include_str!("./ui.css");
    let inner = include_str!("./ui.html");

    let mut response = Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
<script type="text/javascript">
{js}
</script>
<style>
{css}
</style>
</head>
<body>
{inner}
</body>
</html>"#
    ))
    .into_response();

    let cookie = format!("X-Secret-Token={}; SameSite=Strict", token.secret_token);

    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("An invalid secret-token for ui was provided"),
    );
    response
}

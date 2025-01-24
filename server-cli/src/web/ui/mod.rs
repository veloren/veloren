use axum::{
    Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, header::SET_COOKIE},
    response::{Html, IntoResponse},
    routing::get,
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
    headers: HeaderMap,
    State(token): State<UiApiToken>,
) -> impl IntoResponse {
    const X_FORWARDED_FOR: &'_ str = "X-Forwarded-For";
    if !addr.ip().is_loopback()
        || headers.contains_key(axum::http::header::FORWARDED)
        || headers.contains_key(X_FORWARDED_FOR)
    {
        return Html(
            r#"<!DOCTYPE html>
<html>
<body>
Ui is only accessible from 127.0.0.1. Usage of proxies is forbidden.
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

    //Note: at this point we give a user our secret for the Api, this is only
    // intended for local users, protect this route against the whole internet
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("An invalid secret-token for ui was provided"),
    );
    response
}

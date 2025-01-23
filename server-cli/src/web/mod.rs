use crate::web::ui::api::UiRequestSender;
use axum::{Router, body::Bytes, extract::State, response::IntoResponse, routing::get};
use core::{future::Future, ops::Deref};
use http_body_util::Full;
use hyper::{StatusCode, header, http};
use prometheus::{Registry, TextEncoder};
use server::chat::ChatCache;
use std::{future::IntoFuture, net::SocketAddr};

mod chat;
mod ui;

pub async fn run<S, F, R>(
    registry: R,
    cache: ChatCache,
    chat_secret: Option<String>,
    ui_secret: String,
    web_ui_request_s: UiRequestSender,
    addr: S,
    shutdown: F,
) -> Result<(), hyper::Error>
where
    S: Into<SocketAddr>,
    F: Future<Output = ()> + Send,
    R: Deref<Target = Registry> + Clone + Send + Sync + 'static,
{
    let metrics = Router::new()
        .route("/", get(metrics))
        .with_state(registry.deref().clone());

    let app = Router::new()
        .nest("/chat/v1", chat::router(cache, chat_secret))
        .nest(
            "/ui_api/v1",
            ui::api::router(web_ui_request_s, ui_secret.clone()),
        )
        .nest("/ui", ui::router(ui_secret))
        .nest("/metrics", metrics)
        .route("/health", get(|| async {}));

    // run it
    let addr = addr.into();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("can't bind to web-port.");
    tracing::info!("listening on {}", addr);
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .into_future();
    let res = tokio::select! {
        res = server => res,
        _ = shutdown => Ok(()),
    };
    match res {
        Ok(_) => tracing::debug!("webserver shutdown successful"),
        Err(e) => tracing::error!(?e, "webserver shutdown error"),
    }

    Ok(())
}

async fn metrics(State(registry): State<Registry>) -> Result<impl IntoResponse, StatusCode> {
    use prometheus::Encoder;
    let mf = registry.gather();
    let mut buffer = Vec::with_capacity(1024);

    let encoder = TextEncoder::new();
    encoder
        .encode(&mf, &mut buffer)
        .expect("write to vec cannot fail");

    let bytes: Bytes = buffer.into();

    match http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(bytes))
    {
        Err(e) => {
            tracing::warn!(?e, "could not export metrics to HTTP format");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
        Ok(r) => Ok(r),
    }
}

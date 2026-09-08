use std::net::SocketAddr;

use anyhow::Context;
use axum::{Router, routing};
use tokio::net::TcpListener;
use tokio::signal;

pub mod database;
pub mod v1;

pub async fn start(bind: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind)
        .await
        .context("Could not start listening for incoming registry server connections.")?;

    let router = router();

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        signal::ctrl_c()
            .await
            .expect("Could not wait for CTRL + C.");
    })
    .await
    .context("Failed to serve requests for registry server.")?;

    Ok(())
}

fn router() -> Router<()> {
    Router::new()
        .route("/v1/signin", routing::post(v1::routes::signin::handle))
        .route("/v1/signup", routing::post(v1::routes::signup::handle))
}

pub async fn run(args: &Args, cmd: &RunSubcommandArgs) -> anyhow::Result<()> {
    Ok(())
}

pub async fn start(bind: SocketAddr, state: RegistryState) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind)
        .await
        .context("Could not start listening for incoming registry server connections.")?;

    let task_token_cleanup = tokio::spawn(tasks::token_cleanup::run(state.clone()));

    let router = router(state);

    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            signal::ctrl_c()
                .await
                .expect("Could not wait for CTRL + C.");
        })
        .await
        .context("Failed to serve requests for registry server.")?;

    task_token_cleanup.abort();

    Ok(())
}

fn router(state: RegistryState) -> Router {
    Router::new()
        .nest("/storage", state.storage.router())
        .route("/v1/config", routing::get(v1::routes::config::handle))
        .route("/v1/signin", routing::post(v1::routes::signin::handle))
        .route("/v1/signup", routing::post(v1::routes::signup::handle))
        .route("/v1/bundles", routing::get(v1::routes::bundles_list::handle))
        .route("/v1/bundles", routing::post(v1::routes::bundles_create::handle))
        .layer(axum::middleware::from_fn(middleware::logging::handle))
        .with_state(state)
}

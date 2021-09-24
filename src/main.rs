use std::net::SocketAddr;

use axum::{
    body::Body,
    handler::{get, post},
    response::Html,
    http::{Request, StatusCode},
    Router
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{
    add_extension::AddExtensionLayer, auth::RequireAuthorizationLayer,
    compression::CompressionLayer, trace::TraceLayer,
};
use tracing::{debug, info};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "mo_yu_in_clb=debug,tower_http=debug")
    }
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/square", post(square_handle))
        .layer(
            ServiceBuilder::new()
                .load_shed()
                .concurrency_limit(1024)
                .timeout(std::time::Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        );

    let port = std::env::var("PORT")
        .unwrap_or("5000".to_string())
        .parse::<u16>()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn root() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

#[tracing::instrument]
async fn square_handle(body: String) -> Result<String, StatusCode> {
    info!("called");
    let x = body.parse::<i64>().map_err(|_e| StatusCode::BAD_REQUEST)?;
    x.checked_mul(x).map(|r| r.to_string()).ok_or(StatusCode::BAD_REQUEST)
}

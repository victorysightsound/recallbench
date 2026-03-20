pub mod routes;

use std::path::PathBuf;

use anyhow::Result;
use axum::Router;
use rust_embed::Embed;
use tower_http::cors::CorsLayer;

#[derive(Embed)]
#[folder = "src/web/static/"]
struct StaticAssets;

/// Start the web UI server.
pub async fn serve(port: u16, results_dir: PathBuf) -> Result<()> {
    let app = Router::new()
        .merge(routes::api_routes(results_dir))
        .merge(static_routes())
        .layer(CorsLayer::permissive());

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("RecallBench UI running at http://{addr}");

    // Try to open browser
    let url = format!("http://localhost:{port}");
    let _ = open::that(&url);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn static_routes() -> Router {
    use axum::response::{Html, IntoResponse, Response};
    use axum::http::StatusCode;
    use axum::routing::get;

    async fn serve_index() -> impl IntoResponse {
        match StaticAssets::get("index.html") {
            Some(file) => Html(String::from_utf8_lossy(file.data.as_ref()).to_string()).into_response(),
            None => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
        }
    }

    async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> impl IntoResponse {
        match StaticAssets::get(&path) {
            Some(file) => {
                let mime = if path.ends_with(".js") { "application/javascript" }
                    else if path.ends_with(".css") { "text/css" }
                    else { "application/octet-stream" };
                Response::builder()
                    .header("content-type", mime)
                    .body(axum::body::Body::from(file.data.to_vec()))
                    .unwrap()
                    .into_response()
            }
            None => (StatusCode::NOT_FOUND, "Not found").into_response(),
        }
    }

    Router::new()
        .route("/", get(serve_index))
        .route("/static/{*path}", get(serve_static))
}

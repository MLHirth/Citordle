mod city;
mod game;
mod loader;
mod models;
mod routes;
mod session;

use std::{env, net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::{http::HeaderValue, routing::get, Router};
use game::GameService;
use loader::{load_allowed_words, load_cities};
use session::SessionManager;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<()> {
    let cities = load_cities()?;
    let allowed_words = load_allowed_words()?;
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "citordle-dev-secret".to_string());
    let session_manager = SessionManager::new(&jwt_secret);
    let game_service = Arc::new(GameService::new(cities, allowed_words, session_manager));

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .nest("/api", routes::game::router())
        .with_state(game_service)
        .layer(build_cors());

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Citordle backend running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_cors() -> CorsLayer {
    match env::var("FRONTEND_ORIGIN") {
        Ok(origin) => match origin.parse::<HeaderValue>() {
            Ok(value) => CorsLayer::new()
                .allow_origin(value)
                .allow_methods(Any)
                .allow_headers(Any),
            Err(_) => CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        },
        Err(_) => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    }
}

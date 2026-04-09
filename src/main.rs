mod cache;
mod handlers;
mod providers;

use std::env;

use axum::{Router, routing::get};
use dotenvy::dotenv;
use reqwest::Client;
use tracing::info;

use crate::cache::SharedCache;

#[derive(Clone)]
pub struct AppState {
    pub cache: SharedCache,
    pub client: Client,
    pub openweathermap_api_key: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let openweathermap_api_key =
        env::var("OPENWEATHERMAP_API_KEY").expect("OPENWEATHERMAP_API_KEY must be set");

    let port = env::var("PORT")
        .map(|value| value.parse::<u16>().expect("PORT must be a valid u16"))
        .unwrap_or(3000);

    let state = AppState {
        cache: cache::new_cache(),
        client: Client::new(),
        openweathermap_api_key,
    };

    let app = Router::new()
        .route("/geocode", get(handlers::geocode))
        .route("/weather/{loc}", get(handlers::weather))
        .with_state(state);

    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("failed to bind listener");

    info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .expect("server exited with error");
}

mod cache;
mod handlers;
mod providers;

use std::{env, time::Duration};

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
    pub open_meteo_timeout: Duration,
    pub openweathermap_timeout: Duration,
}

const DEFAULT_WEATHER_PROVIDER_TIMEOUT_MS: u64 = 2000;

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
    let open_meteo_timeout =
        duration_from_env("OPEN_METEO_TIMEOUT_MS", DEFAULT_WEATHER_PROVIDER_TIMEOUT_MS);
    let openweathermap_timeout = duration_from_env(
        "OPENWEATHERMAP_TIMEOUT_MS",
        DEFAULT_WEATHER_PROVIDER_TIMEOUT_MS,
    );

    let port = env::var("PORT")
        .map(|value| value.parse::<u16>().expect("PORT must be a valid u16"))
        .unwrap_or(3000);

    let state = AppState {
        cache: cache::new_cache(),
        client: Client::new(),
        openweathermap_api_key,
        open_meteo_timeout,
        openweathermap_timeout,
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

fn duration_from_env(key: &str, default_ms: u64) -> Duration {
    let timeout_ms = env::var(key)
        .ok()
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(
                    trimmed
                        .parse::<u64>()
                        .unwrap_or_else(|_| panic!("{key} must be a valid u64")),
                )
            }
        })
        .unwrap_or(default_ms);

    Duration::from_millis(timeout_ms)
}

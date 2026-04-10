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
    pub weather_cache_ttl: Duration,
}

const DEFAULT_WEATHER_PROVIDER_TIMEOUT_MS: u64 = 2000;
const DEFAULT_WEATHER_CACHE_TTL_SECONDS: u64 = 60 * 60;

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
        duration_ms_from_env("OPEN_METEO_TIMEOUT_MS", DEFAULT_WEATHER_PROVIDER_TIMEOUT_MS);
    let openweathermap_timeout = duration_ms_from_env(
        "OPENWEATHERMAP_TIMEOUT_MS",
        DEFAULT_WEATHER_PROVIDER_TIMEOUT_MS,
    );
    let weather_cache_ttl = duration_seconds_from_env(
        "WEATHER_CACHE_TTL_SECONDS",
        DEFAULT_WEATHER_CACHE_TTL_SECONDS,
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
        weather_cache_ttl,
    };

    let app = Router::new()
        .route("/geocode", get(handlers::geocode))
        .route("/weather", get(handlers::weather_query))
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

fn duration_ms_from_env(key: &str, default_ms: u64) -> Duration {
    Duration::from_millis(u64_from_env(key, default_ms))
}

fn duration_seconds_from_env(key: &str, default_seconds: u64) -> Duration {
    Duration::from_secs(u64_from_env(key, default_seconds))
}

fn u64_from_env(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .map(|value| parse_u64_env_value(key, Some(&value), default))
        .unwrap_or(default)
}

fn parse_u64_env_value(key: &str, value: Option<&str>, default: u64) -> u64 {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(|trimmed| {
            trimmed
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("{key} must be a valid u64"))
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::parse_u64_env_value;

    #[test]
    fn parse_u64_env_value_uses_default_when_missing() {
        assert_eq!(parse_u64_env_value("TEST_KEY", None, 3600), 3600);
    }

    #[test]
    fn parse_u64_env_value_uses_default_when_blank() {
        assert_eq!(parse_u64_env_value("TEST_KEY", Some("   "), 3600), 3600);
    }

    #[test]
    fn parse_u64_env_value_parses_number() {
        assert_eq!(parse_u64_env_value("TEST_KEY", Some("7200"), 3600), 7200);
    }
}

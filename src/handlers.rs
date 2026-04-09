use std::time::Duration;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppState, cache,
    providers::{self, WeatherResponse},
};

const WEATHER_TTL: Duration = Duration::from_secs(60 * 60);
const WEATHER_STALE_WINDOW: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Deserialize)]
pub struct GeocodeQuery {
    q: String,
}

pub async fn geocode(
    State(state): State<AppState>,
    Query(params): Query<GeocodeQuery>,
) -> Response {
    let query = normalize_query(&params.q);

    if let Some(body) = cache::get_fresh(&state.cache, &query).await {
        return json_body(StatusCode::OK, body);
    }

    match providers::fetch_geocode(&state.client, &query).await {
        Ok(body) => {
            cache::set(&state.cache, query, body.clone(), None).await;
            json_body(StatusCode::OK, body)
        }
        Err(()) => error_json(StatusCode::SERVICE_UNAVAILABLE, "service unavailable"),
    }
}

pub async fn weather(State(state): State<AppState>, Path(loc): Path<String>) -> Response {
    let Some(location) = providers::location_for(&loc) else {
        return error_json(StatusCode::BAD_REQUEST, "unknown location");
    };

    let cache_key = format!("weather:{}", location.key);

    if let Some(body) = cache::get_fresh(&state.cache, &cache_key).await {
        return json_body(StatusCode::OK, body);
    }

    match providers::fetch_open_meteo(&state.client, location).await {
        Ok(weather) => {
            let body = weather_json(&weather);
            cache::set(&state.cache, cache_key, body.clone(), Some(WEATHER_TTL)).await;
            json_body(StatusCode::OK, body)
        }
        Err(()) => {
            tracing::warn!(
                location = location.key,
                "primary weather provider failed, using fallback"
            );

            match providers::fetch_openweathermap(
                &state.client,
                location,
                &state.openweathermap_api_key,
            )
            .await
            {
                Ok(weather) => {
                    let body = weather_json(&weather);
                    cache::set(&state.cache, cache_key, body.clone(), Some(WEATHER_TTL)).await;
                    json_body(StatusCode::OK, body)
                }
                Err(()) => {
                    if let Some(body) =
                        cache::get_stale(&state.cache, &cache_key, WEATHER_STALE_WINDOW).await
                    {
                        return json_body(StatusCode::OK, body);
                    }

                    error_json(StatusCode::SERVICE_UNAVAILABLE, "service unavailable")
                }
            }
        }
    }
}

fn normalize_query(query: &str) -> String {
    query.trim().to_string()
}

fn weather_json(weather: &WeatherResponse) -> String {
    serde_json::to_string(weather).expect("weather response serialization should not fail")
}

fn json_body(status: StatusCode, body: String) -> Response {
    let mut response = (status, body).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    response
}

fn error_json(status: StatusCode, message: &'static str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

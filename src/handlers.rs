use std::time::Duration;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use tracing::{info, warn};

use crate::{
    AppState, cache,
    providers::{self, WeatherResponse},
};

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
    info!(endpoint = "/geocode", query, "request received");

    if let Some(body) = cache::get_fresh(&state.cache, &query).await {
        info!(
            endpoint = "/geocode",
            query,
            source = "cache",
            status = StatusCode::OK.as_u16(),
            "request served"
        );
        return json_body(StatusCode::OK, body);
    }

    match providers::fetch_geocode(&state.client, &query).await {
        Ok(body) => {
            cache::set(&state.cache, query, body.clone(), None).await;
            info!(
                endpoint = "/geocode",
                query = params.q.trim(),
                source = "provider",
                provider = "nominatim",
                status = StatusCode::OK.as_u16(),
                "request served"
            );
            json_body(StatusCode::OK, body)
        }
        Err(()) => {
            warn!(
                endpoint = "/geocode",
                query = params.q.trim(),
                source = "error",
                status = StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                "request failed"
            );
            error_json(StatusCode::SERVICE_UNAVAILABLE, "service unavailable")
        }
    }
}

pub async fn weather(State(state): State<AppState>, Path(loc): Path<String>) -> Response {
    info!(endpoint = "/weather/{loc}", loc, "request received");

    let Some(location) = providers::location_for(&loc) else {
        warn!(
            endpoint = "/weather/{loc}",
            loc,
            source = "error",
            status = StatusCode::BAD_REQUEST.as_u16(),
            "unknown location requested"
        );
        return error_json(StatusCode::BAD_REQUEST, "unknown location");
    };

    let cache_key = format!("weather:{}", location.key);

    if let Some(body) = cache::get_fresh(&state.cache, &cache_key).await {
        info!(
            endpoint = "/weather/{loc}",
            loc = location.key,
            source = "cache",
            status = StatusCode::OK.as_u16(),
            "request served"
        );
        return json_body(StatusCode::OK, body);
    }

    match providers::fetch_open_meteo(&state.client, location, state.open_meteo_timeout).await {
        Ok(weather) => {
            let body = weather_json(&weather);
            cache::set(
                &state.cache,
                cache_key,
                body.clone(),
                Some(state.weather_cache_ttl),
            )
            .await;
            info!(
                endpoint = "/weather/{loc}",
                loc = location.key,
                source = "provider",
                provider = "open-meteo",
                status = StatusCode::OK.as_u16(),
                "request served"
            );
            json_body(StatusCode::OK, body)
        }
        Err(()) => {
            warn!(
                location = location.key,
                "primary weather provider failed, using fallback"
            );

            match providers::fetch_openweathermap(
                &state.client,
                location,
                &state.openweathermap_api_key,
                state.openweathermap_timeout,
            )
            .await
            {
                Ok(weather) => {
                    let body = weather_json(&weather);
                    cache::set(
                        &state.cache,
                        cache_key,
                        body.clone(),
                        Some(state.weather_cache_ttl),
                    )
                    .await;
                    info!(
                        endpoint = "/weather/{loc}",
                        loc = location.key,
                        source = "provider",
                        provider = "openweathermap",
                        status = StatusCode::OK.as_u16(),
                        "request served"
                    );
                    json_body(StatusCode::OK, body)
                }
                Err(()) => {
                    if let Some(body) =
                        cache::get_stale(&state.cache, &cache_key, WEATHER_STALE_WINDOW).await
                    {
                        info!(
                            endpoint = "/weather/{loc}",
                            loc = location.key,
                            source = "stale-cache",
                            status = StatusCode::OK.as_u16(),
                            "request served"
                        );
                        return json_body(StatusCode::OK, body);
                    }

                    warn!(
                        endpoint = "/weather/{loc}",
                        loc = location.key,
                        source = "error",
                        status = StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                        "request failed after provider attempts"
                    );
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

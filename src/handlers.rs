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
    providers::{self, WeatherResponse, WeatherTarget},
};

const WEATHER_STALE_WINDOW: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Deserialize)]
pub struct GeocodeQuery {
    q: String,
}

#[derive(Deserialize)]
pub struct WeatherQuery {
    latitude: Option<String>,
    longitude: Option<String>,
}

#[derive(Debug, PartialEq)]
struct ResolvedWeatherRequest {
    cache_key: String,
    target_label: &'static str,
    target: WeatherTarget,
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

pub async fn weather_query(
    State(state): State<AppState>,
    Query(params): Query<WeatherQuery>,
) -> Response {
    let resolved = resolve_weather_query(&params);

    info!(
        endpoint = "/weather",
        latitude = params.latitude.as_deref().unwrap_or(""),
        longitude = params.longitude.as_deref().unwrap_or(""),
        target = resolved.target_label,
        "request received"
    );

    serve_weather(
        &state,
        "/weather",
        resolved.target_label,
        &resolved.cache_key,
        resolved.target,
    )
    .await
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

    let cache_key = format!("weather:{loc}");
    serve_weather(&state, "/weather/{loc}", &loc, &cache_key, location).await
}

fn resolve_weather_query(params: &WeatherQuery) -> ResolvedWeatherRequest {
    match (
        parse_coordinate(params.latitude.as_deref(), -90.0, 90.0),
        parse_coordinate(params.longitude.as_deref(), -180.0, 180.0),
    ) {
        (Some(latitude), Some(longitude)) => ResolvedWeatherRequest {
            cache_key: format!(
                "weather:coords:{}:{}",
                params.latitude.as_deref().unwrap_or(""),
                params.longitude.as_deref().unwrap_or("")
            ),
            target_label: "coords",
            target: WeatherTarget::coords(latitude, longitude),
        },
        _ => ResolvedWeatherRequest {
            cache_key: format!("weather:{}", providers::DEFAULT_LOCATION_KEY),
            target_label: providers::DEFAULT_LOCATION_KEY,
            target: providers::default_location(),
        },
    }
}

fn parse_coordinate(value: Option<&str>, min: f64, max: f64) -> Option<f64> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed = trimmed.parse::<f64>().ok()?;
    if (min..=max).contains(&parsed) {
        Some(parsed)
    } else {
        None
    }
}

async fn serve_weather(
    state: &AppState,
    endpoint: &'static str,
    target_label: &str,
    cache_key: &str,
    target: WeatherTarget,
) -> Response {
    if let Some(body) = cache::get_fresh(&state.cache, cache_key).await {
        info!(
            endpoint,
            target = target_label,
            latitude = target.lat,
            longitude = target.lon,
            source = "cache",
            status = StatusCode::OK.as_u16(),
            "request served"
        );
        return json_body(StatusCode::OK, body);
    }

    match providers::fetch_open_meteo(&state.client, target, state.open_meteo_timeout).await {
        Ok(weather) => {
            let body = weather_json(&weather);
            cache::set(
                &state.cache,
                cache_key.to_string(),
                body.clone(),
                Some(state.weather_cache_ttl),
            )
            .await;
            info!(
                endpoint,
                target = target_label,
                latitude = target.lat,
                longitude = target.lon,
                source = "provider",
                provider = "open-meteo",
                status = StatusCode::OK.as_u16(),
                "request served"
            );
            json_body(StatusCode::OK, body)
        }
        Err(()) => {
            warn!(
                endpoint,
                target = target_label,
                latitude = target.lat,
                longitude = target.lon,
                "primary weather provider failed, using fallback"
            );

            match providers::fetch_openweathermap(
                &state.client,
                target,
                &state.openweathermap_api_key,
                state.openweathermap_timeout,
            )
            .await
            {
                Ok(weather) => {
                    let body = weather_json(&weather);
                    cache::set(
                        &state.cache,
                        cache_key.to_string(),
                        body.clone(),
                        Some(state.weather_cache_ttl),
                    )
                    .await;
                    info!(
                        endpoint,
                        target = target_label,
                        latitude = target.lat,
                        longitude = target.lon,
                        source = "provider",
                        provider = "openweathermap",
                        status = StatusCode::OK.as_u16(),
                        "request served"
                    );
                    json_body(StatusCode::OK, body)
                }
                Err(()) => {
                    if let Some(body) =
                        cache::get_stale(&state.cache, cache_key, WEATHER_STALE_WINDOW).await
                    {
                        info!(
                            endpoint,
                            target = target_label,
                            latitude = target.lat,
                            longitude = target.lon,
                            source = "stale-cache",
                            status = StatusCode::OK.as_u16(),
                            "request served"
                        );
                        return json_body(StatusCode::OK, body);
                    }

                    warn!(
                        endpoint,
                        target = target_label,
                        latitude = target.lat,
                        longitude = target.lon,
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

#[cfg(test)]
mod tests {
    use super::{WeatherQuery, resolve_weather_query};
    use crate::providers;

    #[test]
    fn resolves_missing_coordinates_to_default_location() {
        let resolved = resolve_weather_query(&WeatherQuery {
            latitude: None,
            longitude: None,
        });

        assert_eq!(resolved.cache_key, "weather:hoankiem");
        assert_eq!(resolved.target_label, providers::DEFAULT_LOCATION_KEY);
        assert_eq!(resolved.target, providers::default_location());
    }

    #[test]
    fn resolves_invalid_coordinates_to_default_location() {
        let resolved = resolve_weather_query(&WeatherQuery {
            latitude: Some("abc".to_string()),
            longitude: Some("181".to_string()),
        });

        assert_eq!(resolved.cache_key, "weather:hoankiem");
        assert_eq!(resolved.target_label, providers::DEFAULT_LOCATION_KEY);
        assert_eq!(resolved.target, providers::default_location());
    }

    #[test]
    fn keeps_raw_coordinate_strings_in_cache_key() {
        let resolved = resolve_weather_query(&WeatherQuery {
            latitude: Some("21.2083286".to_string()),
            longitude: Some("105.433452".to_string()),
        });

        assert_eq!(resolved.cache_key, "weather:coords:21.2083286:105.433452");
        assert_eq!(resolved.target_label, "coords");
        assert_eq!(
            resolved.target,
            providers::WeatherTarget::coords(21.2083286, 105.433452)
        );
    }
}

use std::time::Duration;

use reqwest::{
    Client,
    header::{self, HeaderValue},
};
use serde::{Deserialize, Serialize};

const GEOCODE_USER_AGENT: &str = "Hanoi Research Project (john33fiao@tt-inno.com)";
const OPEN_METEO_TIMEOUT_MS: u64 = 700;
const OPEN_WEATHER_TIMEOUT_MS: u64 = 1000;

#[derive(Clone, Copy)]
pub struct Location {
    pub key: &'static str,
    pub lat: &'static str,
    pub lon: &'static str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherCurrent {
    pub temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub wind_speed_10m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherResponse {
    pub current: WeatherCurrent,
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: WeatherCurrent,
}

#[derive(Deserialize)]
struct OpenWeatherMapResponse {
    main: OpenWeatherMapMain,
    wind: OpenWeatherMapWind,
}

#[derive(Deserialize)]
struct OpenWeatherMapMain {
    temp: f64,
    humidity: f64,
}

#[derive(Deserialize)]
struct OpenWeatherMapWind {
    speed: f64,
}

pub fn location_for(loc: &str) -> Option<Location> {
    match loc {
        "hoankiem" => Some(Location {
            key: "hoankiem",
            lat: "21.0287772",
            lon: "105.8510772",
        }),
        "minhchau" => Some(Location {
            key: "minhchau",
            lat: "21.2083286",
            lon: "105.433452",
        }),
        _ => None,
    }
}

pub async fn fetch_geocode(client: &Client, query: &str) -> Result<String, ()> {
    let response = match client
        .get("https://nominatim.openstreetmap.org/search")
        .header(
            header::USER_AGENT,
            HeaderValue::from_static(GEOCODE_USER_AGENT),
        )
        .query(&[
            ("format", "json"),
            ("countrycodes", "vn"),
            ("limit", "1"),
            (
                "viewbox",
                "105.73150648869479,21.12880239822778,105.99722688618795,20.92166633039378",
            ),
            ("bounded", "1"),
            ("accept-language", "en"),
            ("q", query),
        ])
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(error = %error, query, "Nominatim request failed");
            return Err(());
        }
    };

    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(error = %error, query, "Nominatim request failed");
            return Err(());
        }
    };

    match response.text().await {
        Ok(body) => Ok(body),
        Err(error) => {
            tracing::error!(error = %error, query, "Nominatim request failed");
            Err(())
        }
    }
}

pub async fn fetch_open_meteo(client: &Client, location: Location) -> Result<WeatherResponse, ()> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,wind_speed_10m",
        location.lat, location.lon
    );

    let response = match client
        .get(url)
        .timeout(Duration::from_millis(OPEN_METEO_TIMEOUT_MS))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(error = %error, location = location.key, "Open-Meteo request failed");
            return Err(());
        }
    };

    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(error = %error, location = location.key, "Open-Meteo request failed");
            return Err(());
        }
    };

    match response.json::<OpenMeteoResponse>().await {
        Ok(body) => Ok(WeatherResponse {
            current: body.current,
        }),
        Err(error) => {
            tracing::error!(error = %error, location = location.key, "Open-Meteo request failed");
            Err(())
        }
    }
}

pub async fn fetch_openweathermap(
    client: &Client,
    location: Location,
    api_key: &str,
) -> Result<WeatherResponse, ()> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}",
        location.lat, location.lon, api_key
    );

    let response = match client
        .get(url)
        .timeout(Duration::from_millis(OPEN_WEATHER_TIMEOUT_MS))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(
                error = %error,
                location = location.key,
                "OpenWeatherMap request failed"
            );
            return Err(());
        }
    };

    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(
                error = %error,
                location = location.key,
                "OpenWeatherMap request failed"
            );
            return Err(());
        }
    };

    match response.json::<OpenWeatherMapResponse>().await {
        Ok(body) => Ok(WeatherResponse {
            current: WeatherCurrent {
                temperature_2m: body.main.temp - 273.15,
                relative_humidity_2m: body.main.humidity,
                wind_speed_10m: body.wind.speed,
            },
        }),
        Err(error) => {
            tracing::error!(
                error = %error,
                location = location.key,
                "OpenWeatherMap request failed"
            );
            Err(())
        }
    }
}

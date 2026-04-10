use std::{collections::BTreeMap, time::Duration};

use reqwest::{
    Client,
    header::{self, HeaderValue},
};
use serde::{Deserialize, Serialize};

const GEOCODE_USER_AGENT: &str = "Hanoi Research Project (john33fiao@tt-inno.com)";
const OPEN_METEO_CURRENT_FIELDS: &str = "temperature_2m,relative_humidity_2m,apparent_temperature,is_day,precipitation,rain,showers,snowfall,weather_code,cloud_cover,pressure_msl,surface_pressure,wind_speed_10m,wind_direction_10m,wind_gusts_10m";
const OPEN_METEO_DAILY_FIELDS: &str = "temperature_2m_max,temperature_2m_min";

pub const DEFAULT_LOCATION_KEY: &str = "hoankiem";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeatherTarget {
    pub lat: f64,
    pub lon: f64,
    pub timezone: Option<&'static str>,
    pub elevation: Option<f64>,
}

impl WeatherTarget {
    const fn named(lat: f64, lon: f64, timezone: &'static str, elevation: f64) -> Self {
        Self {
            lat,
            lon,
            timezone: Some(timezone),
            elevation: Some(elevation),
        }
    }

    pub const fn coords(lat: f64, lon: f64) -> Self {
        Self {
            lat,
            lon,
            timezone: None,
            elevation: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherCurrentUnits {
    pub time: String,
    pub interval: String,
    pub temperature_2m: String,
    pub relative_humidity_2m: String,
    pub apparent_temperature: String,
    pub is_day: String,
    pub precipitation: String,
    pub rain: String,
    pub showers: String,
    pub snowfall: String,
    pub weather_code: String,
    pub cloud_cover: String,
    pub pressure_msl: String,
    pub surface_pressure: String,
    pub wind_speed_10m: String,
    pub wind_direction_10m: String,
    pub wind_gusts_10m: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherCurrent {
    pub time: String,
    pub interval: i64,
    pub temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub apparent_temperature: f64,
    pub is_day: i32,
    pub precipitation: f64,
    pub rain: f64,
    pub showers: f64,
    pub snowfall: f64,
    pub weather_code: i32,
    pub cloud_cover: f64,
    pub pressure_msl: f64,
    pub surface_pressure: f64,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: f64,
    pub wind_gusts_10m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WeatherDailyUnits {
    pub time: String,
    pub temperature_2m_max: String,
    pub temperature_2m_min: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WeatherDaily {
    pub time: Vec<String>,
    pub temperature_2m_max: Vec<f64>,
    pub temperature_2m_min: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub generationtime_ms: f64,
    pub utc_offset_seconds: i64,
    pub timezone: String,
    pub timezone_abbreviation: String,
    pub elevation: f64,
    pub current_units: WeatherCurrentUnits,
    pub current: WeatherCurrent,
    pub daily_units: WeatherDailyUnits,
    pub daily: WeatherDaily,
}

#[derive(Deserialize)]
struct OpenWeatherMapResponse {
    coord: OpenWeatherMapCoord,
    #[serde(default)]
    weather: Vec<OpenWeatherMapWeather>,
    main: OpenWeatherMapMain,
    wind: OpenWeatherMapWind,
    clouds: Option<OpenWeatherMapClouds>,
    rain: Option<OpenWeatherMapPrecipitation>,
    snow: Option<OpenWeatherMapPrecipitation>,
    dt: i64,
    timezone: i64,
    sys: OpenWeatherMapSys,
}

#[derive(Deserialize)]
struct OpenWeatherMapCoord {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct OpenWeatherMapWeather {
    id: i32,
}

#[derive(Deserialize)]
struct OpenWeatherMapMain {
    temp: f64,
    feels_like: f64,
    humidity: f64,
    pressure: f64,
    sea_level: Option<f64>,
    grnd_level: Option<f64>,
}

#[derive(Deserialize)]
struct OpenWeatherMapWind {
    speed: f64,
    deg: Option<f64>,
    gust: Option<f64>,
}

#[derive(Deserialize)]
struct OpenWeatherMapClouds {
    all: f64,
}

#[derive(Deserialize)]
struct OpenWeatherMapPrecipitation {
    #[serde(rename = "1h")]
    one_hour: Option<f64>,
}

#[derive(Deserialize)]
struct OpenWeatherMapSys {
    sunrise: i64,
    sunset: i64,
}

#[derive(Deserialize)]
struct OpenWeatherMapForecastResponse {
    #[serde(default)]
    list: Vec<OpenWeatherMapForecastItem>,
    city: OpenWeatherMapForecastCity,
}

#[derive(Deserialize)]
struct OpenWeatherMapForecastItem {
    dt: i64,
    main: OpenWeatherMapForecastMain,
}

#[derive(Deserialize)]
struct OpenWeatherMapForecastMain {
    temp_min: f64,
    temp_max: f64,
}

#[derive(Deserialize)]
struct OpenWeatherMapForecastCity {
    timezone: i64,
}

pub fn default_location() -> WeatherTarget {
    location_for(DEFAULT_LOCATION_KEY).expect("default location must exist")
}

pub fn location_for(loc: &str) -> Option<WeatherTarget> {
    match loc {
        "hoankiem" => Some(WeatherTarget::named(
            21.0287772,
            105.8510772,
            "Asia/Bangkok",
            18.0,
        )),
        "minhchau" => Some(WeatherTarget::named(
            21.2083286,
            105.433452,
            "Asia/Bangkok",
            12.0,
        )),
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

pub async fn fetch_open_meteo(
    client: &Client,
    target: WeatherTarget,
    timeout: Duration,
) -> Result<WeatherResponse, ()> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current={}&daily={}&timezone=auto&cell_selection=nearest",
        target.lat, target.lon, OPEN_METEO_CURRENT_FIELDS, OPEN_METEO_DAILY_FIELDS
    );

    let response = match client.get(url).timeout(timeout).send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(
                error = %error,
                latitude = target.lat,
                longitude = target.lon,
                "Open-Meteo request failed"
            );
            return Err(());
        }
    };

    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(
                error = %error,
                latitude = target.lat,
                longitude = target.lon,
                "Open-Meteo request failed"
            );
            return Err(());
        }
    };

    match response.json::<WeatherResponse>().await {
        Ok(body) => Ok(body),
        Err(error) => {
            tracing::error!(
                error = %error,
                latitude = target.lat,
                longitude = target.lon,
                "Open-Meteo request failed"
            );
            Err(())
        }
    }
}

pub async fn fetch_openweathermap(
    client: &Client,
    target: WeatherTarget,
    api_key: &str,
    timeout: Duration,
) -> Result<WeatherResponse, ()> {
    let current = fetch_openweathermap_current(client, target, api_key, timeout).await?;
    let forecast = fetch_openweathermap_forecast(client, target, api_key, timeout).await?;
    let daily = normalize_openweathermap_daily(forecast);
    Ok(normalize_openweathermap_response(target, current, daily))
}

async fn fetch_openweathermap_current(
    client: &Client,
    target: WeatherTarget,
    api_key: &str,
    timeout: Duration,
) -> Result<OpenWeatherMapResponse, ()> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}",
        target.lat, target.lon, api_key
    );

    let response = match client.get(url).timeout(timeout).send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(
                error = %error,
                latitude = target.lat,
                longitude = target.lon,
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
                latitude = target.lat,
                longitude = target.lon,
                "OpenWeatherMap request failed"
            );
            return Err(());
        }
    };

    match response.json::<OpenWeatherMapResponse>().await {
        Ok(body) => Ok(body),
        Err(error) => {
            tracing::error!(
                error = %error,
                latitude = target.lat,
                longitude = target.lon,
                "OpenWeatherMap request failed"
            );
            Err(())
        }
    }
}

async fn fetch_openweathermap_forecast(
    client: &Client,
    target: WeatherTarget,
    api_key: &str,
    timeout: Duration,
) -> Result<OpenWeatherMapForecastResponse, ()> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&appid={}",
        target.lat, target.lon, api_key
    );

    let response = match client.get(url).timeout(timeout).send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(
                error = %error,
                latitude = target.lat,
                longitude = target.lon,
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
                latitude = target.lat,
                longitude = target.lon,
                "OpenWeatherMap request failed"
            );
            return Err(());
        }
    };

    match response.json::<OpenWeatherMapForecastResponse>().await {
        Ok(body) => Ok(body),
        Err(error) => {
            tracing::error!(
                error = %error,
                latitude = target.lat,
                longitude = target.lon,
                "OpenWeatherMap request failed"
            );
            Err(())
        }
    }
}

fn normalize_openweathermap_response(
    target: WeatherTarget,
    body: OpenWeatherMapResponse,
    daily: WeatherDaily,
) -> WeatherResponse {
    let weather_id = body
        .weather
        .first()
        .map(|weather| weather.id)
        .unwrap_or(804);
    let rain_amount = body.rain.and_then(|rain| rain.one_hour).unwrap_or(0.0);
    let snowfall_mm = body.snow.and_then(|snow| snow.one_hour).unwrap_or(0.0);
    let (rain, showers) = if matches!(weather_id, 520..=531) {
        (0.0, rain_amount)
    } else {
        (rain_amount, 0.0)
    };
    let timezone_abbreviation = format_gmt_offset(body.timezone);
    let timezone = match target.timezone {
        Some(timezone) => timezone.to_string(),
        None => timezone_abbreviation.clone(),
    };

    WeatherResponse {
        latitude: body.coord.lat,
        longitude: body.coord.lon,
        generationtime_ms: 0.0,
        utc_offset_seconds: body.timezone,
        timezone,
        timezone_abbreviation,
        elevation: target.elevation.unwrap_or(0.0),
        current_units: weather_current_units(),
        current: WeatherCurrent {
            time: format_local_time(body.dt, body.timezone),
            interval: 3600,
            temperature_2m: kelvin_to_celsius(body.main.temp),
            relative_humidity_2m: body.main.humidity,
            apparent_temperature: kelvin_to_celsius(body.main.feels_like),
            is_day: (body.dt >= body.sys.sunrise && body.dt < body.sys.sunset) as i32,
            precipitation: rain_amount + snowfall_mm,
            rain,
            showers,
            snowfall: snowfall_mm / 10.0,
            weather_code: map_openweathermap_weather_code(weather_id),
            cloud_cover: body.clouds.map(|clouds| clouds.all).unwrap_or(0.0),
            pressure_msl: body.main.sea_level.unwrap_or(body.main.pressure),
            surface_pressure: body.main.grnd_level.unwrap_or(body.main.pressure),
            wind_speed_10m: ms_to_kmh(body.wind.speed),
            wind_direction_10m: body.wind.deg.unwrap_or(0.0),
            wind_gusts_10m: ms_to_kmh(body.wind.gust.unwrap_or(body.wind.speed)),
        },
        daily_units: weather_daily_units(),
        daily,
    }
}

fn normalize_openweathermap_daily(body: OpenWeatherMapForecastResponse) -> WeatherDaily {
    let mut grouped = BTreeMap::<String, (f64, f64)>::new();

    for item in body.list {
        let date = format_local_date(item.dt, body.city.timezone);
        grouped
            .entry(date)
            .and_modify(|(max_temp, min_temp)| {
                *max_temp = max_temp.max(item.main.temp_max);
                *min_temp = min_temp.min(item.main.temp_min);
            })
            .or_insert((item.main.temp_max, item.main.temp_min));
    }

    let mut time = Vec::with_capacity(grouped.len());
    let mut temperature_2m_max = Vec::with_capacity(grouped.len());
    let mut temperature_2m_min = Vec::with_capacity(grouped.len());

    for (date, (max_temp, min_temp)) in grouped {
        time.push(date);
        temperature_2m_max.push(kelvin_to_celsius(max_temp));
        temperature_2m_min.push(kelvin_to_celsius(min_temp));
    }

    WeatherDaily {
        time,
        temperature_2m_max,
        temperature_2m_min,
    }
}

fn weather_current_units() -> WeatherCurrentUnits {
    WeatherCurrentUnits {
        time: "iso8601".to_string(),
        interval: "seconds".to_string(),
        temperature_2m: "°C".to_string(),
        relative_humidity_2m: "%".to_string(),
        apparent_temperature: "°C".to_string(),
        is_day: String::new(),
        precipitation: "mm".to_string(),
        rain: "mm".to_string(),
        showers: "mm".to_string(),
        snowfall: "cm".to_string(),
        weather_code: "wmo code".to_string(),
        cloud_cover: "%".to_string(),
        pressure_msl: "hPa".to_string(),
        surface_pressure: "hPa".to_string(),
        wind_speed_10m: "km/h".to_string(),
        wind_direction_10m: "°".to_string(),
        wind_gusts_10m: "km/h".to_string(),
    }
}

fn weather_daily_units() -> WeatherDailyUnits {
    WeatherDailyUnits {
        time: "iso8601".to_string(),
        temperature_2m_max: "°C".to_string(),
        temperature_2m_min: "°C".to_string(),
    }
}

fn kelvin_to_celsius(value: f64) -> f64 {
    value - 273.15
}

fn ms_to_kmh(value: f64) -> f64 {
    value * 3.6
}

fn format_gmt_offset(offset_seconds: i64) -> String {
    let total_minutes = offset_seconds / 60;
    let sign = if total_minutes >= 0 { '+' } else { '-' };
    let abs_minutes = total_minutes.abs();
    let hours = abs_minutes / 60;
    let minutes = abs_minutes % 60;

    if minutes == 0 {
        format!("GMT{sign}{hours}")
    } else {
        format!("GMT{sign}{hours}:{minutes:02}")
    }
}

fn format_local_time(unix_seconds: i64, utc_offset_seconds: i64) -> String {
    let local_seconds = unix_seconds + utc_offset_seconds;
    let seconds_of_day = local_seconds.rem_euclid(86_400);
    let days = (local_seconds - seconds_of_day) / 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}")
}

fn format_local_date(unix_seconds: i64, utc_offset_seconds: i64) -> String {
    let local_seconds = unix_seconds + utc_offset_seconds;
    let seconds_of_day = local_seconds.rem_euclid(86_400);
    let days = (local_seconds - seconds_of_day) / 86_400;
    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

fn map_openweathermap_weather_code(weather_id: i32) -> i32 {
    match weather_id {
        200..=232 => 95,
        300..=301 => 51,
        302..=321 => 53,
        500 => 61,
        501 => 63,
        502..=504 => 65,
        511 => 66,
        520 => 80,
        521 => 81,
        522..=531 => 82,
        600 => 71,
        601 => 73,
        602 => 75,
        611..=616 => 68,
        620 => 85,
        621..=622 => 86,
        701..=762 => 45,
        771 => 95,
        781 => 99,
        800 => 0,
        801 => 1,
        802 => 2,
        803..=804 => 3,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        OpenWeatherMapClouds, OpenWeatherMapCoord, OpenWeatherMapForecastCity,
        OpenWeatherMapForecastItem, OpenWeatherMapForecastMain, OpenWeatherMapForecastResponse,
        OpenWeatherMapMain, OpenWeatherMapPrecipitation, OpenWeatherMapResponse, OpenWeatherMapSys,
        OpenWeatherMapWeather, OpenWeatherMapWind, WeatherDaily, WeatherDailyUnits,
        WeatherResponse, WeatherTarget, format_gmt_offset, format_local_time,
        map_openweathermap_weather_code, normalize_openweathermap_daily,
        normalize_openweathermap_response, weather_daily_units,
    };

    #[test]
    fn deserializes_open_meteo_response_with_daily() {
        let body = json!({
            "latitude": 37.55,
            "longitude": 127.0,
            "generationtime_ms": 0.27382373809814453,
            "utc_offset_seconds": 32400,
            "timezone": "Asia/Seoul",
            "timezone_abbreviation": "GMT+9",
            "elevation": 34.0,
            "current_units": {
                "time": "iso8601",
                "interval": "seconds",
                "temperature_2m": "°C",
                "relative_humidity_2m": "%",
                "apparent_temperature": "°C",
                "is_day": "",
                "precipitation": "mm",
                "rain": "mm",
                "showers": "mm",
                "snowfall": "cm",
                "weather_code": "wmo code",
                "cloud_cover": "%",
                "pressure_msl": "hPa",
                "surface_pressure": "hPa",
                "wind_speed_10m": "km/h",
                "wind_direction_10m": "°",
                "wind_gusts_10m": "km/h"
            },
            "current": {
                "time": "2026-04-10T12:00",
                "interval": 900,
                "temperature_2m": 9.7,
                "relative_humidity_2m": 90,
                "apparent_temperature": 8.0,
                "is_day": 1,
                "precipitation": 0.0,
                "rain": 0.0,
                "showers": 0.0,
                "snowfall": 0.0,
                "weather_code": 3,
                "cloud_cover": 94,
                "pressure_msl": 1001.6,
                "surface_pressure": 997.5,
                "wind_speed_10m": 8.2,
                "wind_direction_10m": 247,
                "wind_gusts_10m": 24.1
            },
            "daily_units": {
                "time": "iso8601",
                "temperature_2m_max": "°C",
                "temperature_2m_min": "°C"
            },
            "daily": {
                "time": ["2026-04-10", "2026-04-11", "2026-04-12", "2026-04-13", "2026-04-14", "2026-04-15", "2026-04-16"],
                "temperature_2m_max": [11.0, 16.0, 20.3, 23.2, 25.7, 24.3, 23.9],
                "temperature_2m_min": [8.3, 6.5, 4.3, 6.5, 9.8, 6.1, 8.5]
            }
        });

        let response: WeatherResponse =
            serde_json::from_value(body).expect("sample open-meteo response should deserialize");

        assert_eq!(response.timezone, "Asia/Seoul");
        assert_eq!(response.daily_units, weather_daily_units());
        assert_eq!(response.daily.time.len(), 7);
        assert_eq!(response.daily.time[0], "2026-04-10");
        assert_eq!(response.daily.temperature_2m_max[2], 20.3);
        assert_eq!(response.daily.temperature_2m_min[6], 8.5);
    }

    #[test]
    fn formats_gmt_offsets_like_open_meteo() {
        assert_eq!(format_gmt_offset(25_200), "GMT+7");
        assert_eq!(format_gmt_offset(-19_800), "GMT-5:30");
    }

    #[test]
    fn formats_local_time_from_unix_seconds() {
        assert_eq!(format_local_time(0, 25_200), "1970-01-01T07:00");
        assert_eq!(format_local_time(86_460, 0), "1970-01-02T00:01");
    }

    #[test]
    fn maps_openweathermap_codes_to_wmo_codes() {
        assert_eq!(map_openweathermap_weather_code(800), 0);
        assert_eq!(map_openweathermap_weather_code(803), 3);
        assert_eq!(map_openweathermap_weather_code(500), 61);
        assert_eq!(map_openweathermap_weather_code(521), 81);
        assert_eq!(map_openweathermap_weather_code(601), 73);
    }

    #[test]
    fn groups_openweathermap_forecast_by_local_date() {
        let daily = normalize_openweathermap_daily(OpenWeatherMapForecastResponse {
            list: vec![
                OpenWeatherMapForecastItem {
                    dt: 0,
                    main: OpenWeatherMapForecastMain {
                        temp_min: 270.15,
                        temp_max: 280.15,
                    },
                },
                OpenWeatherMapForecastItem {
                    dt: 7_200,
                    main: OpenWeatherMapForecastMain {
                        temp_min: 268.15,
                        temp_max: 282.15,
                    },
                },
                OpenWeatherMapForecastItem {
                    dt: 82_800,
                    main: OpenWeatherMapForecastMain {
                        temp_min: 275.15,
                        temp_max: 285.15,
                    },
                },
            ],
            city: OpenWeatherMapForecastCity { timezone: 3_600 },
        });

        assert_eq!(
            daily,
            WeatherDaily {
                time: vec!["1970-01-01".to_string(), "1970-01-02".to_string()],
                temperature_2m_max: vec![9.0, 12.0],
                temperature_2m_min: vec![-5.0, 2.0],
            }
        );
    }

    #[test]
    fn normalizes_openweathermap_gps_fallback_metadata_and_daily() {
        let response = normalize_openweathermap_response(
            WeatherTarget::coords(21.0288, 105.8511),
            OpenWeatherMapResponse {
                coord: OpenWeatherMapCoord {
                    lat: 21.0288,
                    lon: 105.8511,
                },
                weather: vec![OpenWeatherMapWeather { id: 521 }],
                main: OpenWeatherMapMain {
                    temp: 301.12,
                    feels_like: 302.89,
                    humidity: 63.0,
                    pressure: 1007.0,
                    sea_level: Some(1007.0),
                    grnd_level: Some(1006.0),
                },
                wind: OpenWeatherMapWind {
                    speed: 2.3,
                    deg: Some(165.0),
                    gust: Some(3.11),
                },
                clouds: Some(OpenWeatherMapClouds { all: 12.0 }),
                rain: Some(OpenWeatherMapPrecipitation {
                    one_hour: Some(1.4),
                }),
                snow: Some(OpenWeatherMapPrecipitation {
                    one_hour: Some(0.5),
                }),
                dt: 1_775_789_152,
                timezone: 25_200,
                sys: OpenWeatherMapSys {
                    sunrise: 1_775_774_527,
                    sunset: 1_775_819_610,
                },
            },
            WeatherDaily {
                time: vec!["1970-01-01".to_string(), "1970-01-02".to_string()],
                temperature_2m_max: vec![9.0, 12.0],
                temperature_2m_min: vec![-5.0, 2.0],
            },
        );

        assert_eq!(response.latitude, 21.0288);
        assert_eq!(response.longitude, 105.8511);
        assert_eq!(response.timezone, "GMT+7");
        assert_eq!(response.timezone_abbreviation, "GMT+7");
        assert_eq!(response.elevation, 0.0);
        assert_eq!(
            response.daily_units,
            WeatherDailyUnits {
                time: "iso8601".to_string(),
                temperature_2m_max: "°C".to_string(),
                temperature_2m_min: "°C".to_string(),
            }
        );
        assert_eq!(
            response.daily,
            WeatherDaily {
                time: vec!["1970-01-01".to_string(), "1970-01-02".to_string()],
                temperature_2m_max: vec![9.0, 12.0],
                temperature_2m_min: vec![-5.0, 2.0],
            }
        );
        assert_eq!(response.current.weather_code, 81);
        assert_eq!(response.current.precipitation, 1.9);
        assert_eq!(response.current.rain, 0.0);
        assert_eq!(response.current.showers, 1.4);
        assert_eq!(response.current.snowfall, 0.05);
        assert!((response.current.wind_speed_10m - 8.28).abs() < 0.001);
        assert!((response.current.wind_gusts_10m - 11.196).abs() < 0.001);
        assert_eq!(response.current.wind_direction_10m, 165.0);
    }
}

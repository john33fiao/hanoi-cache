# hanoi-cache

하노이 권역의 지오코딩과 날씨 조회를 위한 작은 HTTP 서버입니다.

- `/geocode`: 하노이 범위로 제한된 지오코딩 검색
- `/weather/{loc}`: 미리 정의된 위치의 현재 날씨 조회
- 인메모리 캐시 사용
- 날씨 조회 시 1차 공급자 실패하면 2차 공급자로 자동 폴백

설계와 내부 구조는 [docs/architecture.md](docs/architecture.md)에 정리했습니다.
리눅스 서버 `podman` 배포 절차는 [docs/podman-deploy.md](docs/podman-deploy.md)를 확인하면 됩니다.

## 빠른 시작

### 준비물

- Rust 툴체인
- OpenWeatherMap API 키

이 서버는 `/weather`를 호출하지 않더라도 시작 시 `OPENWEATHERMAP_API_KEY`를 반드시 요구합니다.

### 실행

PowerShell 기준:

```powershell
Copy-Item .env.example .env
# .env 파일에 API 키를 채웁니다.
cargo run --release
```

기본 포트는 `3000`이고, 서버는 `0.0.0.0:<PORT>`에 바인딩됩니다.
애플리케이션은 시작 시 현재 작업 디렉터리의 `.env`를 자동으로 읽고, 이미 셸에 설정된 환경 변수가 있으면 그 값을 우선 사용합니다.

셸 환경 변수로 직접 지정해도 됩니다.

```powershell
$env:OPENWEATHERMAP_API_KEY="your-api-key"
$env:PORT="3000"
$env:RUST_LOG="info"
cargo run --release
```

실행 후 기본 주소:

```text
http://127.0.0.1:3000
```

## 사용법

### 1. 지오코딩 조회

`GET /geocode?q=<query>`

예시:

```bash
curl "http://127.0.0.1:3000/geocode?q=Hoan%20Kiem%20Lake"
```

대표 응답 예시:

```json
[
  {
    "lat": "21.0281",
    "lon": "105.8522",
    "display_name": "Hoan Kiem Lake, Hoan Kiem, Hanoi, Vietnam"
  }
]
```

동작 방식:

- 입력값은 앞뒤 공백만 제거합니다.
- 결과는 하노이 주변으로 제한된 Nominatim 검색 결과입니다.
- 결과 수는 최대 1개입니다.
- 성공 응답 본문은 Nominatim JSON을 거의 그대로 반환합니다.
- 캐시는 인메모리에 저장되며 만료 시간이 없습니다.

### 2. 날씨 조회

`GET /weather/{loc}`

현재 지원하는 `loc` 값:

| loc | 설명 |
| --- | --- |
| `hoankiem` | 호안끼엠 |
| `minhchau` | 민쩌우 |

예시:

```bash
curl "http://127.0.0.1:3000/weather/hoankiem"
curl "http://127.0.0.1:3000/weather/minhchau"
```

대표 응답 예시:

```json
{
  "latitude": 21.0,
  "longitude": 105.875,
  "generationtime_ms": 0.135,
  "utc_offset_seconds": 25200,
  "timezone": "Asia/Bangkok",
  "timezone_abbreviation": "GMT+7",
  "elevation": 18.0,
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
    "time": "2026-03-25T13:30",
    "interval": 900,
    "temperature_2m": 29.4,
    "relative_humidity_2m": 78.0,
    "apparent_temperature": 32.1,
    "is_day": 1,
    "precipitation": 0.0,
    "rain": 0.0,
    "showers": 0.0,
    "snowfall": 0.0,
    "weather_code": 3,
    "cloud_cover": 89.0,
    "pressure_msl": 1010.0,
    "surface_pressure": 1007.9,
    "wind_speed_10m": 19.7,
    "wind_direction_10m": 140.0,
    "wind_gusts_10m": 35.6
  }
}
```

동작 방식:

- 1차 공급자는 Open-Meteo입니다.
- Open-Meteo가 실패하면 OpenWeatherMap으로 폴백합니다.
- Open-Meteo는 `current=temperature_2m,...,wind_gusts_10m&timezone=auto&cell_selection=nearest` 형태로 호출합니다.
- 두 날씨 공급자 타임아웃은 환경 변수로 조정할 수 있고, 미설정 또는 빈 값이면 둘 다 기본값 `2000ms`를 사용합니다.
- 성공 응답은 Open-Meteo 스타일 메타데이터(`latitude`, `timezone`, `current_units`)와 상세 `current` 필드를 포함합니다.
- OpenWeatherMap 폴백도 같은 JSON 스키마로 정규화합니다.
- OpenWeatherMap 폴백에서는 온도 Kelvin -> Celsius, 풍속 m/s -> km/h, 날씨 코드는 WMO code로 변환합니다.
- 성공 응답은 fresh cache로 유지되며, `WEATHER_CACHE_TTL_SECONDS`로 초 단위 조정이 가능합니다.
- `WEATHER_CACHE_TTL_SECONDS`를 설정하지 않거나 빈 값이면 기본값 `3600`초(1시간)를 사용합니다.
- 두 공급자가 모두 실패해도, 만료 후 추가 2시간 안이면 stale cache를 반환합니다.

### 3. 오류 응답

잘못된 위치:

```json
{
  "error": "unknown location"
}
```

외부 공급자 장애:

```json
{
  "error": "service unavailable"
}
```

### 4. 요청 로그 확인

`RUST_LOG=info`로 실행하면 각 요청마다 요청 수신과 최종 응답 경로가 로그에 남습니다.

- `endpoint`: 어떤 엔드포인트 요청인지
- `query` 또는 `loc`: 어떤 입력이 들어왔는지
- `source`: `cache`, `provider`, `stale-cache`, `error`
- `provider`: 외부 API를 직접 호출한 경우 `nominatim`, `open-meteo`, `openweathermap`

즉, 같은 요청이라도 캐시에서 반환됐는지, 외부 API를 직접 호출했는지, stale cache를 사용했는지 로그에서 바로 구분할 수 있습니다.

## 환경 변수

| 변수 | 필수 | 기본값 | 설명 |
| --- | --- | --- | --- |
| `OPENWEATHERMAP_API_KEY` | 예 | 없음 | OpenWeatherMap 폴백 호출에 사용. 셸 환경 변수 또는 `.env`에서 읽음 |
| `OPEN_METEO_TIMEOUT_MS` | 아니오 | `2000` | Open-Meteo 요청 타임아웃. 단위 ms. 미설정 또는 빈 값이면 기본값 사용 |
| `OPENWEATHERMAP_TIMEOUT_MS` | 아니오 | `2000` | OpenWeatherMap 요청 타임아웃. 단위 ms. 미설정 또는 빈 값이면 기본값 사용 |
| `WEATHER_CACHE_TTL_SECONDS` | 아니오 | `3600` | 날씨 fresh cache TTL. 단위 초. 미설정 또는 빈 값이면 기본값 1시간 사용 |
| `PORT` | 아니오 | `3000` | 서버 바인딩 포트 |
| `RUST_LOG` | 아니오 | `info` | `tracing_subscriber` 로그 레벨 |

## 개발 메모

- 상태는 모두 메모리에만 저장됩니다. 재시작하면 캐시가 사라집니다.
- `/weather/{loc}`는 자유 입력이 아니라 코드에 하드코딩된 위치 슬러그만 지원합니다.
- 더 자세한 구조와 설계 의도는 [docs/architecture.md](docs/architecture.md)를 확인하면 됩니다.

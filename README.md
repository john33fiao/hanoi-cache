# hanoi-cache

하노이 권역의 지오코딩과 날씨 조회를 위한 작은 HTTP 서버입니다.

- `/geocode`: 하노이 범위로 제한된 지오코딩 검색
- `/weather/{loc}`: 미리 정의된 위치의 현재 날씨 조회
- 인메모리 캐시 사용
- 날씨 조회 시 1차 공급자 실패하면 2차 공급자로 자동 폴백

설계와 내부 구조는 [docs/architecture.md](docs/architecture.md)에 정리했습니다.

## 빠른 시작

### 준비물

- Rust 툴체인
- OpenWeatherMap API 키

이 서버는 `/weather`를 호출하지 않더라도 시작 시 `OPENWEATHERMAP_API_KEY`를 반드시 요구합니다.

### 실행

PowerShell 기준:

```powershell
$env:OPENWEATHERMAP_API_KEY="your-api-key"
$env:PORT="3000"
$env:RUST_LOG="info"
cargo run
```

기본 포트는 `3000`이고, 서버는 `0.0.0.0:<PORT>`에 바인딩됩니다.

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
```

대표 응답 예시:

```json
{
  "current": {
    "temperature_2m": 29.4,
    "relative_humidity_2m": 78.0,
    "wind_speed_10m": 2.8
  }
}
```

동작 방식:

- 1차 공급자는 Open-Meteo입니다.
- Open-Meteo가 실패하면 OpenWeatherMap으로 폴백합니다.
- OpenWeatherMap 응답의 온도는 Kelvin에서 Celsius로 변환합니다.
- 성공 응답은 1시간 동안 fresh cache로 유지됩니다.
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

## 환경 변수

| 변수 | 필수 | 기본값 | 설명 |
| --- | --- | --- | --- |
| `OPENWEATHERMAP_API_KEY` | 예 | 없음 | OpenWeatherMap 폴백 호출에 사용 |
| `PORT` | 아니오 | `3000` | 서버 바인딩 포트 |
| `RUST_LOG` | 아니오 | `info` | `tracing_subscriber` 로그 레벨 |

## 개발 메모

- 상태는 모두 메모리에만 저장됩니다. 재시작하면 캐시가 사라집니다.
- `/weather/{loc}`는 자유 입력이 아니라 코드에 하드코딩된 위치 슬러그만 지원합니다.
- 더 자세한 구조와 설계 의도는 [docs/architecture.md](docs/architecture.md)를 확인하면 됩니다.

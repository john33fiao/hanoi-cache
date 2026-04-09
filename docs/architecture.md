# Architecture

## 목적

이 서비스의 목적은 하노이 권역에서 필요한 최소한의 위치/날씨 정보를 빠르게 제공하는 것입니다.

핵심 목표:

- 단순한 HTTP 인터페이스
- 외부 API 지연을 줄이기 위한 인메모리 캐시
- 날씨 공급자 장애 시 폴백
- 운영 복잡도 최소화

## 구성 요소

| 파일 | 역할 |
| --- | --- |
| `src/main.rs` | 애플리케이션 시작점, `.env` 및 환경 변수 로딩, 라우터 구성 |
| `src/handlers.rs` | HTTP 핸들러, 캐시 정책, 오류 응답 |
| `src/providers.rs` | 외부 API 호출과 응답 변환 |
| `src/cache.rs` | 인메모리 캐시 구현 |

런타임 상태:

- `AppState.cache`: `Arc<RwLock<HashMap<...>>>` 기반 공유 캐시
- `AppState.client`: 재사용되는 `reqwest::Client`
- `AppState.openweathermap_api_key`: 폴백 공급자 호출용 API 키

환경 변수 로딩 정책:

- 시작 시 현재 작업 디렉터리의 `.env`를 먼저 읽습니다.
- 같은 키가 셸 환경 변수에 이미 있으면 셸 값이 우선합니다.

## 라우팅

현재 노출하는 엔드포인트는 두 개입니다.

- `GET /geocode?q=<query>`
- `GET /weather/{loc}`

라우터는 `axum`으로 구성되며, 모든 핸들러는 공유 `AppState`를 사용합니다.

## 요청 흐름

### `/geocode`

1. `q` 쿼리 문자열을 읽고 앞뒤 공백만 제거합니다.
2. 정규화된 문자열을 캐시 키로 사용해 fresh cache를 조회합니다.
3. 캐시 미스면 Nominatim을 호출합니다.
4. 성공 시 응답 본문 문자열을 그대로 캐시에 저장하고 클라이언트에 반환합니다.
5. 실패 시 `503 service unavailable`을 반환합니다.

Nominatim 호출 특성:

- `countrycodes=vn`
- `limit=1`
- 하노이 주변 `viewbox`
- `bounded=1`
- `accept-language=en`

의미:

- 전 세계 검색이 아니라 하노이 주변의 베트남 결과만 찾습니다.
- 응답은 서비스 내부 DTO로 재구성하지 않고 외부 JSON을 그대로 전달합니다.

### `/weather/{loc}`

1. 경로 파라미터 `loc`를 내부 위치 테이블에서 조회합니다.
2. 위치가 없으면 `400 unknown location`을 반환합니다.
3. `weather:{loc}` 키로 fresh cache를 조회합니다.
4. 캐시 미스면 Open-Meteo를 먼저 호출합니다.
5. Open-Meteo가 실패하면 경고 로그를 남기고 OpenWeatherMap을 호출합니다.
6. OpenWeatherMap까지 실패하면 stale cache를 확인합니다.
7. stale cache도 없으면 `503 service unavailable`을 반환합니다.

현재 지원 위치:

- `hoankiem`
- `minhchau`

## 캐시 설계

캐시는 `HashMap<String, CacheEntry>` 하나로 구성됩니다.

각 엔트리:

- `value: String`
- `expires_at: Option<Instant>`

조회 정책:

- `get_fresh`: 만료되지 않은 엔트리만 반환
- `get_stale`: 만료됐더라도 `expires_at + stale_window` 안이면 반환

TTL 정책:

- 지오코딩: TTL 없음
- 날씨: fresh TTL 1시간
- 날씨 stale window: 추가 2시간

이 설계의 장점:

- 구현이 단순합니다.
- 외부 API 지연과 호출 수를 빠르게 줄일 수 있습니다.
- 날씨 공급자 장애 시 마지막 성공값을 활용할 수 있습니다.

이 설계의 한계:

- 프로세스 재시작 시 캐시가 모두 사라집니다.
- 지오코딩 캐시는 무기한 유지되므로 오래된 결과가 남을 수 있습니다.
- 캐시 크기 제한과 eviction 정책이 없습니다.

## 외부 공급자 설계

### Geocode 공급자

- 공급자: OpenStreetMap Nominatim
- 반환 형식: 응답 문자열 그대로 전달
- 타임아웃: 코드에 별도 설정 없음

### Weather 1차 공급자

- 공급자: Open-Meteo
- 타임아웃: 700ms
- 사용 필드:
  - `temperature_2m`
  - `relative_humidity_2m`
  - `wind_speed_10m`

### Weather 2차 공급자

- 공급자: OpenWeatherMap
- 타임아웃: 1000ms
- 내부 표준 응답으로 변환 후 반환
- 온도는 Kelvin에서 Celsius로 변환

표준 날씨 응답 구조:

```json
{
  "current": {
    "temperature_2m": 0.0,
    "relative_humidity_2m": 0.0,
    "wind_speed_10m": 0.0
  }
}
```

## 오류 처리와 관측성

오류 응답:

- 잘못된 위치: `400`
- 외부 공급자 실패: `503`

로그:

- 시작 시 리스닝 주소를 `info`로 출력
- 공급자 요청 실패는 `error`
- 날씨 폴백 사용은 `warn`

로그 레벨은 `RUST_LOG`로 조정합니다.

## 현재 제약 사항

- 서버 시작 시 `OPENWEATHERMAP_API_KEY`가 항상 필요합니다.
- 이 값은 셸 환경 변수 또는 `.env` 파일에서 제공할 수 있습니다.
- `/weather/{loc}` 위치 집합이 코드에 고정되어 있습니다.
- health check, metrics, rate limit은 없습니다.
- 테스트 코드가 없습니다.
- geocode 응답은 외부 API 형식에 직접 결합되어 있습니다.

## 확장 방향

우선순위가 높은 개선 후보:

1. geocode TTL 추가 및 키 정규화 개선
2. 위치 목록을 설정 파일 또는 데이터 저장소로 분리
3. Redis 같은 외부 캐시 도입
4. `/health` 엔드포인트와 메트릭 추가
5. 외부 API 응답 스키마를 내부 DTO로 명확히 고정

# Podman 배포 가이드

이 문서는 이 저장소의 Rust HTTP 서버를 리눅스 서버에서 `podman`으로 실행하는 최소 절차를 정리합니다.

대상 전제:

- 서버에 `podman`이 이미 설치되어 있음
- 애플리케이션은 `OPENWEATHERMAP_API_KEY`가 없으면 시작하지 않음
- 애플리케이션은 기본적으로 컨테이너 내부 `3000` 포트에서 리슨함
- 캐시는 메모리 기반이므로 컨테이너 재시작 시 사라짐

## 1. 서버에 소스를 올립니다

서버에서 새 작업 디렉터리를 만들고 소스를 준비합니다.

```bash
mkdir -p ~/apps
cd ~/apps
git clone <REPO_URL> hanoi-cache
cd hanoi-cache
```

`git clone` 대신 로컬에서 서버로 파일을 복사해도 됩니다.

## 2. 배포용 환경 변수 파일을 만듭니다

서버의 저장소 루트에서 `deploy.env` 파일을 만듭니다.

```bash
cp .env.example deploy.env
vi deploy.env
```

최소한 아래 값은 실제 값으로 채워야 합니다.

```env
OPENWEATHERMAP_API_KEY=your-real-api-key
OPEN_METEO_TIMEOUT_MS=2000
OPENWEATHERMAP_TIMEOUT_MS=2000
WEATHER_CACHE_TTL_SECONDS=3600
PORT=3000
RUST_LOG=info
```

주의:

- `OPENWEATHERMAP_API_KEY`가 비어 있으면 컨테이너가 바로 종료됩니다.
- `PORT`는 컨테이너 내부 포트입니다. 이 문서에서는 `3000`을 유지합니다.

## 3. 이미지를 빌드합니다

저장소 루트에서 아래 명령을 실행합니다.

```bash
podman build -t localhost/hanoi-cache:latest -f Containerfile .
```

정상 빌드 여부 확인:

```bash
podman images | grep hanoi-cache
```

## 4. 컨테이너를 실행합니다

처음에는 루트리스 환경에서 다루기 쉬운 `8080:3000` 포트 매핑을 권장합니다.

```bash
podman run -d \
  --name hanoi-cache \
  --replace \
  --env-file ./deploy.env \
  -p 8080:3000 \
  localhost/hanoi-cache:latest
```

설명:

- `--replace`: 같은 이름 컨테이너가 있으면 교체
- `--env-file`: 애플리케이션 환경 변수를 파일로 주입
- `-p 8080:3000`: 서버 `8080` 포트를 컨테이너 `3000` 포트로 연결

## 5. 실행 직후 확인합니다

로그 확인:

```bash
podman logs -f hanoi-cache
```

기본 호출 확인:

```bash
curl "http://127.0.0.1:8080/geocode?q=Hoan%20Kiem%20Lake"
curl "http://127.0.0.1:8080/weather/hoankiem"
curl "http://127.0.0.1:8080/weather?latitude=21.2083286&longitude=105.433452"
curl "http://127.0.0.1:8080/weather"
```

다른 PC에서 접속한다면 서버 IP로 확인합니다.

```bash
curl "http://192.168.0.100:8080/geocode?q=Hoan%20Kiem%20Lake"
```

## 6. 자주 만나는 문제

### 컨테이너가 바로 종료됨

먼저 로그를 봅니다.

```bash
podman logs hanoi-cache
```

대표 원인:

- `OPENWEATHERMAP_API_KEY` 누락
- 서버에서 외부 API로 나가는 네트워크 차단
- 빌드는 됐지만 런타임 라이브러리 문제 발생

### 빌드 중 `rustc ... is not supported` 오류가 남

현재 lockfile 기준으로 일부 의존성은 `rustc 1.86` 이상을 요구할 수 있습니다.

이 저장소의 `Containerfile`은 builder 이미지를 `rust:1.86-slim-bookworm`으로 맞춰 두었습니다.
예전에 받은 오래된 `Containerfile`로 빌드 중이라면 최신 파일을 다시 반영한 뒤 재시도합니다.

### 서버 밖에서 접속이 안 됨

확인 순서:

1. `podman ps`로 컨테이너가 떠 있는지 확인
2. `ss -lntp | grep 8080` 또는 `podman port hanoi-cache`로 포트 바인딩 확인
3. 서버 방화벽에서 `8080/tcp` 허용 여부 확인

예시:

```bash
podman ps
podman port hanoi-cache
sudo firewall-cmd --list-ports
```

### 업데이트 후 재배포

같은 저장소 루트에서 아래 순서로 갱신합니다.

```bash
git pull
podman build -t localhost/hanoi-cache:latest -f Containerfile .
podman run -d \
  --name hanoi-cache \
  --replace \
  --env-file ./deploy.env \
  -p 8080:3000 \
  localhost/hanoi-cache:latest
```

## 7. 재부팅 후 자동 시작

수동 실행이 안정화된 뒤에 `systemd --user`로 연결하는 것을 권장합니다.

컨테이너가 실행 중인 상태에서:

```bash
mkdir -p ~/.config/systemd/user
podman generate systemd --name hanoi-cache --files
mv container-hanoi-cache.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now container-hanoi-cache.service
loginctl enable-linger "$USER"
```

주의:

- `podman 3.x` 환경에서는 `generate systemd` 결과가 버전에 따라 조금 다를 수 있습니다.
- 먼저 수동 실행이 안정적인지 확인한 뒤 자동 시작을 붙이는 편이 문제 분리에 유리합니다.

## 8. 운영 메모

- 이 서비스는 `/weather/{loc}`에서 고정 슬러그를 지원하고, `/weather?latitude&longitude`에서 GPS 좌표를 지원합니다.
- `/weather`에서 좌표가 없거나 잘못되면 기본 위치 `hoankiem`으로 처리합니다.
- 날씨 응답은 현재값 `current`와 일별 요약 `daily`를 함께 포함합니다.
- 캐시는 메모리 기반이라 컨테이너 재시작 시 모두 초기화됩니다.
- 현재 `/health` 엔드포인트는 없습니다. 운영 체크는 실제 API 호출이나 로그 확인 기준으로 해야 합니다.

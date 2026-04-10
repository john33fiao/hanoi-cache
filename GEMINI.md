# GEMINI.md

이 문서는 `CLAUDE.md`를 추종합니다.
이 저장소의 표준 에이전트 지침은 `CLAUDE.md`이며, 충돌 시 `CLAUDE.md`를 우선합니다.

## 시작 규칙

- 작업 전 `CLAUDE.md`를 먼저 읽습니다.
- 이어서 `README.md`와 `docs/architecture.md`를 확인합니다.
- 코드와 문서가 어긋나면 함께 수정합니다.

## 작업 규칙

- 문서와 사용자 대상 설명은 한국어를 기본으로 합니다.
- 현재 구조를 유지하며 최소 변경 원칙으로 작업합니다.
- 날씨 API는 `/weather/{loc}` 고정 슬러그와 `/weather?latitude&longitude` GPS 좌표를 함께 지원하며, 좌표가 없거나 잘못되면 기본 위치 `hoankiem`으로 처리합니다.
- 날씨 응답은 `current`와 `daily`를 함께 포함합니다.
- 구현 전에는 `skills/rtd-before/SKILL.md`를 사용해 계획과 범위를 정리합니다.
- 구현 후에는 `skills/rtd-after/SKILL.md`를 사용해 회귀와 READY 여부를 점검합니다.
- 저장소에 실제로 없는 파일이나 검증 절차를 추측해서 쓰지 않습니다.

## 동기화 규칙

- 이 문서는 `CLAUDE.md`의 추종 문서입니다.
- 에이전트 지침 변경 시 `CLAUDE.md`를 먼저 수정하고, 이후 이 파일을 동기화합니다.

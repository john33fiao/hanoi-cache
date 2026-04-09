# AGENTS.md

이 문서는 `CLAUDE.md`를 추종합니다.
이 저장소의 에이전트 기준 문서는 `CLAUDE.md`이며, 충돌 시 `CLAUDE.md`를 우선합니다.

## 시작 규칙

- 먼저 `CLAUDE.md`를 읽습니다.
- 이어서 `README.md`와 `docs/architecture.md`를 확인합니다.
- API 의미나 사용법이 바뀌면 문서도 함께 갱신합니다.

## 작업 규칙

- 문서와 사용자 대상 설명은 한국어로 유지합니다.
- 현재의 단순한 Rust 구조를 유지하고 과도한 설계를 피합니다.
- 구현 전에는 `skills/rtd-before/SKILL.md`의 계획 게이트를 따릅니다.
- 구현 후에는 `skills/rtd-after/SKILL.md`의 품질 게이트를 따릅니다.
- 없는 파일, 없는 테스트, 없는 실행 경로를 가정하지 않습니다.

## 동기화 규칙

- 이 파일은 `CLAUDE.md` 요약본입니다.
- 지침 변경은 `CLAUDE.md`를 먼저 수정한 뒤 이 파일에 반영합니다.

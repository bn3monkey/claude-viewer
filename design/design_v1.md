# 클로드 뷰어 설계 v1 — v0 구현 후 추가된 것

v0 설계([design_v0.md](design_v0.md))는 그대로 유효하다. 이 문서는 실제 CLI(2.1.314) 동작에 맞춰 바뀐 점과 v1 에서 추가한 기능만 적는다.

## 1. 실제 CLI 와 다르게 구현한 점

| 항목 | v0 설계 | 실제 |
|---|---|---|
| 세션 폴더 인코딩 | `/ \ :` → `-` | 영숫자가 아닌 모든 문자 → `-`. 드라이브 문자 대소문자가 다를 수 있어 대소문자 무시로 찾는다 |
| 세션 제목 | `summary` 줄 | `ai-title` 줄(`aiTitle`) 우선, 없으면 첫 사용자 메시지. `@파일` 첨부 줄은 건너뛴다 |
| 외부 세션 완료 판정 | 마지막 줄이 `result` | 로그에 `result` 가 없다. `assistant.stop_reason` 이 `tool_use` 가 아니고 60초간 쓰기가 없으면 완료 |
| 새 세션 id | — | `--session-id <uuid>` 로 앱이 정한다. 첫 메시지 전에는 프로세스를 띄우지 않는다 |
| 도구 권한 | `--permission-mode` 만 | **`--permission-prompt-tool stdio`** 가 없으면 질문이 전부 거부된다. 있으면 stdout 으로 `control_request(can_use_tool)` 가 오고 stdin 으로 `control_response` 를 보낸다 |
| Compact | — | `/compact` 를 사용자 메시지로 보내면 된다. 요약은 `This session is being continued…` 로 시작하는 user 메시지로 온다 |
| 사용량 | — | `rate_limit_event.rate_limit_info.unifiedWindows` (`five_hour`, `seven_day`, …) 와 `result.modelUsage[model].contextWindow` |
| WebView2 | `webviewInstallMode` | 인스톨러 없이 exe 만 배포하므로 동작하지 않는다. 런타임이 없으면 안내창을 띄우고 설치 페이지를 연다 |

## 2. v1 기능

| 기능 | 구현 |
|---|---|
| HTML 앱 안에서 렌더링 | `htmlview` 커스텀 URI 스킴이 프로젝트 안 파일을 서빙하고 `<iframe sandbox="allow-scripts">` 로 띄운다. URL 이 파일 경로 구조를 유지해 상대 경로 css·이미지가 그대로 풀린다. srcdoc 은 앱 CSP 를 물려받아 스크립트·외부 css 가 막히므로 쓰지 않는다 |
| Alt+Enter Commit | Ctrl/⌘+Enter 와 함께 |
| 대화 ↔ 입력창 분할 | `react-resizable-panels` 세로 분할, 비율은 `localStorage` 에 저장 |
| 뷰어 탭 우클릭 | 닫기 / 다른 탭 모두 닫기 / 모든 탭 닫기 |
| 디렉터리 트리 | `.gitignore`·`node_modules` 와 상관없이 전부 보인다(`.git` 만 숨김). `ignore` 크레이트 대신 `std::fs::read_dir` 너비 우선 탐색. 5만 개 상한에 걸려도 얕은 항목이 먼저 들어가므로 큰 폴더의 깊은 곳만 잘린다. 심볼릭 링크·정션은 따라가지 않는다. v0 의 `respectIgnore` 설정은 없앴다 |
| 프로젝트 처음 열기 | 활성 세션 없이 빈 프로젝트로 시작한다. 기존 세션은 "새 활성화 세션" 다이얼로그에서 고른다 |
| Git 토글 | 트리 헤더의 Tree / Git. 현재 브랜치, 브랜치 목록(최근 커밋순), 선택 브랜치의 커밋 목록(해시 6자리 · MM-DD HH:mm · 제목 16자, 누르면 그 아래로 `git show -s --format=fuller` 전체 로그가 펼쳐짐), 선택 브랜치의 변경 파일. git 출력은 사용자 `i18n.logOutputEncoding` 과 무관하게 `-c i18n.logOutputEncoding=UTF-8` 로 받는다. 현재 브랜치면 작업 트리(`git status --porcelain`, 미추적 포함), 다른 브랜치면 `git diff --name-status HEAD...branch`. 파일을 누르면 뷰어에 unified diff 탭(`shiki` diff 문법). 텍스트·소스·html·md 만 |
| 권한 요청 | 대화 안에 카드로 표시. 허용 / 항상 허용(CLI 의 `permission_suggestions` 를 `updatedPermissions` 로 돌려줌) / 거부. 입력창 옆 권한 드롭다운으로 `--permission-mode` 선택(바꾸면 다음 메시지 때 `--resume` 으로 다시 띄움) |
| 터미널로 실행 | `.exe .bat .cmd .ps1 .sh .bash` 우클릭 → cmd / powershell / git bash / macOS Terminal 로 실행 (다른 OS 항목은 비활성). `portable-pty` 로 PTY 를 열고 오른쪽 뷰어에 `xterm` 탭으로 띄운다. 작업 폴더는 파일 폴더, 실행 뒤에도 셸이 남아 입력할 수 있다(cmd `/K`, PowerShell `-NoExit`, bash `-c '…; exec bash -l -i'`). 탭을 닫으면 프로세스 트리를 끝낸다. 터미널 id 는 프론트가 정하고 출력 구독을 먼저 마친 뒤 실행한다 — ConPTY 는 시작 시 커서 위치 질의(`ESC[6n`)에 답이 올 때까지 출력을 멈추기 때문 |
| 더블클릭 | `.exe` 는 바로 실행, 스크립트는 확장자별 기본 터미널(.bat/.cmd → cmd, .ps1 → PowerShell, .sh/.bash → git bash, macOS 는 Terminal 셸) |
| 설정 창 | 상단 "설정" 버튼. cmd / PowerShell / git bash / macOS Terminal 경로. 비우면 자동 감지(`ComSpec`, `WindowsPowerShell\v1.0`, `where git` 기준 `bin\bash.exe`, `/bin/zsh`) |
| .exe 실행 | 트리에서 `.exe` 더블클릭 또는 우클릭 → 실행하기. 작업 폴더는 exe 가 있는 폴더, 콘솔 프로그램은 새 콘솔 창(`CREATE_NEW_CONSOLE`). 관리자 권한이 필요한 exe 는 `start` 로 다시 시도해 UAC 창을 띄운다. 한 번 클릭은 선택만 하고 뷰어 탭을 열지 않는다. 프로젝트 밖·다른 확장자는 백엔드에서 거부 |
| 사용량 바 | 세션 상단. Context 토큰/창 크기(누르면 `/compact`), 세션 비용·출력 토큰, 5시간 창, 7일 창, 그 외 창(모델별 창이 오면 Fable 칸에 표시) |

## 3. 레이어 추가

- `domain/entity/git.rs`, `domain/repository/git_repository.rs`, `domain/usecase/git/`, `data/datasource/git_cli.rs`, `data/repository/git_repository_impl.rs`, `presentation/viewmodel/git_vm.rs`
- `ChatEvent` 에 `PermissionRequest`, `Usage`, `RateLimit`, `Compacted` 추가. `Result` 에 `context_window`
- `ViewerMode` 에 `Html`, `Diff`

## 4. 확인한 제약

- 같은 세션에서 한 번 허용한 도구(예: 프로젝트 안 Write)는 CLI 가 다시 묻지 않는다.
- `Fable` 전용 사용량 창은 CLI 가 `unifiedWindows` 에 그런 키를 줄 때만 채워진다. Haiku 로 확인한 응답에는 `five_hour`, `seven_day` 만 있었다.

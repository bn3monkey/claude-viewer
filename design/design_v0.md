# 클로드 뷰어 설계 v0 — 기술 선택과 제약

대상: [requirement_v0.md](../requirement/requirement_v0.md)
GUI 초안: [gui_draft_v0.html](../gui_draft/gui_draft_v0.html)

## 1. 결론

| 항목 | 선택 |
|---|---|
| 앱 프레임워크 | **Tauri 2** (Rust 백엔드 + OS 내장 WebView) |
| 백엔드 언어 | **Rust** (stable, edition 2021) |
| 프론트엔드 | **TypeScript + React 18 + Vite** |
| 배포 형태 | Windows: 단일 `claude_viewer.exe` / macOS: `Claude Viewer.app` (안에 단일 Mach-O 바이너리) |
| Claude 연동 방식 | 설치된 **Claude Code CLI 바이너리를 자식 프로세스로 실행**하고 `stream-json` 으로 대화. SDK 미사용 |

### 왜 Tauri 인가

| 후보 | 단일 exe | Win + macOS | PDF/docx/md/mp4 렌더링 | 비고 |
|---|---|---|---|---|
| **Tauri 2** | ◎ (수 MB) | ◎ | ◎ WebView 가 전부 처리 | Rust 학습 비용. WebView2 런타임 의존(아래 참고) |
| Electron | △ (portable exe 150MB+, 첫 실행 시 임시폴더 압축 해제) | ◎ | ◎ | Node 가 있어 Agent SDK 직접 사용 가능하나 "단일 exe" 제약을 가장 나쁘게 만족 |
| Wails (Go) | ◎ | ◎ | ◎ | Tauri 와 동급. 생태계·플러그인(다이얼로그, 셸, 파일감시)이 Tauri 가 더 풍부 |
| Avalonia (C#) | ○ (self-contained single file, 60MB+) | ◎ | ✕ PDF/docx 는 별도 상용·반쪽 라이브러리 필요 | 뷰어 요구사항 구현 비용이 가장 큼 |
| PySide6 / PyInstaller | △ (onefile 느림, 100MB+) | ◎ | △ | 제외 |

요구사항의 핵심 비용이 **다양한 포맷의 읽기 전용 뷰어**인데, 이 영역은 브라우저 엔진이 가장 저렴하게 해결한다. 그 위에서 바이너리 크기와 단일 파일 제약을 가장 잘 만족하는 것이 Tauri 다.

### "단일 .exe" 제약의 해석

- Windows: Tauri 가 만드는 exe 는 프론트엔드 자산을 바이너리에 포함하므로 **exe 하나만 복사해도 실행**된다. 렌더링은 OS 의 **WebView2 런타임** 을 사용하며, 런타임 설치는 허용된 제약이다(결정됨). Windows 10/11 에는 기본 설치되어 있고, 없을 경우 첫 실행 시 `webviewInstallMode: downloadBootstrapper` 로 자동 설치한다. 런타임을 exe 에 박는 `fixedRuntime` 은 쓰지 않는다.
- macOS: WKWebView 가 OS 에 내장이라 추가 의존성이 없다. 배포 단위는 `.app` 번들이지만 내부 실행 파일은 하나다.
- 인스톨러(MSI/NSIS/DMG)는 만들지 않는다. 설정은 `%APPDATA%\claude_viewer\` / `~/Library/Application Support/claude_viewer/` 에 JSON 으로 저장한다.

## 2. Claude 연결 설계

실제 환경(Windows, Claude Code 2.1.314)에서 확인한 경로를 기준으로 한다.

### 자동 찾기 순서

| 순위 | 위치 | 비고 |
|---|---|---|
| 1 | `~/.vscode/extensions/anthropic.claude-code-<ver>-<platform>/resources/native-binary/claude(.exe)` | VSCode 확장 동봉 바이너리. 여러 버전이 있으면 가장 높은 버전 선택. `~/.vscode-insiders`, `~/.cursor` 도 같은 규칙으로 검사 |
| 2 | `~/.local/bin/claude(.exe)` | 네이티브 설치 스크립트 기본 위치 (이 머신에서 확인) |
| 3 | `%APPDATA%\npm\claude.cmd`, `/opt/homebrew/bin/claude`, `/usr/local/bin/claude` | npm 글로벌 / Homebrew |
| 4 | `PATH` 탐색 (`where claude` / `which claude`) | 최후 수단 |

찾은 바이너리는 `claude --version` 을 실행해 검증하고, 상단 바에 버전을 표시한다.

### 세션 데이터 위치와 해석

- 세션 로그: `~/.claude/projects/<인코딩된 프로젝트 경로>/<session-id>.jsonl`
  - 인코딩 규칙: 절대 경로의 `/`, `\`, `:` 를 `-` 로 치환 (예: `d:\repo\claude_viewer` → `d--repo-claude-viewer`). 한글 등 비ASCII 문자는 `-` 로 치환되므로, 폴더 이름만으로 역변환은 불가하다. 따라서 **프로젝트 폴더는 사용자가 File Picker 로 고르고, 앱이 같은 규칙으로 인코딩해 세션 폴더를 찾는다.**
  - 한 줄이 JSON 하나. `type` 이 `user` / `assistant` 인 줄이 대화이며 `cwd`, `sessionId`, `timestamp`, `gitBranch`, `version` 필드를 가진다.
- 세션 이름: 첫 `user` 메시지의 텍스트 앞부분을 기본값으로 쓰고, 로그에 요약(`summary`)이 기록되면 그것으로 교체한다. 새로 만든 세션은 첫 메시지 전까지 `New Session...` 으로 표시한다.
- 진행 여부
  - 이 앱이 띄운 세션: 자식 프로세스가 살아 있고 마지막 `result` 메시지를 받기 전이면 **진행 중(빨강)**, `result` 를 받으면 **완료(초록)**.
  - 다른 곳(VSCode, 터미널)에서 돌고 있는 세션: `.jsonl` 파일을 `notify` 로 감시하여 최근 N초 안에 쓰기가 있고 마지막 줄이 `assistant`/tool 이면 진행 중, 마지막 줄이 `result` 또는 일정 시간 변경이 없으면 완료로 본다. (휴리스틱이므로 v0 에서는 "이 앱이 띄운 세션" 만 정확히 보장)

### 세션 실행 / 재개 / 중단

```text
# 새 세션
claude -p --output-format stream-json --input-format stream-json --verbose --model <model> [--permission-mode ...]
# 기존 세션 재개 (새 활성화 세션 다이얼로그에서 선택)
claude -p --resume <session-id> --output-format stream-json --input-format stream-json --verbose
```

- stdin 으로 `{"type":"user","message":{...}}` 를 한 줄씩 보내고, stdout 의 `stream-json` 을 줄 단위로 파싱해 Session UI 에 그린다. `--verbose` 가 있어야 중간 assistant/tool 이벤트가 전부 나온다.
- Commit = stdin 에 메시지 1줄 쓰기. 선택한 파일은 메시지 본문 앞에 `@상대경로` 로 붙여 보낸다.
- Abort = 자식 프로세스에 interrupt. Windows 는 `GenerateConsoleCtrlEvent(CTRL_C)` 가 콘솔 공유 문제로 불안정하므로 v0 에서는 **프로세스 kill 후 `--resume` 으로 다시 띄우는 방식**으로 구현한다. (stream-json 의 control 메시지로 interrupt 가 가능한지는 CLI 버전별로 확인 필요)
- 모델 드롭다운 값은 `--model` 로 전달. 목록은 앱 설정 JSON 에 고정(`claude-fable-5-1`, `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`)하고 사용자가 편집할 수 있게 한다.
- "AI 가 File Viewer 에 파일을 열어 달라고 요청하는 API" 는 v0 에서 **대화 출력 안의 지시문을 파싱**하는 방식으로 시작한다(예: 응답 텍스트에 `<viewer open="path" mode="markdown"/>` 가 나오면 앱이 열어줌). 정식으로는 로컬 MCP 서버를 앱 안에 띄우고 `--mcp-config` 로 주입하는 방식으로 확장한다.

## 3. 기능별 라이브러리

### Rust (백엔드, `src-tauri/`)

| 기능 | 크레이트 | 비고 |
|---|---|---|
| 앱 셸·IPC·창 | `tauri` 2.x | 프론트와 `invoke`/`emit` 으로 통신 |
| 파일·폴더 선택 다이얼로그 | `tauri-plugin-dialog` | 📁 아이콘, 수동 찾기 |
| 탐색기/Finder 로 열기, 브라우저로 열기 | `tauri-plugin-opener` | `reveal_item_in_dir`, `open_url` |
| 클립보드 (경로 복사) | `tauri-plugin-clipboard-manager` | 파일 자체 복사는 아래 참고 |
| 파일/폴더 복사 (클립보드) | Windows `clipboard-win` (CF_HDROP) / macOS `objc2-app-kit` (NSPasteboard fileURL) | OS 별 cfg 분기. v0 에서는 "경로 복사"만 두고 파일 복사는 v1 |
| 자식 프로세스 실행·스트림 읽기 | `tokio` (process, io) | `tauri-plugin-shell` 대신 직접 제어(stdin 쓰기·kill 필요) |
| JSONL 파싱 | `serde`, `serde_json` | 라인별 `Value` 파싱 후 필요한 필드만 struct 로 |
| 디렉터리 트리 | `walkdir` + `ignore` | `.gitignore`·`node_modules` 제외 옵션 |
| 파일 시스템 감시 | `notify` + `notify-debouncer-full` | 프로젝트 트리 자동 갱신, 세션 `.jsonl` 변경 감지 |
| 삭제 (휴지통) | `trash` | 영구 삭제 대신 휴지통 이동 |
| 설정 저장 | `tauri-plugin-store` | 프로젝트 목록, 모델 목록, Claude 경로 |
| 홈/AppData 경로 | `dirs` | 자동 찾기 후보 경로 생성 |
| 글로브 (확장 버전 폴더 찾기) | `glob` | `anthropic.claude-code-*` |
| entity → TypeScript 타입 생성 | `ts-rs` | `domain/entity` 를 프론트와 공유 (4.3 참고) |
| 에러 타입 | `thiserror` | `AppError` 정의 |

### TypeScript (프론트엔드, `src/`)

| 기능 | 라이브러리 | 비고 |
|---|---|---|
| UI 프레임워크 | `react`, `react-dom`, `vite`, `typescript` | |
| 상태 관리 | `zustand` | 프로젝트/세션/뷰어 상태. Redux 보다 가벼움 |
| 레이아웃 분할 (드래그로 폭 조절) | `react-resizable-panels` | 좌/중/우 3단 + 좌측 상하 20% |
| 트리 뷰 + 드래그 이동 | `react-arborist` | 가상 스크롤, D&D, inline rename 내장 |
| 컨텍스트 메뉴, 드롭다운, 다이얼로그 | `@radix-ui/react-context-menu`, `-dropdown-menu`, `-dialog` | 접근성·키보드 처리 포함. 스타일은 직접 |
| 아이콘 | `lucide-react` | 폴더·파일·상태 아이콘 |
| Markdown 렌더링 | `react-markdown` + `remark-gfm` + `rehype-sanitize` | 세션 대화 출력과 .md 미리보기 공용 |
| 코드 구문 강조 (읽기 전용) | `shiki` | VSCode 와 같은 TextMate 문법. `.md`/`.html` 소스 보기에도 사용 |
| PDF 렌더링 | `pdfjs-dist` | canvas 렌더, 페이지 이동/줌 툴바는 직접 |
| DOCX 렌더링 | `docx-preview` | 문단·표·이미지를 HTML 로. 레이아웃 충실도가 `mammoth` 보다 높음 |
| HTML 렌더링 (브라우저로 열기) | 없음 → `tauri-plugin-opener` 로 외부 브라우저 | 앱 안에서 임의 HTML 을 실행하지 않음(보안) |
| 이미지 (.png .jpeg .webp .bmp) | 없음 → `<img>` | Tauri `asset:` 프로토콜로 로컬 파일 접근 |
| 동영상 (.mp4) | 없음 → `<video controls>` | WebView2 와 WKWebView 모두 H.264/AAC mp4 재생 지원 |
| 세션 대화 가상 스크롤 | `@tanstack/react-virtual` | 긴 세션 성능 |

#### 뷰어 열기 규칙 (.md / .html)

두 방법을 항상 같이 제공한다.

| 동작 | .md | .html |
|---|---|---|
| 트리에서 **왼쪽 클릭** | `shiki` 로 구문 강조된 **소스 텍스트** 탭 | `shiki` 로 구문 강조된 **소스 텍스트** 탭 |
| 트리에서 **우클릭 → 특수 항목** | "Markdown 미리보기로 열기" → `react-markdown` 으로 **렌더링된 탭** (소스 탭과 별도 탭) | "Browser로 열기" → `tauri-plugin-opener` 로 **기본 브라우저**에서 렌더링 |

뷰어 탭은 `(경로, 모드)` 쌍으로 식별하므로 같은 .md 의 소스 탭과 미리보기 탭이 동시에 열려 있을 수 있다.

## 4. 구조

백엔드(Rust)는 **Clean Architecture** 로 짠다. 의존 방향은 바깥 → 안쪽 한 방향이며, `domain` 은 어떤 바깥 레이어도 모른다.

```text
presentation ─┐
              ├─→ domain ←─ data ←─ infrastructure
di ───────────┘   (entity / usecase / repository trait)
```

### 4.1 디렉터리

```text
claude_viewer/
├─ src/                          # React (TypeScript) — 프론트엔드
│  ├─ app/                       # 레이아웃(3단 분할), 테마
│  ├─ features/
│  │  ├─ connection/             # ① 경로·자동찾기·수동찾기
│  │  ├─ projects/               # ② 프로젝트·세션 목록, 새 활성화 세션 다이얼로그
│  │  ├─ tree/                   # ③ 디렉터리 트리, 컨텍스트 메뉴, D&D
│  │  ├─ session/                # ④ 대화 뷰, 입력, 모델 선택, Commit/Abort
│  │  └─ viewer/                 # ⑤ 포맷별 뷰어 컴포넌트
│  ├─ viewmodel/                 # 백엔드 viewmodel 과 1:1 대응하는 훅 (useSessionVM 등)
│  └─ generated/entity/          # ★ Rust domain/entity 에서 자동 생성된 TS 타입 (수정 금지)
│
├─ src-tauri/src/
│  ├─ main.rs                    # tauri::Builder, di::build() 호출만
│  ├─ di/                        # 조립. 모든 Repository 구현체·UseCase 를 생성해 AppState 에 넣음
│  │  └─ mod.rs                  #   build(platform: Platform) -> AppState
│  ├─ presentation/
│  │  └─ viewmodel/              # 프론트엔드와의 유일한 접점
│  │     ├─ connection_vm.rs     #   #[tauri::command] 함수 + emit 이벤트
│  │     ├─ project_vm.rs
│  │     ├─ tree_vm.rs
│  │     ├─ session_vm.rs
│  │     └─ viewer_vm.rs
│  ├─ domain/
│  │  ├─ entity/                 # ★ 프론트와 공유되는 순수 데이터 타입 (serde + ts-rs)
│  │  │  ├─ claude_binary.rs     #   ClaudeBinary { path, version, source }
│  │  │  ├─ project.rs           #   Project { root, name, done, total }
│  │  │  ├─ session.rs           #   Session { id, project, title, state: Running|Done }
│  │  │  ├─ chat.rs              #   ChatEvent (user / assistant / tool / result …)
│  │  │  ├─ fs_node.rs           #   FsNode { path, kind: Dir|File, children }
│  │  │  └─ viewer.rs            #   ViewerRequest { path, mode: Source|Markdown|Pdf|… }
│  │  ├─ usecase/                # 비즈니스 로직. 기능별 폴더, 함수 형태
│  │  │  ├─ connection/          #   find_claude_auto(), set_claude_manual(), verify_claude()
│  │  │  ├─ project/             #   open_project(), close_project(), list_projects()
│  │  │  ├─ session/             #   list_sessions(), start_session(), resume_session(),
│  │  │  │                       #   send_message(), abort_session(), close_session()
│  │  │  ├─ tree/                #   read_tree(), rename(), move_(), delete(), create_file(), copy_path()
│  │  │  └─ viewer/              #   resolve_viewer_mode(), open_external()
│  │  └─ repository/             # trait 만. 구현은 data/ 에
│  │     ├─ claude_binary_repository.rs   # locate_candidates(), probe_version()
│  │     ├─ session_repository.rs         # scan(project), watch(project), read_log(id)
│  │     ├─ claude_process_repository.rs  # spawn(), write_stdin(), kill(), subscribe()
│  │     ├─ file_system_repository.rs     # list(), rename(), move_(), trash(), create()
│  │     ├─ clipboard_repository.rs       # write_text(), write_files()
│  │     ├─ shell_repository.rs           # reveal_in_explorer(), open_in_browser()
│  │     └─ settings_repository.rs        # load(), save()
│  ├─ data/
│  │  ├─ repository/             # domain/repository trait 의 실제 구현 (impl … for …)
│  │  │  ├─ claude_binary_repository_impl.rs
│  │  │  ├─ session_repository_impl.rs
│  │  │  ├─ claude_process_repository_impl.rs
│  │  │  ├─ file_system_repository_impl.rs
│  │  │  ├─ clipboard_repository_impl.rs
│  │  │  ├─ shell_repository_impl.rs
│  │  │  └─ settings_repository_impl.rs
│  │  └─ datasource/             # 외부 형식을 다루는 저수준 코드
│  │     ├─ jsonl_session_log.rs #   ~/.claude/projects/**/*.jsonl 파서
│  │     ├─ stream_json.rs       #   claude CLI stdin/stdout 프로토콜 인코더·디코더
│  │     ├─ settings_store.rs    #   tauri-plugin-store 래퍼
│  │     └─ fs_walker.rs         #   walkdir + ignore
│  └─ infrastructure/            # OS 의존 기능. cfg(target_os) 분기는 여기서만
│     ├─ platform.rs             #   Platform trait: home_dir(), app_data_dir(), candidate_paths()
│     ├─ windows/                #   CF_HDROP 클립보드, explorer.exe /select, .exe 탐색 경로
│     ├─ macos/                  #   NSPasteboard, open -R, Homebrew 경로
│     ├─ process.rs              #   tokio::process 래퍼, kill / interrupt
│     ├─ watcher.rs              #   notify 래퍼
│     └─ trash.rs                #   trash 크레이트 래퍼
│
└─ requirement/ gui_draft/ design/
```

### 4.2 레이어별 규칙

| 레이어 | 역할 | 의존 허용 | 크레이트 |
|---|---|---|---|
| `di/` | 구현체를 고르고 조립. `Platform` 을 보고 Windows/macOS 구현을 주입. 테스트에서는 Fake 리포지토리 주입 | 전부 | — |
| `presentation/viewmodel/` | `#[tauri::command]` 함수와 `app.emit()` 이벤트. 입력 검증과 entity ↔ 응답 변환만 하고 로직 없음 | domain | `tauri` |
| `domain/entity/` | 순수 데이터. `#[derive(Serialize, Deserialize, TS)]` 로 프론트와 공유 | 없음 | `serde`, `ts-rs` |
| `domain/usecase/` | 기능별 폴더 안에 **자유 함수**. 인자로 repository trait 객체(`&dyn XxxRepository`)를 받음 | entity, repository trait | — |
| `domain/repository/` | trait 정의만 | entity | — |
| `data/repository/` | trait 구현. datasource 와 infrastructure 를 조합 | domain, datasource, infrastructure | `serde_json` |
| `data/datasource/` | 파일 형식·프로토콜 파싱, 외부 저장소 접근 | entity | `serde_json`, `walkdir`, `ignore`, 플러그인 |
| `infrastructure/` | OS 의존 호출. 여기 밖에서는 `cfg(target_os)` 금지 | 없음 (domain 도 모름) | `tokio`, `notify`, `trash`, `clipboard-win`, `objc2-*`, `dirs` |

### 4.3 프론트엔드와의 연결

- **entity 공유**: `domain/entity/*.rs` 에 `#[derive(ts_rs::TS)] #[ts(export)]` 를 붙이고 `cargo test` 시 `src/generated/entity/*.ts` 로 내보낸다. 프론트는 이 타입만 import 하며 손으로 타입을 중복 정의하지 않는다.
- **viewmodel ↔ 프론트**: viewmodel 하나가 `invoke` 명령 묶음과 `emit` 이벤트 묶음을 가진다. 프론트의 `src/viewmodel/useXxxVM.ts` 가 같은 이름으로 1:1 대응해 `invoke` 를 감싸고 `listen` 으로 상태를 갱신한다.
  - 요청/응답: `invoke("session_send_message", { sessionId, text })` → `Result<(), AppError>`
  - 스트림/푸시: `session://event/{sessionId}` 로 `ChatEvent` 전송, `project://changed` 로 `Project` 전송, `tree://changed` 로 변경된 `FsNode` 전송
- **에러**: `domain` 에 `AppError` enum 하나를 두고 viewmodel 이 그대로 직렬화해 보낸다. 프론트는 `generated/entity/AppError.ts` 로 받아 토스트에 표시한다.
- **세션 스트림 흐름**: `claude_process_repository_impl` 이 stdout 을 읽어 `stream_json` 데이터소스로 `ChatEvent` 로 바꾸고 채널에 넣는다 → `usecase::session::subscribe` 가 채널을 돌려준다 → `session_vm` 이 `emit` 한다. 프론트는 로직 없이 그리기만 한다.

프론트는 파일 시스템과 프로세스를 직접 만지지 않고 전부 viewmodel 명령으로만 접근한다. Tauri 의 `fs` 스코프는 사용자가 연 프로젝트 폴더와 `~/.claude/projects` 로 제한한다.

## 5. v0 에서 미루는 것 / 확인이 필요한 것

- Abort 를 프로세스 kill 이 아닌 진짜 interrupt 로 구현하는 방법 (stream-json control 메시지 지원 여부).
- 다른 클라이언트에서 돌고 있는 세션의 "진행 중" 판정 정확도.
- 파일 자체를 OS 클립보드에 복사하는 기능 (경로 복사는 v0 포함).
- Linux 는 요구사항 밖이지만 Tauri 라 추가 비용은 WebKitGTK 의존성 정도다.

# 클로드 뷰어 설계 v1.1 — 분할 화면 · 프로젝트별 레이아웃 · 인터넷 탭

요구사항은 [requirement_v1.1.0.html](../requirement/requirement_v1.1.0.html). 이 문서는 구현하면서 정한 것과 요구사항 문서와 달라진 점만 적는다.

## 1. 상태 모델

| 항목 | 구현 |
|---|---|
| 뷰어 상태 | `useViewerVM` 이 `layouts: Record<루트, Layout>` 과 `current` 를 가진다. `Layout = { panes(1~2), focused, ratio, sessionHidden }`, `Pane = { id, tabs, activeKey }`. 레이아웃이 없는 프로젝트는 공용 `BLANK` 를 보고, 처음 고칠 때 만든다 |
| 정규화 | 고칠 때마다 `normalize` 가 빈 분할(둘일 때)을 없애고 활성 탭·포커스가 없는 것을 가리키지 않게 맞춘다. 탭 키는 한 레이아웃 안에서 하나뿐이다 |
| 열기 대상 | `open(request, extra, { target, root })`. `focused`(트리·뷰어·터미널·F12), `left`(세션: `viewer://open` 과 대화 링크, 포커스는 그대로), `{ pane }`(인터넷 탭). 이미 열린 탭이면 그 분할에서 활성화한다 |
| 세션 출처 | 대화 링크는 `Markdown` 의 `LinkOpenTarget` 컨텍스트(세션 패널이 `"left"` 를 넣음)로 구분한다. `viewer://open` 은 경로가 속한 프로젝트(`projectRootOf`, 가장 깊은 루트)의 레이아웃 왼쪽 분할에 들어간다 — 다른 프로젝트 세션이 연 파일은 그 프로젝트로 돌아갔을 때 보인다 |
| 프로젝트 전환 | `App` 의 선택 effect 가 트리와 함께 `switchProject(root)` 를 부른다. 프로젝트 닫기는 `dropProject` (터미널 종료·웹뷰 제거) |

## 2. 저장

- `Settings.viewer_layouts: BTreeMap<루트, SavedLayout>`. `SavedLayout { panes: [SavedPane { tabs, active }], focused, ratio, session_hidden }`.
- `viewer_save_layouts` 는 터미널 탭을 빼고, 탭이 없고 대화창도 보이는 레이아웃은 저장하지 않는다. 비율은 20~80 으로 자른다.
- `viewer_restore_layouts` 는 닫힌 프로젝트의 레이아웃을 버리고, 분할마다 없어진 파일을 거른다. 인터넷 탭은 `is_web_url` 만 본다.
- 1.0.x 의 `viewer_tabs` 만 있으면 탭을 경로가 속한 프로젝트(canonicalize 후 접두사)의 단일 분할 레이아웃으로 나누고 **복원하면서 바로 저장**한다. 어디에도 속하지 않는 탭은 `""` 로 간다. `viewer_tabs`·`active_tab` 은 비어 있으면 직렬화하지 않는다.
- 분할 비율은 `PanelGroup.onLayout` 으로 레이아웃에 들어간다(`autoSaveId` 를 쓰지 않음). 세션 대화창 너비는 지금처럼 `layout-main`.

## 3. 분할과 끌기

- 탭 끌기는 포인터 이벤트로 직접 한다(`beginTabDrag`). 처음엔 HTML5 DnD 였으나 WebView2 에서 끌기가 곧바로 취소되어 분할이 안 되고 `dragging` 이 남아 인터넷 탭이 계속 숨는 버그가 있었다. 5px 이상 움직이면 끌기로 보고 포인터를 capture 하며, 놓일 자리는 `elementFromPoint` 로 `data-tabs-pane`(탭 바, `data-tab-key` 앞) / `data-main-pane`(본문 가장자리 22%·가운데)을 찾는다. pointerup 은 놓기, pointercancel·창 blur·Esc 는 취소이며 어느 경우든 `endDrag` 가 불린다.
- 끄는 동안 각 분할 본문(머리글 포함)에 `DropZones` 가 덮인다. 양쪽 22% 는 `splitWith(key, side)`, 가운데는 다른 분할이면 `moveTab`. 분할이 하나이고 탭도 하나면 가장자리 영역을 그리지 않는다.
- 탭 바에 놓으면 가리킨 탭 앞(파란 세로선), 빈 곳이면 맨 뒤.
- 분할 닫기(탭 바 끝 ×)와 우클릭 "이 분할의 탭 모두 닫기" 는 같은 `closePane`. 저장 안 한 편집은 확인한다.
- Ctrl+F·정의 목록·이미지 맞춤은 분할별. 정의 목록은 포커스된 분할에만 뜬다.

## 4. 터미널

- xterm 인스턴스를 `viewmodel/terminalInstances.ts` 에 탭이 닫힐 때까지 둔다. `TerminalView` 는 붙을 때 `term.element` 를 자기 자리로 옮기고(처음이면 `open`) 맞춘 뒤 다시 그린다. 그래서 분할 이동·프로젝트 전환에도 스크롤백과 입력이 남는다. 예전처럼 모든 터미널을 숨겨서 마운트해 둘 필요가 없어졌다.
- 붙여넣기: `attachCustomKeyEventHandler` 로 Ctrl+V / Ctrl+Shift+V / Shift+Insert (macOS ⌘V) 를 잡아 `preventDefault` 하고 백엔드 `terminal_paste`(clipboard-manager 로 읽기)의 글자를 `term.paste()` 로 넣는다. 브라우저 paste 이벤트는 쓰지 않으므로 두 번 붙지 않는다.
- 복사: Ctrl+C 는 선택이 있을 때만 `terminal_copy` 로 복사하고 선택을 지운다. 없으면 셸에 `^C`. Ctrl+Shift+C 는 항상 복사.
- 우클릭: 선택이 있으면 복사, 없으면 붙여넣기.

## 5. 인터넷 탭

| 항목 | 구현 |
|---|---|
| 웹뷰 | `tauri` 의 `unstable` 기능으로 `Window::add_child`. 라벨 `web-<uuid>`, 탭 키 `web|<uuid>`, 현재 URL 은 탭의 `path`. 메인 웹뷰와 같은 WebView2 환경이라 쿠키·로그인이 앱 데이터 폴더에 남는다 |
| 명령 | `web_show`(없으면 만들고 있으면 옮겨 보임), `web_set_bounds`, `web_hide`, `web_close`, `web_navigate`, `web_reload`, `web_history(forward)` |
| 이벤트 | `web://changed { id, url?, title?, loading? }` — `on_navigation`, `on_page_load`(Started/Finished), `on_document_title_changed` 에서 메인 창으로만 보낸다 |
| 안전 | `on_navigation` 이 http(s) 밖을 막는다. 새 창(`target=_blank`, `window.open`)은 별도 팝업 창(`web-popup-N`)으로 연다. `window_features` 로 여는 쪽과 같은 WebView2 환경을 써서 쿠키와 `window.opener` 가 이어지므로 OAuth 로그인 팝업이 동작한다(처음엔 같은 웹뷰에서 열었으나 로그인이 끝나지 않아 바꿈). 팝업 창은 capability 대상(`main`)이 아니라 IPC 가 없다. Tauri 2 는 remote origin 의 IPC 를 `remote` capability 가 있어야만 허용하므로(`webview/mod.rs` 의 ACL 검사) **별도 capability 파일 없이** 앱 명령에 닿지 못한다 |
| 배치 | `WebView` 컴포넌트가 본문 `div` 의 `getBoundingClientRect` 를 매 프레임 비교해 바뀔 때만 `web_set_bounds`. 만들기와 옮기기는 Promise 줄로 차례를 지킨다 |
| 숨김 | 탭이 화면에서 내려감(언마운트) · 탭 끄는 중 · Radix 메뉴/다이얼로그(`body` 직속 포털을 `MutationObserver` 로 감시, `useOverlayOpen`) · **토스트가 떠 있는 동안**(오른쪽 아래 토스트를 가리지 않게) |

### 요구사항과 달라진 점

- 뒤로/앞으로: Tauri 에 history API 가 없어 페이지에 `history.back()/forward()` 를 eval 한다. 갈 곳이 있는지 알 수 없어 버튼은 늘 활성이다.
- 별도 `capabilities/web.json` 은 만들지 않았다(위 안전 항목).
- 토스트가 떠 있는 동안에도 웹뷰를 숨긴다(R3 의 대응을 위치 대신 숨김으로).

## 6. 세션 대화창 접기

- 세션 `Panel` 을 `collapsible collapsedSize={0}` 로 두고 `sessionHidden` 이 바뀌면 `collapse()/expand()`. 패널을 직접 끌어 접은 것도 `onCollapse/onExpand` 로 같은 값에 반영된다.
- 띠(`SessionToggle`, 12px)는 왼쪽 열의 오른쪽 끝에 있어 대화창이 접혀도 같은 자리에 남는다. 접힌 동안 선택된 세션이 진행 중이거나 답하지 않은 권한 질문(`useSessionVM.pending`)이 있으면 빨간 점.
- 세션 행 클릭, "Session 열기", 새 세션 시작·재개는 `showSession(root)` 로 그 프로젝트의 대화창을 편다. 프로젝트 행 클릭은 저장값을 따른다.
- 접힌 동안 대화창 ↔ 뷰어 리사이즈 핸들은 `display: none`.

## 7. 확인한 것 / 못 한 것

- `cargo test` 131개(레이아웃 저장·복원·빈 분할 정리·1.0.x 마이그레이션 포함), `tsc --noEmit`, 릴리스 빌드 통과.
- GUI 조작(끌기·웹뷰 겹침·붙여넣기)은 이 환경에서 화면을 띄워 확인하지 못했다. 수동 검증은 요구사항 문서 7장 시나리오 S1~S8 로 한다. macOS 의 자식 웹뷰 동작은 CI 빌드 후 확인이 필요하다.

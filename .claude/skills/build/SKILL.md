---
name: build
description: claude_viewer 를 빌드·테스트하는 방법. 단일 exe 릴리스 빌드, Rust 테스트(entity TS 타입 생성 포함), 프론트 타입 체크, 개발 모드 실행.
---

# claude_viewer 빌드

## 준비물

- Node.js + npm (`npm install` 로 의존성 설치)
- Rust stable (MSVC 툴체인). 이 머신에서는 scoop `rustup-msvc` 로 설치되어 있다.
  - Bash 에서 cargo 가 PATH 에 없으면 먼저:
    ```bash
    export PATH="$HOME/scoop/apps/rustup-msvc/current/.cargo/bin:$PATH" \
      CARGO_HOME="$HOME/scoop/persist/rustup-msvc/.cargo" \
      RUSTUP_HOME="$HOME/scoop/persist/rustup-msvc/.rustup"
    ```
- MSVC 링커: Visual Studio 18 Insiders 의 C++ 도구. `vswhere` 기본 조회로는 안 잡히지만 rustc 는 찾는다.
- Tauri CLI 는 `@tauri-apps/cli` (devDependency) 로 `npx tauri` 를 쓴다.

## 명령

| 목적 | 명령 (저장소 루트) |
|---|---|
| Rust 테스트 + `src/generated/entity/*.ts` 재생성 | `cd src-tauri && cargo test` |
| 프론트 타입 체크 | `npx tsc --noEmit` |
| 릴리스 단일 exe | 아래 "릴리스 빌드 절차" 를 따른다 → `src-tauri/target/release/claude_viewer.exe` |
| 개발 모드 (핫 리로드) | `npx tauri dev` |

- `domain/entity` 를 바꿨으면 프론트 빌드 전에 반드시 `cargo test` 로 TS 타입을 다시 만든다.
- 인스톨러는 만들지 않는다 (`bundle.active = false`). exe 하나만 복사해 실행한다.
- 아이콘을 바꾸려면 `src-tauri/icons/source.svg` 를 고치고 `npx tauri icon src-tauri/icons/source.svg -o src-tauri/icons` 후 android/ios/Square*·StoreLogo 산출물은 지운다.

## 릴리스 빌드 절차 (항상 이 순서)

실행 중인 exe 는 Windows 에서 덮어쓰거나 지울 수 없지만 **이름은 바꿀 수 있다.** 그래서 빌드 전에 기존 exe 를 `.old` 로 바꿔 두고 빌드한다. 앱을 끄지 않아도 빌드된다.

```bash
cd src-tauri/target/release
rm -f claude_viewer.exe.old                                  # 지난 .old (실행 중이면 지워지지 않음)
[ -f claude_viewer.exe ] && mv claude_viewer.exe claude_viewer.exe.old
cd ../../.. && npx tauri build --no-bundle
```

- `rm` 이 실패하는 건 지난 `.old` 가 아직 실행 중이라는 뜻이다. 그 앱을 닫고 다시 한다.
- 빌드가 실패하면 `claude_viewer.exe.old` 를 `claude_viewer.exe` 로 되돌린다.
- 새 exe 를 쓰려면 실행 중인 앱을 닫고 새로 연다.

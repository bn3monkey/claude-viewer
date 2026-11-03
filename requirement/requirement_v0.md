# 클로드 뷰어

## 목표

여러 프로젝트에서 사용하는 Claude Session의 진행 상황과 결과를 한눈에 확인하는 단일 binary 세션 만들기

## 요구사항

- GUI는 아래의 요구사항을 만족시킨다.

### Claude 연결 UI

- 맨 위쪽에 Claude 연결 UI가 있다.

  - [경로          ] [자동 찾기] [수동 찾기] UI로 제공한다.
  - 자동 찾기는 VSCode Plugin이 설치된 경로를 1순위로 찾고 2순위는 일반적으로 Claude Code가 깔리는 위치를 검색한다.
  - 수동 찾기는 내가 위치를 지정한다.
  - Claude Session은 다른 앱들이 기성적으로 사용하는 App dependent한 임시 경로에 쓰인다.

### Project & Session UI

- 맨 왼쪽에 Project에 대한 UI가 있다.
  - 위쪽에는 20%의 크기를 차지하는 Project & Session View가 있다.
    - Project Abstract View는 "Project 폴더 이름"과 "완료된 세션 / 전체 세션" 태그가 붙어있다.
    - "완료된 세션 / 전체 세션"은 완료된 세션이 0이면 빨강색, 완료된 세션이 하나라도 생기면 노랑색, 전부 완료되면 초록색이 된다.
    - Project Abstract View의 Label에 "폴더 모양"의 아이콘이 있어서, 클릭하면 File Picker로 폴더를 열 수 있다.
    - Project를 오른쪽 클릭하면, 아래의 기능을 제공한다.
      - Project 닫기
      - 새 활성화 세션
    - 새 활성화 세션을 누르면, 기존에 있는 세션들의 목록이 Dialog로 나온다. 그 list에서 선택하거나, 아예 새 세션을 생성할 수 있다. 세션 이름은 "New Session..."이다가 첫 대화를 시작하면 VScode claude code extension처럼 만들어진 세션 이름으로 바뀐다.
    - Project 아래에 활성화된 Session이 보인다.
      - Session 목록은 Session 이름과 진행 여부(진행 중 : 빨강색 / 완료함 : 초록색)으로 보여진다.
    - Session을 오른쪽 클릭하면, Session을 닫을 수 있다.

### Project Tree UI

- Project Directory Tree View가 있다.
  - 평소에는 비어있다가, 왼쪽에서 Project를 선택하거나 Project 밑에 있는 세션을 선택하면 그 Project에 대한 Directory Tree 구조가 나온다.
    - Directory를 오른쪽 클릭하면 아래의 기능을 제공한다.
      - Directory 삭제
      - Directory 이름 바꾸기
      - Directory 이동 (얘는 사실 드래그하면 가능)
      - Directory 클립보드에 복사
      - 아래에 새 Directory 생성
      - 탐색기로 열기
      - 새 파일 생성하기
      - 절대 경로를 클립보드에 복사
      - 상대 경로를 클립보드에 복사
    - File을 오른쪽 클릭하면 아래의 기능을 제공한다.
      - File 삭제
      - File 이름 바꾸기
      - File 이동 (얘는 사실 드래그하면 가능)
      - File 복사
      - 탐색기로 열기
      - 절대 경로를 클립보드에 복사
      - 상대 경로를 클립보드에 복사
      - Markdown일 경우, 미리 보기로 열기
      - html일 경우, browser로 열기
    - 빈 칸을 오른쪽 클릭하면 아래의 기능 제공한다
      - 새 파일 생성
      - 새 폴더 생성

### Session UI

Session UI는 VS Code의 Claude Code 화면 처럼 보여지되 훨씬 더 간략화된다.

- 대화가 보여지는 Session 화면
- 현재, Project Tree Ui에서 내가 선택한 파일
- 현재 내가 선택한 모델 (Dropdown으로 모델 리스트에서 선택할 수 있음)
- 텍스트 상자
- Commit button. 
- Abort button. (이미 생각 중인 거 interrupt 걸어서 취소하기)

### File Viewer

- 수정 불가한 Viewer로 아래의 파일에 대해서 보여준다.

  - pdf는 pdf rendering
  - docx는 docx rendering
  - 코드 파일이나 .txt는 텍스트 파일. 그러나 수정 불가능. (code 종류에 따라 syntax highlighting 기능은 제공)
  - markdown도 syntax 기능 제공. 미리보기로 열기로 하면 markdown rendering된 상태로 열림 
  - html도 syntax 기능 제공. browser로 열기로 하면 html rendering된 상태로 열림
  - 이미지 파일(.jpeg, .png, .webp, .bmp)도 열림
  - 동영상 파일(mp4만)도 재생 가능
- Project Tree UI에서 선택한 것만 보여짐.
  - 대신, AI와 연결되면 특정 파일을 어떤 형식으로 이 File Viewer에 보여달라고 하면 보여줄 수 있는 API는 제공해줌.


## 제약사항

1. Desktop 환경에서 가능하면 좋겠음

- Windows
- MacOS

2. 단일 .exe 파일로 돌아갔으면 좋겠음.
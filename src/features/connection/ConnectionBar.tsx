import { useEffect, useState } from "react";
import { Settings } from "lucide-react";
import { useConnectionVM } from "../../viewmodel/useConnectionVM";
import { useTerminalSettingsVM } from "../../viewmodel/useTerminalVM";

const SOURCE_LABEL = {
  vsCodeExtension: "VSCode 확장",
  nativeInstall: "네이티브 설치",
  packageManager: "패키지 매니저",
  path: "PATH",
  manual: "수동 지정",
} as const;

/** ① Claude 연결 UI: [경로] [자동 찾기] [수동 찾기] */
export function ConnectionBar() {
  const { binary, checking, findAuto, pickManual, setManual } = useConnectionVM();
  const [path, setPath] = useState("");

  useEffect(() => setPath(binary?.path ?? ""), [binary]);

  return (
    <div className="top">
      <span className="label">Claude 경로</span>
      <input
        value={path}
        spellCheck={false}
        placeholder="claude 실행 파일 경로"
        onChange={(e) => setPath(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && path.trim()) setManual(path.trim());
        }}
      />
      <button onClick={findAuto} disabled={checking}>
        자동 찾기
      </button>
      <button onClick={pickManual} disabled={checking}>
        수동 찾기
      </button>
      <span className="sep" />
      <span className="conn">
        {checking ? (
          <>
            <span className="dot wait" />
            확인 중…
          </>
        ) : binary ? (
          <>
            <span className="dot ok" />
            연결됨 · v{binary.version} · {SOURCE_LABEL[binary.source]} · 세션 데이터: ~/.claude/projects
          </>
        ) : (
          <>
            <span className="dot bad" />
            연결 안 됨
          </>
        )}
      </span>
      <span className="sep" />
      <button className="icon" title="설정 (터미널 경로)" onClick={() => useTerminalSettingsVM.getState().show()}>
        <Settings size={14} /> 설정
      </button>
    </div>
  );
}

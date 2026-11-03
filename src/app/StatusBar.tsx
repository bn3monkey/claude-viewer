import { useConnectionVM } from "../viewmodel/useConnectionVM";
import { useProjectVM } from "../viewmodel/useProjectVM";

export function StatusBar() {
  const binary = useConnectionVM((s) => s.binary);
  const projects = useProjectVM((s) => s.projects);
  const sessions = projects.flatMap((p) => p.sessions);
  const running = sessions.filter((s) => s.state === "running").length;

  return (
    <div className="status">
      <span>{binary ? "● Claude 연결됨" : "○ Claude 미연결"}</span>
      <span>
        활성 세션 {sessions.length} (진행 {running} / 완료 {sessions.length - running})
      </span>
      <span>감시 중: ~/.claude/projects</span>
    </div>
  );
}

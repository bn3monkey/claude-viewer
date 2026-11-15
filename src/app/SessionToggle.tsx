import { ChevronLeft, ChevronRight } from "lucide-react";
import { findSession, useProjectVM } from "../viewmodel/useProjectVM";
import { useSessionVM } from "../viewmodel/useSessionVM";
import { selectLayout, useViewerVM } from "../viewmodel/useViewerVM";

/**
 * 트리 열과 세션 대화창 사이에 늘 떠 있는 좁은 띠. 누르면 현재 프로젝트의 대화창을 접거나 편다.
 * 접힌 동안 선택된 세션이 진행 중이거나 권한 질문이 기다리면 빨간 점을 찍는다.
 */
export function SessionToggle() {
  const hidden = useViewerVM((s) => selectLayout(s).sessionHidden);
  const toggle = useViewerVM((s) => s.toggleSession);
  const session = useProjectVM((s) => findSession(s.projects, s.selection));
  const waiting = useSessionVM((s) => (session ? (s.pending[session.id]?.length ?? 0) > 0 : false));
  const busy = !!session && (session.state === "running" || session.busy || waiting);

  return (
    <button
      className={`session-toggle ${hidden ? "collapsed" : ""}`}
      title={hidden ? "세션 대화창 보이기" : "세션 대화창 숨기기"}
      onClick={toggle}
    >
      {hidden ? <ChevronRight size={11} /> : <ChevronLeft size={11} />}
      {hidden && busy && <span className="toggle-dot" title={waiting ? "응답을 기다리는 질문이 있습니다" : "세션 진행 중"} />}
    </button>
  );
}

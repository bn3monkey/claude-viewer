import type { ChatEvent } from "../../generated/entity/ChatEvent";
import { formatElapsed, formatTokens, STATUS_LABEL, statusOf, useSubagentVM } from "../../viewmodel/useSubagentVM";

/** 서브에이전트를 띄우는 도구 이름. 예전 CLI 는 Task 였다. */
export const isAgentTool = (name: string) => name === "Agent" || name === "Task";

/** 대화 속 Agent 호출. 서브에이전트 기록이 있으면 진행 상황과 "대화 보기" 를 보여 준다. */
export function AgentCard({ event, sessionId }: { event: Extract<ChatEvent, { kind: "toolUse" }>; sessionId: string }) {
  const run = useSubagentVM((s) => s.runs[sessionId]?.find((r) => r.toolUseId === event.id));
  const select = useSubagentVM((s) => s.select);
  if (!run) {
    return (
      <div className="msg tool">
        ▶ {event.name} {event.summary}
      </div>
    );
  }
  const status = statusOf(run);
  return (
    <div className={`msg agent ${status}`}>
      <div className="ahead">
        <span className="astatus">{STATUS_LABEL[status]}</span>
        <b>{run.agentType}</b>
        <span className="adesc" title={run.description}>
          {run.description}
        </span>
        <span className="spacer" />
        <button onClick={() => select(sessionId, run.agentId)}>탭에서 보기</button>
      </div>
      <div className="astats">
        도구 {run.toolUses}회 · 출력 {formatTokens(run.outputTokens)} 토큰 · {formatElapsed(run)}
        {run.model && ` · ${run.model}`}
        {run.background && " · 백그라운드"}
      </div>
      {status !== "done" && run.lastActivity && <div className="alast">↳ {run.lastActivity}</div>}
    </div>
  );
}

import type { ChatEvent } from "../../generated/entity/ChatEvent";
import type { SubagentRun } from "../../generated/entity/SubagentRun";
import { formatElapsed, formatTokens, STATUS_LABEL, statusOf, useSubagentVM } from "../../viewmodel/useSubagentVM";

/** 서브에이전트를 띄우는 도구 이름. 예전 CLI 는 Task 였다. */
export const isAgentTool = (name: string) => name === "Agent" || name === "Task";

const EMPTY: SubagentRun[] = [];

/** 대화 속 Agent 호출. 서브에이전트 기록이 있으면 진행 상황과 "대화 보기" 를 보여 준다. */
export function AgentCard({ event, sessionId }: { event: Extract<ChatEvent, { kind: "toolUse" }>; sessionId: string }) {
  const run = useSubagentVM((s) => s.runs[sessionId]?.find((r) => r.toolUseId === event.id));
  const open = useSubagentVM((s) => s.open);
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
        <button onClick={() => open(sessionId, run.agentId)}>대화 보기</button>
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

/** 대화 위에 붙는 띠. 지금 돌고 있는 서브에이전트만 보여 준다. */
export function SubagentStrip({ sessionId }: { sessionId: string }) {
  const runs = useSubagentVM((s) => s.runs[sessionId]) ?? EMPTY;
  const open = useSubagentVM((s) => s.open);
  const running = runs.filter((r) => r.running);
  if (running.length === 0) return null;
  return (
    <div className="agent-strip">
      <span className="hint">서브에이전트 {running.length}개 실행 중</span>
      {running.map((run) => (
        <button key={run.agentId} title={run.lastActivity ?? run.description} onClick={() => open(sessionId, run.agentId)}>
          <span className="astatus">●</span> {run.agentType} · {run.description}
          {run.lastActivity && <span className="hint"> — {run.lastActivity}</span>}
        </button>
      ))}
    </div>
  );
}

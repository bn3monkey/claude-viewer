import { X } from "lucide-react";
import type { SubagentRun } from "../../generated/entity/SubagentRun";
import { formatElapsed, formatTokens, STATUS_LABEL, statusOf, useSubagentVM } from "../../viewmodel/useSubagentVM";

const EMPTY: SubagentRun[] = [];

/** 사용량 밑의 얇은 탭 줄. Main 과, 이 세션이 띄운 서브에이전트들. */
export function AgentTabs({ sessionId }: { sessionId: string }) {
  const runs = useSubagentVM((s) => s.runs[sessionId]) ?? EMPTY;
  const active = useSubagentVM((s) => s.active[sessionId] ?? null);
  const select = useSubagentVM((s) => s.select);
  return (
    <div className="agent-tabs" role="tablist">
      <button role="tab" aria-selected={active === null} className={`atab${active === null ? " on" : ""}`} onClick={() => select(sessionId, null)}>
        Main
      </button>
      {runs.map((run) => {
        const status = statusOf(run);
        const on = active === run.agentId;
        return (
          <div
            key={run.agentId}
            role="tab"
            aria-selected={on}
            className={`atab ${status}${on ? " on" : ""}`}
            title={`${STATUS_LABEL[status]} · ${run.agentType} · ${run.description}\n도구 ${run.toolUses}회 · 출력 ${formatTokens(run.outputTokens)} 토큰 · ${formatElapsed(run)}${run.lastActivity && status !== "done" ? `\n↳ ${run.lastActivity}` : ""}`}
            onClick={() => select(sessionId, run.agentId)}
          >
            <span className={`dot ${status}`} />
            <span className="atitle">{run.description || run.agentType}</span>
            {/* 서브에이전트를 직접 멈추는 건 위험해서 모양만 둔다. */}
            <span className="aclose" aria-hidden title="서브에이전트는 여기서 닫거나 멈출 수 없습니다">
              <X size={11} />
            </span>
          </div>
        );
      })}
    </div>
  );
}

/** 서브에이전트 탭 맨 위의 한 줄 요약. */
export function SubagentSummary({ run }: { run: SubagentRun }) {
  const status = statusOf(run);
  return (
    <div className="subagent-summary">
      <span className={`astatus ${status}`}>{STATUS_LABEL[status]}</span>
      <b>{run.agentType}</b>
      <span className="astats">
        도구 {run.toolUses}회 · 출력 {formatTokens(run.outputTokens)} 토큰 · {formatElapsed(run)}
        {run.model && ` · ${run.model}`}
        {run.background && " · 백그라운드"}
      </span>
      {status !== "done" && run.lastActivity && <span className="alast">↳ {run.lastActivity}</span>}
    </div>
  );
}

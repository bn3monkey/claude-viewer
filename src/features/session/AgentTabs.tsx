import { X } from "lucide-react";
import type { SubagentRun } from "../../generated/entity/SubagentRun";
import { formatElapsed, formatTokens, STATUS_LABEL, statusOf, useSubagentVM } from "../../viewmodel/useSubagentVM";

const EMPTY: SubagentRun[] = [];
/** 기록이 멈춘 지 이만큼 지난 미완료 에이전트는 죽은 것으로 보고 탭에서 뺀다. */
const STALLED_TTL_MS = 10 * 60_000;

/**
 * 탭에 둘 에이전트. 끝난 것은 빼되, 지금 보고 있는 탭은 다른 탭으로 옮길 때까지 남긴다.
 * 로그 파일은 Claude Code 가 그대로 두므로 이 거르기가 없으면 앱을 다시 켤 때마다 옛 탭이 쌓인다.
 */
function visibleRuns(runs: SubagentRun[], active: string | null) {
  const now = Date.now();
  return runs.filter((run) => {
    if (run.agentId === active) return true;
    const status = statusOf(run);
    return status === "running" || (status === "stalled" && now - run.updatedAt < STALLED_TTL_MS);
  });
}

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
      {visibleRuns(runs, active).map((run) => {
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

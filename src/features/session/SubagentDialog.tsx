import { useLayoutEffect, useMemo, useRef } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import type { ChatEvent } from "../../generated/entity/ChatEvent";
import { formatElapsed, formatTokens, STATUS_LABEL, statusOf, useSubagentVM } from "../../viewmodel/useSubagentVM";
import { LinkBase } from "../viewer/Markdown";
import { ChatItem } from "./ChatItem";

const EMPTY: ChatEvent[] = [];

/** 서브에이전트 하나의 대화. 기록이 늘면 실시간으로 따라간다. */
export function SubagentDialog() {
  const viewing = useSubagentVM((s) => s.viewing);
  const close = useSubagentVM((s) => s.close);
  const run = useSubagentVM((s) => (viewing ? s.runs[viewing.sessionId]?.find((r) => r.agentId === viewing.agentId) : undefined));
  const log = useSubagentVM((s) => (viewing ? s.logs[viewing.agentId] : undefined)) ?? EMPTY;
  const items = useMemo(() => log.filter((e) => e.kind !== "init" && e.kind !== "usage" && e.kind !== "rateLimit"), [log]);

  const scroller = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);
  useLayoutEffect(() => {
    const el = scroller.current;
    if (el && stickToBottom.current) el.scrollTop = el.scrollHeight;
  }, [items.length, viewing?.agentId]);

  const status = run ? statusOf(run) : null;
  return (
    <Dialog.Root
      open={!!viewing}
      onOpenChange={(open) => {
        if (open) return;
        stickToBottom.current = true;
        close();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="overlay" />
        <Dialog.Content className="dialog subagent-dialog" aria-describedby={undefined}>
          <Dialog.Title asChild>
            <h2>
              {status && <span className={`astatus ${status}`}>{STATUS_LABEL[status]} </span>}
              {run ? `${run.agentType} · ${run.description}` : "서브에이전트"}
            </h2>
          </Dialog.Title>
          {run && (
            <div className="astats subagent-stats">
              도구 {run.toolUses}회 · 출력 {formatTokens(run.outputTokens)} 토큰 · {formatElapsed(run)}
              {run.model && ` · ${run.model}`}
              {run.background && " · 백그라운드"}
            </div>
          )}
          <LinkBase.Provider value={viewing?.root ?? ""}>
            <div
              className="chat subagent-chat"
              ref={scroller}
              onScroll={(e) => {
                const el = e.currentTarget;
                stickToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
              }}
            >
              {items.length === 0 && <div className="hint">기록을 읽는 중…</div>}
              {viewing &&
                items.map((event, i) => (
                  <div key={i} className="chat-item">
                    <ChatItem event={event} sessionId={viewing.sessionId} />
                  </div>
                ))}
              {status === "running" && <div className="msg thinking">작업 중 ▍</div>}
            </div>
          </LinkBase.Provider>
          <div className="dfoot">
            <Dialog.Close asChild>
              <button>닫기</button>
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

import { memo, useState } from "react";
import type { ChatEvent } from "../../generated/entity/ChatEvent";
import { useSessionVM } from "../../viewmodel/useSessionVM";
import { Markdown } from "../viewer/Markdown";

const seconds = (ms: number) => `${(ms / 1000).toFixed(1)}s`;

export const ChatItem = memo(function ChatItem({ event, sessionId }: { event: ChatEvent; sessionId: string }) {
  switch (event.kind) {
    case "user":
      return <div className="msg user">{event.text}</div>;
    case "assistantText":
      return (
        <div className="msg assistant md">
          <Markdown text={event.text} />
        </div>
      );
    case "thinking":
      return <div className="msg thinking">{event.text}</div>;
    case "toolUse":
      return (
        <div className="msg tool">
          ▶ {event.name} {event.summary}
        </div>
      );
    case "toolResult":
      // 성공한 결과는 접어 두고, 실패만 펼쳐 보인다.
      return (
        <details className={`msg tool ${event.isError ? "error" : ""}`} open={event.isError}>
          <summary>{event.isError ? "✕ 도구 오류" : "↳ 결과"}</summary>
          {event.text}
        </details>
      );
    case "permissionRequest":
      return <PermissionCard event={event} sessionId={sessionId} />;
    case "question":
      return <QuestionCard event={event} sessionId={sessionId} />;
    case "compacted":
      return (
        <details className="msg meta compacted">
          <summary>⟲ 대화가 압축되었습니다 — 요약 보기</summary>
          <pre>{event.summary}</pre>
        </details>
      );
    case "result":
      return event.isError ? (
        <div className="msg error">✕ {event.text || "응답 실패"}</div>
      ) : (
        <div className="msg meta">
          완료 · {seconds(event.durationMs)} · ${event.costUsd.toFixed(4)}
        </div>
      );
    case "exited":
      if (event.aborted) return <div className="msg meta">■ 중단됨 — 다음 메시지부터 이어서 진행합니다</div>;
      return event.code === 0 ? null : <div className="msg error">프로세스가 종료되었습니다 (code {event.code ?? "?"})</div>;
    case "error":
      return <div className="msg error">{event.message}</div>;
    case "init":
    case "usage":
    case "rateLimit":
      return null;
  }
});

function PermissionCard({ event, sessionId }: { event: Extract<ChatEvent, { kind: "permissionRequest" }>; sessionId: string }) {
  const pending = useSessionVM((s) => s.pending[sessionId]?.includes(event.requestId) ?? false);
  const respond = useSessionVM((s) => s.respondPermission);
  const hasSuggestions = event.suggestions !== "null" && event.suggestions !== "[]" && event.suggestions !== "";
  // plan 모드의 ExitPlanMode 는 계획 승인 요청이다. 입력의 plan 을 펼쳐 보여준다.
  const plan = event.toolName === "ExitPlanMode" ? planOf(event.input) : null;
  return (
    <div className={`msg permission ${pending ? "pending" : ""}`}>
      <div className="ptitle">
        {plan !== null ? (
          <>📋 계획 승인 요청 — 허용하면 plan 모드를 벗어나 실행합니다</>
        ) : (
          <>
            🔒 권한 요청 · <b>{event.toolName}</b> {event.description && <span className="hint">{event.description}</span>}
          </>
        )}
      </div>
      {plan !== null ? (
        <div className="plan md">
          <Markdown text={plan} />
        </div>
      ) : (
        <details>
          <summary>입력 보기</summary>
          <pre>{event.input}</pre>
        </details>
      )}
      {pending ? (
        <div className="actions">
          <button className="primary" onClick={() => respond(sessionId, event.requestId, true, null)}>
            허용
          </button>
          {hasSuggestions && (
            <button onClick={() => respond(sessionId, event.requestId, true, event.suggestions)} title="이 세션에서는 다시 묻지 않음">
              항상 허용
            </button>
          )}
          <button className="danger" onClick={() => respond(sessionId, event.requestId, false, null)}>
            거부
          </button>
        </div>
      ) : (
        <div className="hint">응답함</div>
      )}
    </div>
  );
}

function planOf(input: string): string | null {
  try {
    const plan = JSON.parse(input)?.plan;
    return typeof plan === "string" ? plan : null;
  } catch {
    return null;
  }
}

type QuestionEvent = Extract<ChatEvent, { kind: "question" }>;

/** AskUserQuestion: 질문마다 선택지(단일/복수) + 직접 입력. */
function QuestionCard({ event, sessionId }: { event: QuestionEvent; sessionId: string }) {
  const pending = useSessionVM((s) => s.pending[sessionId]?.includes(event.requestId) ?? false);
  const { answerQuestion, respondPermission } = useSessionVM.getState();
  const [picked, setPicked] = useState<string[][]>(() => event.questions.map(() => []));
  const [other, setOther] = useState<string[]>(() => event.questions.map(() => ""));
  const [sent, setSent] = useState<string[] | null>(null);

  const answerOf = (i: number) => [...picked[i], other[i].trim()].filter(Boolean).join(", ");
  const complete = event.questions.every((_, i) => answerOf(i).length > 0);

  const toggle = (i: number, label: string, multi: boolean) => {
    const current = picked[i];
    const next = multi ? (current.includes(label) ? current.filter((l) => l !== label) : [...current, label]) : [label];
    setPicked(picked.map((p, j) => (j === i ? next : p)));
    // 단일 선택에서 선택지를 고르면 직접 입력은 비운다.
    if (!multi) setOther(other.map((o, j) => (j === i ? "" : o)));
  };

  const submit = async () => {
    const answers = event.questions.map((q, i) => [q.question, answerOf(i)] as [string, string]);
    setSent(answers.map(([, a]) => a));
    await answerQuestion(sessionId, event.requestId, event.input, answers);
  };

  return (
    <div className={`msg permission question ${pending ? "pending" : ""}`}>
      <div className="ptitle">❓ Claude 의 질문</div>
      {event.questions.map((q, i) => (
        <div key={i} className="q">
          <div className="qtext">
            {q.header && <span className="qhead">{q.header}</span>}
            {q.question}
            {q.multiSelect && <span className="hint"> (여러 개 선택 가능)</span>}
          </div>
          {pending ? (
            <>
              <div className="qopts">
                {q.options.map((o) => (
                  <button
                    key={o.label}
                    className={picked[i].includes(o.label) ? "picked" : ""}
                    title={o.description}
                    onClick={() => toggle(i, o.label, q.multiSelect)}
                  >
                    <span className="mark">{q.multiSelect ? (picked[i].includes(o.label) ? "☑" : "☐") : picked[i].includes(o.label) ? "◉" : "○"}</span>
                    <span>
                      <b>{o.label}</b>
                      {o.description && <span className="desc">{o.description}</span>}
                    </span>
                  </button>
                ))}
              </div>
              <input
                className="qother"
                placeholder="직접 입력 (기타)"
                value={other[i]}
                onChange={(e) => {
                  setOther(other.map((o, j) => (j === i ? e.target.value : o)));
                  if (!q.multiSelect && e.target.value) setPicked(picked.map((p, j) => (j === i ? [] : p)));
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && complete) submit();
                }}
              />
            </>
          ) : (
            <div className="hint">→ {sent?.[i] ?? "응답함"}</div>
          )}
        </div>
      ))}
      {pending && (
        <div className="actions">
          <button className="primary" disabled={!complete} onClick={submit}>
            답변 보내기
          </button>
          <button onClick={() => respondPermission(sessionId, event.requestId, false, null)} title="답하지 않고 Claude 가 알아서 진행하게 함">
            건너뛰기
          </button>
        </div>
      )}
    </div>
  );
}

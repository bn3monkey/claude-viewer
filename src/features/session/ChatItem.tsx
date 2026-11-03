import { memo } from "react";
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
        <div className="msg assistant">
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
  return (
    <div className={`msg permission ${pending ? "pending" : ""}`}>
      <div className="ptitle">
        🔒 권한 요청 · <b>{event.toolName}</b> {event.description && <span className="hint">{event.description}</span>}
      </div>
      <details>
        <summary>입력 보기</summary>
        <pre>{event.input}</pre>
      </details>
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

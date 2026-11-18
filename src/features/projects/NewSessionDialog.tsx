import { useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import type { Session } from "../../generated/entity/Session";
import { ClaudeIcon } from "../../app/icons";
import { useProjectVM } from "../../viewmodel/useProjectVM";

const formatDate = (ms: number | null) => (ms ? new Date(ms).toLocaleString() : "");

/** 새 활성화 세션: 기존 세션 중 하나를 고르거나 새 세션을 만든다. */
export function NewSessionDialog({ root, onClose }: { root: string | null; onClose: () => void }) {
  const { projects, listSessions, clearSessions, startSession, resumeSession } = useProjectVM();
  const [sessions, setSessions] = useState<Session[] | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const project = projects.find((p) => p.root === root);
  const active = new Set(project?.sessions.map((s) => s.id));

  useEffect(() => {
    setSessions(null);
    setPicked(null);
    setConfirmClear(false);
    if (root) listSessions(root).then(setSessions);
  }, [root, listSessions]);

  const resume = async () => {
    if (root && picked) await resumeSession(root, picked);
    onClose();
  };

  const clearable = sessions?.filter((s) => !active.has(s.id)).length ?? 0;

  const clear = async () => {
    if (!root) return;
    if (!confirmClear) {
      setConfirmClear(true);
      return;
    }
    setConfirmClear(false);
    setPicked(null);
    await clearSessions(root);
    setSessions(await listSessions(root));
  };

  return (
    <Dialog.Root open={!!root} onOpenChange={(open) => !open && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="overlay" />
        <Dialog.Content className="dialog" aria-describedby={undefined}>
          <Dialog.Title asChild>
            <h2>새 활성화 세션 — {project?.name}</h2>
          </Dialog.Title>
          <div className="dbody">
            <div className="hint" style={{ marginBottom: 6 }}>
              ~/.claude/projects 에서 찾은 기존 세션
            </div>
            {sessions === null && <div className="empty">불러오는 중…</div>}
            {sessions?.length === 0 && <div className="empty">기존 세션이 없습니다.</div>}
            {sessions?.map((s) => {
              const isActive = active.has(s.id);
              return (
                <div
                  key={s.id}
                  className={`row ${picked === s.id ? "selected" : ""}`}
                  style={isActive ? { opacity: 0.5, cursor: "default" } : undefined}
                  onClick={() => !isActive && setPicked(s.id)}
                  onDoubleClick={() => !isActive && root && resumeSession(root, s.id).then(onClose)}
                >
                  <span className="ico claude">
                    <ClaudeIcon />
                  </span>
                  <span className="name">
                    {s.title}
                    <br />
                    <small>
                      {s.id.slice(0, 8)} · {formatDate(s.updatedAt)} · 메시지 {s.messageCount}
                      {isActive && " · 이미 활성화됨"}
                    </small>
                  </span>
                  <span className={`state ${s.state}`} />
                </div>
              );
            })}
          </div>
          <div className="dfoot">
            <button onClick={onClose}>취소</button>
            <button
              disabled={clearable === 0}
              onClick={clear}
              onBlur={() => setConfirmClear(false)}
              title="활성화되지 않은 기존 세션 기록을 모두 삭제합니다"
              style={confirmClear ? { color: "var(--red)" } : undefined}
            >
              {confirmClear ? `${clearable}개 세션 삭제 확인` : "세션 전부 Clear"}
            </button>
            <span className="spacer" />
            <button
              onClick={async () => {
                if (root) await startSession(root);
                onClose();
              }}
            >
              ＋ 새 세션 생성
            </button>
            <button className="primary" disabled={!picked} onClick={resume}>
              선택한 세션 재개
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

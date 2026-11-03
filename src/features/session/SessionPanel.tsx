import { useEffect, useMemo, useRef, useState } from "react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronDown, X } from "lucide-react";
import type { ChatEvent } from "../../generated/entity/ChatEvent";
import { useConnectionVM } from "../../viewmodel/useConnectionVM";
import { findSession, useProjectVM } from "../../viewmodel/useProjectVM";
import { summarizeUsage, useSessionVM } from "../../viewmodel/useSessionVM";
import { useTreeVM } from "../../viewmodel/useTreeVM";
import { ChatItem } from "./ChatItem";
import { UsageBar } from "./UsageBar";

const EMPTY: ChatEvent[] = [];

/** `--permission-mode` 선택지. null 은 CLI 기본값(매번 물어봄). */
const PERMISSION_MODES: { value: string | null; label: string; hint: string }[] = [
  { value: null, label: "default", hint: "도구마다 물어봄" },
  { value: "acceptEdits", label: "acceptEdits", hint: "파일 편집은 자동 허용" },
  { value: "plan", label: "plan", hint: "읽기만, 계획 수립" },
  { value: "bypassPermissions", label: "bypassPermissions", hint: "전부 자동 허용 (주의)" },
];

/** ④ Session: 사용량, 대화, 선택 파일, 모델, 권한, 입력, Commit / Abort */
export function SessionPanel() {
  const { projects, selection } = useProjectVM();
  const session = findSession(projects, selection);
  const project = projects.find((p) => p.root === selection?.root);
  const events = useSessionVM((s) => (session ? s.events[session.id] : undefined)) ?? EMPTY;
  const rateWindows = useSessionVM((s) => s.rateWindows);
  const { ensureLoaded, send, compact, abort } = useSessionVM();
  const selectedFile = useTreeVM((s) => s.selectedFile);
  const treeRoot = useTreeVM((s) => s.root);
  const settings = useConnectionVM((s) => s.settings);
  const [model, setModel] = useState<string | null>(null);
  const [permissionMode, setPermissionMode] = useState<string | null | undefined>(undefined);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [sending, setSending] = useState(false);

  useEffect(() => {
    if (session && project) ensureLoaded(project.root, session.id);
  }, [session?.id, project?.root, ensureLoaded]);

  const currentModel = model ?? settings?.selectedModel ?? settings?.models[0] ?? "";
  const currentPermission = permissionMode === undefined ? (settings?.permissionMode ?? null) : permissionMode;
  const draft = session ? (drafts[session.id] ?? "") : "";
  const attached = selectedFile && treeRoot === project?.root ? selectedFile : null;
  const relative = attached && project ? attached.slice(project.root.length).replace(/^[\\/]/, "").replace(/\\/g, "/") : null;
  // 다른 클라이언트에서 돌고 있는 세션에 끼어들면 대화가 갈라지므로 막는다.
  const runningElsewhere = !!session && session.state === "running" && !session.busy && session.messageCount > 0;
  const idle = !!session && !session.busy && !runningElsewhere && !sending;
  const canCommit = idle && draft.trim().length > 0;
  const usage = useMemo(() => summarizeUsage(events), [events]);

  const commit = async () => {
    if (!session || !project || !canCommit) return;
    setSending(true);
    const ok = await send({
      root: project.root,
      sessionId: session.id,
      text: draft.trim(),
      model: currentModel,
      permissionMode: currentPermission,
      attachedFile: attached,
    });
    setSending(false);
    if (ok) setDrafts({ ...drafts, [session.id]: "" });
  };

  if (!session || !project) {
    return (
      <div className="session-panel">
        <div className="head">
          <span className="crumb">{project ? `${project.name} /` : ""}</span>
        </div>
        <div className="empty" style={{ marginTop: 40 }}>
          {project ? "세션을 선택하거나, 프로젝트를 우클릭해 새 활성화 세션을 만드세요." : "왼쪽에서 프로젝트를 열고 세션을 선택하세요."}
        </div>
      </div>
    );
  }

  return (
    <div className="session-panel">
      <div className="head">
        <span className="crumb">{project.name} /</span>
        <strong title={session.title}>{session.title}</strong>
        <span className={`state ${session.state}`} style={{ marginLeft: 4 }} title={session.state === "running" ? "진행 중" : "완료"} />
        <span className="spacer" />
        <span className="hint">
          session {session.id.slice(0, 4)}…{session.id.slice(-4)} · 메시지 {session.messageCount}
        </span>
      </div>
      <UsageBar
        {...usage}
        rateWindows={rateWindows}
        canCompact={idle && usage.contextTokens > 0}
        onCompact={() => compact({ root: project.root, sessionId: session.id, model: currentModel, permissionMode: currentPermission })}
      />
      <PanelGroup direction="vertical" autoSaveId="layout-session">
        <Panel defaultSize={75} minSize={20}>
          <ChatList events={events} busy={session.busy} sessionId={session.id} />
        </Panel>
        <PanelResizeHandle className="resize-v" />
        <Panel defaultSize={25} minSize={12}>
          <div className="compose">
            <div className="ctx">
              <span>선택 파일</span>
              {relative ? (
                <span className="chip" title={attached!}>
                  {relative}
                  <button title="첨부 해제" onClick={() => useTreeVM.getState().selectFile(null)}>
                    <X size={12} />
                  </button>
                </span>
              ) : (
                <span className="hint">없음</span>
              )}
              <span>모델</span>
              <DropdownMenu.Root>
                <DropdownMenu.Trigger asChild>
                  <button className="icon">
                    {currentModel} <ChevronDown size={12} />
                  </button>
                </DropdownMenu.Trigger>
                <DropdownMenu.Portal>
                  <DropdownMenu.Content className="menu" align="start">
                    {settings?.models.map((m) => (
                      <DropdownMenu.Item key={m} className="it" onSelect={() => setModel(m)}>
                        {m}
                        {m === currentModel && <span className="k">✓</span>}
                      </DropdownMenu.Item>
                    ))}
                  </DropdownMenu.Content>
                </DropdownMenu.Portal>
              </DropdownMenu.Root>
              <span>권한</span>
              <DropdownMenu.Root>
                <DropdownMenu.Trigger asChild>
                  <button className="icon" title="--permission-mode. 바꾸면 다음 메시지부터 적용">
                    {PERMISSION_MODES.find((m) => m.value === currentPermission)?.label ?? currentPermission} <ChevronDown size={12} />
                  </button>
                </DropdownMenu.Trigger>
                <DropdownMenu.Portal>
                  <DropdownMenu.Content className="menu" align="start">
                    {PERMISSION_MODES.map((m) => (
                      <DropdownMenu.Item key={m.label} className="it" onSelect={() => setPermissionMode(m.value)}>
                        {m.label}
                        <span className="k">{m.value === currentPermission ? "✓ " : ""}{m.hint}</span>
                      </DropdownMenu.Item>
                    ))}
                  </DropdownMenu.Content>
                </DropdownMenu.Portal>
              </DropdownMenu.Root>
            </div>
            <textarea
              placeholder="메시지를 입력하세요… (Ctrl+Enter 또는 Alt+Enter 로 Commit)"
              value={draft}
              onChange={(e) => setDrafts({ ...drafts, [session.id]: e.target.value })}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey || e.altKey)) {
                  e.preventDefault();
                  commit();
                }
              }}
            />
            <div className="actions">
              <span className="hint">
                {runningElsewhere
                  ? "다른 곳(VSCode, 터미널)에서 진행 중인 세션입니다. 끝난 뒤에 보낼 수 있습니다."
                  : "선택한 파일은 프롬프트에 @경로 로 자동 첨부됩니다"}
              </span>
              <span className="spacer" />
              <button className="danger" disabled={!session.busy} title="진행 중인 응답을 중단" onClick={() => abort(session.id)}>
                ■ Abort
              </button>
              <button className="primary" disabled={!canCommit} onClick={commit}>
                Commit ⏎
              </button>
            </div>
          </div>
        </Panel>
      </PanelGroup>
    </div>
  );
}

function ChatList({ events, busy, sessionId }: { events: ChatEvent[]; busy: boolean; sessionId: string }) {
  const scroller = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);
  const items = useMemo(() => events.filter((e) => e.kind !== "init" && e.kind !== "usage" && e.kind !== "rateLimit"), [events]);
  const pendingCount = useSessionVM((s) => s.pending[sessionId]?.length ?? 0);
  const showSpinner = busy && pendingCount === 0;
  const count = items.length + (showSpinner ? 1 : 0);

  const virtualizer = useVirtualizer({
    count,
    getScrollElement: () => scroller.current,
    estimateSize: () => 48,
    overscan: 8,
  });

  // 맨 아래를 보고 있을 때만 새 메시지를 따라 내려간다.
  useEffect(() => {
    if (stickToBottom.current && count > 0) virtualizer.scrollToIndex(count - 1, { align: "end" });
  }, [count, virtualizer]);

  return (
    <div
      className="chat"
      ref={scroller}
      onScroll={(e) => {
        const el = e.currentTarget;
        stickToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
      }}
    >
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => (
          <div
            key={row.key}
            data-index={row.index}
            ref={virtualizer.measureElement}
            className="chat-item"
            style={{ position: "absolute", top: 0, left: 0, width: "100%", transform: `translateY(${row.start}px)` }}
          >
            {row.index < items.length ? (
              <ChatItem event={items[row.index]} sessionId={sessionId} />
            ) : (
              <div className="msg thinking">응답 생성 중 ▍</div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

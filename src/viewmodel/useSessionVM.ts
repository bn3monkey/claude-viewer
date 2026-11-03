import { create } from "zustand";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ChatEvent } from "../generated/entity/ChatEvent";
import type { Project } from "../generated/entity/Project";
import type { RateWindow } from "../generated/entity/RateWindow";
import { call } from "./invoke";
import { useProjectVM } from "./useProjectVM";

export type SendArgs = {
  root: string;
  sessionId: string;
  text: string;
  model: string;
  permissionMode: string | null;
  attachedFile: string | null;
};

type SessionVM = {
  /** 세션별 대화. 처음 볼 때 로그를 읽고, 이후 실시간 이벤트를 덧붙인다. */
  events: Record<string, ChatEvent[]>;
  /** 이 앱에서 메시지를 보낸 세션. 로그를 다시 읽으면 실시간 이벤트와 겹치므로 다시 읽지 않는다. */
  live: Record<string, boolean>;
  /** 아직 답하지 않은 권한 요청·질문 id. 세션별. */
  pending: Record<string, string[]>;
  /** 가장 최근에 받은 사용량 창 (계정 단위라 세션과 무관). */
  rateWindows: RateWindow[] | null;
  ensureLoaded: (root: string, id: string) => Promise<void>;
  reload: (root: string, id: string) => Promise<void>;
  send: (args: SendArgs) => Promise<boolean>;
  compact: (args: Omit<SendArgs, "text" | "attachedFile">) => Promise<void>;
  respondPermission: (sessionId: string, requestId: string, allow: boolean, alwaysSuggestions: string | null) => Promise<void>;
  /** answers: [질문, 답] 쌍. 건너뛰기는 respondPermission(…, false, null). */
  answerQuestion: (sessionId: string, requestId: string, input: string, answers: [string, string][]) => Promise<void>;
  abort: (sessionId: string) => Promise<void>;
};

export const useSessionVM = create<SessionVM>((set, get) => ({
  events: {},
  live: {},
  pending: {},
  rateWindows: null,

  ensureLoaded: async (root, id) => {
    if (!get().events[id]) await get().reload(root, id);
  },

  reload: async (root, id) => {
    const log = await call<ChatEvent[]>("session_read_log", { root, id });
    if (log && !get().live[id]) set({ events: { ...get().events, [id]: log } });
  },

  send: async (args) => {
    set({ live: { ...get().live, [args.sessionId]: true } });
    const ok = await call("session_send_message", args);
    return ok !== undefined;
  },

  compact: async (args) => {
    set({ live: { ...get().live, [args.sessionId]: true } });
    await call("session_compact", args);
  },

  respondPermission: async (sessionId, requestId, allow, alwaysSuggestions) => {
    const ok = await call("session_respond_permission", { sessionId, requestId, allow, alwaysSuggestions });
    if (ok !== undefined) setPending(sessionId, (ids) => ids.filter((id) => id !== requestId));
  },

  answerQuestion: async (sessionId, requestId, input, answers) => {
    const ok = await call("session_answer_question", { sessionId, requestId, input, answers });
    if (ok !== undefined) setPending(sessionId, (ids) => ids.filter((id) => id !== requestId));
  },

  abort: async (sessionId) => void (await call("session_abort", { sessionId })),
}));

function setPending(sessionId: string, update: (ids: string[]) => string[]) {
  const { pending } = useSessionVM.getState();
  useSessionVM.setState({ pending: { ...pending, [sessionId]: update(pending[sessionId] ?? []) } });
}

function append(id: string, event: ChatEvent) {
  if (event.kind === "rateLimit") {
    useSessionVM.setState({ rateWindows: event.windows });
    return;
  }
  if (event.kind === "permissionRequest" || event.kind === "question") setPending(id, (ids) => [...ids, event.requestId]);
  // 프로세스가 끝나면 답할 수 없으므로 남은 요청을 지운다.
  if (event.kind === "exited" || event.kind === "result") setPending(id, () => []);

  const { events } = useSessionVM.getState();
  // 아직 열어 보지 않은 세션은 나중에 로그로 한꺼번에 읽는다.
  if (!events[id]) return;
  useSessionVM.setState({ events: { ...events, [id]: [...events[id], event] } });
}

/** 활성 세션마다 session://event/{id} 를 구독하고, 다른 곳에서 바뀐 세션은 로그를 다시 읽는다. */
export function startSessionEvents() {
  const listeners = new Map<string, Promise<UnlistenFn>>();

  const sync = (projects: Project[], previous: Project[]) => {
    const active = new Set(projects.flatMap((p) => p.sessions.map((s) => s.id)));
    for (const id of active) {
      if (!listeners.has(id)) {
        listeners.set(id, listen<ChatEvent>(`session://event/${id}`, ({ payload }) => append(id, payload)));
      }
    }
    for (const [id, unlisten] of listeners) {
      if (!active.has(id)) {
        unlisten.then((f) => f());
        listeners.delete(id);
      }
    }
    const { events, live, reload } = useSessionVM.getState();
    for (const project of projects) {
      const before = previous.find((p) => p.root === project.root);
      for (const session of project.sessions) {
        const old = before?.sessions.find((s) => s.id === session.id);
        if (events[session.id] && !live[session.id] && old && old.updatedAt !== session.updatedAt) {
          reload(project.root, session.id);
        }
      }
    }
  };

  sync(useProjectVM.getState().projects, []);
  const unsubscribe = useProjectVM.subscribe((state, prev) => {
    if (state.projects !== prev.projects) sync(state.projects, prev.projects);
  });
  return () => {
    unsubscribe();
    for (const unlisten of listeners.values()) unlisten.then((f) => f());
  };
}

/** 세션 이벤트에서 사용량 요약을 뽑는다. */
export function summarizeUsage(events: ChatEvent[]) {
  let contextTokens = 0;
  let contextWindow = 0;
  let costUsd = 0;
  let outputTokens = 0;
  for (const e of events) {
    if (e.kind === "usage") {
      contextTokens = e.inputTokens + e.cacheReadTokens + e.cacheCreationTokens;
      outputTokens += e.outputTokens;
    } else if (e.kind === "result") {
      costUsd += e.costUsd;
      if (e.contextWindow > 0) contextWindow = e.contextWindow;
    } else if (e.kind === "compacted") {
      contextTokens = 0;
    }
  }
  return { contextTokens, contextWindow: contextWindow || 200_000, costUsd, outputTokens };
}

import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import type { ChatEvent } from "../generated/entity/ChatEvent";
import type { SubagentRun } from "../generated/entity/SubagentRun";
import { call } from "./invoke";

type SubagentVM = {
  /** 세션별 서브에이전트. 세션을 열 때 읽고, 로그가 바뀌면 다시 읽는다. */
  runs: Record<string, SubagentRun[]>;
  /** 세션 id → 프로젝트 루트. 다시 읽을 때 쓴다. */
  roots: Record<string, string>;
  /** 에이전트 id → 대화. 탭을 연 것만. */
  logs: Record<string, ChatEvent[]>;
  /** 세션 id → 보고 있는 서브에이전트 탭. 없으면 Main. */
  active: Record<string, string>;
  load: (root: string, sessionId: string) => Promise<void>;
  /** 탭을 바꾼다. null 이면 Main. */
  select: (sessionId: string, agentId: string | null) => void;
};

export const useSubagentVM = create<SubagentVM>((set, get) => ({
  runs: {},
  roots: {},
  logs: {},
  active: {},

  load: async (root, sessionId) => {
    set({ roots: { ...get().roots, [sessionId]: root } });
    const runs = await call<SubagentRun[]>("session_subagents", { root, id: sessionId });
    if (runs) set({ runs: { ...get().runs, [sessionId]: runs } });
  },

  select: (sessionId, agentId) => {
    const { [sessionId]: _, ...rest } = get().active;
    set({ active: agentId ? { ...rest, [sessionId]: agentId } : rest });
    const root = get().roots[sessionId];
    if (agentId && root) void loadLog(root, sessionId, agentId);
  },
}));

async function loadLog(root: string, sessionId: string, agentId: string) {
  const log = await call<ChatEvent[]>("session_read_subagent_log", { root, id: sessionId, agentId });
  if (log) useSubagentVM.setState({ logs: { ...useSubagentVM.getState().logs, [agentId]: log } });
}

function refresh(sessionId: string) {
  const { roots, runs, active, load } = useSubagentVM.getState();
  const root = roots[sessionId];
  if (!root || !runs[sessionId]) return;
  void load(root, sessionId);
  if (active[sessionId]) void loadLog(root, sessionId, active[sessionId]);
}

/** 로그가 바뀐 세션을 다시 읽는다. 기록이 멈춘 에이전트도 "응답 없음" 으로 바뀌도록 실행 중인 세션은 가끔 다시 읽는다. */
export function startSubagentEvents() {
  const unlisten = listen<string>("subagent://changed", ({ payload }) => refresh(payload));
  const timer = window.setInterval(() => {
    for (const [sessionId, runs] of Object.entries(useSubagentVM.getState().runs)) {
      if (runs.some((r) => r.running)) refresh(sessionId);
    }
  }, 15_000);
  return () => {
    unlisten.then((f) => f());
    window.clearInterval(timer);
  };
}

export type SubagentStatus = "running" | "done" | "stalled";

export const statusOf = (run: SubagentRun): SubagentStatus => (run.running ? "running" : run.finished ? "done" : "stalled");

export const STATUS_LABEL: Record<SubagentStatus, string> = {
  running: "● 실행 중",
  done: "✓ 완료",
  stalled: "⏸ 응답 없음",
};

export const formatTokens = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : `${n}`);

export const formatElapsed = (run: SubagentRun) => {
  const s = Math.max(0, Math.round((run.updatedAt - run.startedAt) / 1000));
  return s >= 60 ? `${Math.floor(s / 60)}분 ${s % 60}초` : `${s}초`;
};

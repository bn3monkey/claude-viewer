import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Project } from "../generated/entity/Project";
import type { Session } from "../generated/entity/Session";
import { call } from "./invoke";

export type Selection = { root: string; sessionId: string | null };

type ProjectVM = {
  projects: Project[];
  selection: Selection | null;
  collapsed: Record<string, boolean>;
  load: () => Promise<void>;
  pickAndOpen: () => Promise<void>;
  close: (root: string) => Promise<void>;
  select: (selection: Selection | null) => void;
  toggle: (root: string) => void;
  listSessions: (root: string) => Promise<Session[]>;
  startSession: (root: string) => Promise<void>;
  resumeSession: (root: string, id: string) => Promise<void>;
  closeSession: (root: string, id: string) => Promise<void>;
};

export const useProjectVM = create<ProjectVM>((set, get) => ({
  projects: [],
  selection: null,
  collapsed: {},

  load: async () => {
    const projects = await call<Project[]>("project_list");
    if (projects) set({ projects });
  },

  pickAndOpen: async () => {
    const path = await open({ directory: true, multiple: false, title: "프로젝트 폴더 선택" });
    if (typeof path !== "string") return;
    const project = await call<Project>("project_open", { path });
    if (!project) return;
    upsert(project);
    set({ selection: { root: project.root, sessionId: null } });
  },

  close: async (root) => {
    if ((await call("project_close", { root })) === undefined) return;
    const { projects, selection } = get();
    set({
      projects: projects.filter((p) => p.root !== root),
      selection: selection?.root === root ? null : selection,
    });
  },

  select: (selection) => set({ selection }),

  toggle: (root) => set({ collapsed: { ...get().collapsed, [root]: !get().collapsed[root] } }),

  listSessions: async (root) => (await call<Session[]>("session_list", { root })) ?? [],

  startSession: async (root) => {
    const id = await call<string>("session_start", { root });
    if (id) set({ selection: { root, sessionId: id } });
  },

  resumeSession: async (root, id) => {
    if ((await call("session_resume", { root, id })) === undefined) return;
    set({ selection: { root, sessionId: id } });
  },

  closeSession: async (root, id) => {
    if ((await call("session_close", { root, id })) === undefined) return;
    const { selection } = get();
    if (selection?.sessionId === id) set({ selection: { root, sessionId: null } });
  },
}));

function upsert(project: Project) {
  const { projects } = useProjectVM.getState();
  const exists = projects.some((p) => p.root === project.root);
  useProjectVM.setState({
    projects: exists ? projects.map((p) => (p.root === project.root ? project : p)) : [...projects, project],
  });
}

/** 백엔드가 보낸 project://changed 를 반영한다. 닫힌 프로젝트는 다시 넣지 않는다. */
export function startProjectEvents() {
  const unlisten = listen<Project>("project://changed", ({ payload }) => {
    if (useProjectVM.getState().projects.some((p) => p.root === payload.root)) upsert(payload);
  });
  // 다른 클라이언트의 세션은 쓰기가 멈춘 뒤 일정 시간이 지나야 완료로 바뀌므로 주기적으로 다시 계산한다.
  const timer = setInterval(() => useProjectVM.getState().load(), 15000);
  return () => {
    unlisten.then((f) => f());
    clearInterval(timer);
  };
}

export function findSession(projects: Project[], selection: Selection | null) {
  if (!selection?.sessionId) return null;
  const project = projects.find((p) => p.root === selection.root);
  return project?.sessions.find((s) => s.id === selection.sessionId) ?? null;
}

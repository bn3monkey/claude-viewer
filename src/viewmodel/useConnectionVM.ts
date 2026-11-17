import { create } from "zustand";
import { open } from "@tauri-apps/plugin-dialog";
import type { ClaudeBinary } from "../generated/entity/ClaudeBinary";
import type { Settings } from "../generated/entity/Settings";
import { call } from "./invoke";

type ConnectionVM = {
  binary: ClaudeBinary | null;
  checking: boolean;
  settings: Settings | null;
  init: () => Promise<void>;
  findAuto: () => Promise<void>;
  pickManual: () => Promise<void>;
  setManual: (path: string) => Promise<void>;
  /** 보낸 뒤 백엔드가 저장한 세션·프로젝트별 모델을 로컬 사본에도 반영한다. */
  rememberModel: (root: string, sessionId: string, model: string) => void;
};

/** 세션 기록 → 프로젝트 기록 → 기본 모델 순으로 고른다. */
export function modelFor(settings: Settings | null, root: string | undefined, sessionId: string | undefined): string {
  if (!settings) return "";
  return (
    (sessionId ? settings.sessionModels[sessionId] : undefined) ??
    (root !== undefined ? settings.projectModels[root] : undefined) ??
    settings.selectedModel ??
    settings.models[0] ??
    ""
  );
}

export const useConnectionVM = create<ConnectionVM>((set, get) => ({
  binary: null,
  checking: false,
  settings: null,

  init: async () => {
    set({ checking: true });
    const [binary, settings] = await Promise.all([
      call<ClaudeBinary | null>("connection_get"),
      call<Settings>("settings_get"),
    ]);
    set({ binary: binary ?? null, settings: settings ?? null, checking: false });
    // 저장된 경로가 없거나 깨졌으면 한 번 자동으로 찾아 본다.
    if (!binary) await get().findAuto();
  },

  findAuto: async () => {
    set({ checking: true });
    const binary = await call<ClaudeBinary>("connection_find_auto");
    set({ binary: binary ?? get().binary, checking: false });
  },

  pickManual: async () => {
    const path = await open({ multiple: false, directory: false, title: "claude 실행 파일 선택" });
    if (typeof path === "string") await get().setManual(path);
  },

  setManual: async (path) => {
    set({ checking: true });
    const binary = await call<ClaudeBinary>("connection_set_manual", { path });
    set({ binary: binary ?? get().binary, checking: false });
  },

  rememberModel: (root, sessionId, model) => {
    const settings = get().settings;
    if (!settings) return;
    set({
      settings: {
        ...settings,
        sessionModels: { ...settings.sessionModels, [sessionId]: model },
        projectModels: { ...settings.projectModels, [root]: model },
      },
    });
  },
}));

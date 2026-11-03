import { create } from "zustand";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TerminalKind } from "../generated/entity/TerminalKind";
import type { TerminalPaths } from "../generated/entity/TerminalPaths";
import type { TerminalSession } from "../generated/entity/TerminalSession";
import { call } from "./invoke";
import { useTreeVM } from "./useTreeVM";
import { useViewerVM } from "./useViewerVM";

export const IS_WINDOWS = navigator.userAgent.includes("Windows");

export const TERMINAL_MENU: { kind: TerminalKind; label: string; windows: boolean | null }[] = [
  { kind: "cmd", label: "cmd로 실행 (Windows)", windows: true },
  { kind: "powerShell", label: "powershell로 실행 (Windows)", windows: true },
  { kind: "gitBash", label: "git bash로 실행", windows: null },
  { kind: "macTerminal", label: "macOS Terminal로 실행 (macOS)", windows: false },
];

/**
 * 터미널 출력은 탭이 그려지기 전에도 도착하므로 여기서 모아 두고,
 * 화면(TerminalView)이 붙으면 쌓인 것부터 흘려보낸다.
 */
type Sink = (data: string) => void;
const buffers = new Map<string, { chunks: string[]; sink: Sink | null; unlisten: Promise<UnlistenFn>[] }>();

function deliver(id: string, data: string) {
  const buffer = buffers.get(id);
  if (!buffer) return;
  if (buffer.sink) buffer.sink(data);
  else buffer.chunks.push(data);
}

export function attachTerminal(id: string, sink: Sink) {
  const buffer = buffers.get(id);
  if (!buffer) return () => {};
  for (const chunk of buffer.chunks.splice(0)) sink(chunk);
  buffer.sink = sink;
  return () => {
    if (buffer.sink === sink) buffer.sink = null;
  };
}

export function disposeTerminal(id: string) {
  const buffer = buffers.get(id);
  buffer?.unlisten.forEach((u) => u.then((f) => f()));
  buffers.delete(id);
}

export async function launchTerminal(path: string, kind: TerminalKind) {
  // 구독을 먼저 마치고 실행해야 시작 직후 출력(ConPTY 의 커서 위치 질의 포함)을 놓치지 않는다.
  const id = crypto.randomUUID();
  const unlisten = [
    listen<string>(`terminal://data/${id}`, ({ payload }) => deliver(id, payload)),
    listen<number | null>(`terminal://exit/${id}`, ({ payload }) =>
      deliver(id, `\r\n\x1b[90m[프로세스 종료${payload === null ? "" : ` · code ${payload}`}]\x1b[0m\r\n`),
    ),
  ];
  buffers.set(id, { chunks: [], sink: null, unlisten });
  await Promise.all(unlisten);
  const session = await call<TerminalSession>("terminal_launch", { id, path, kind, cols: 100, rows: 30 });
  if (!session) {
    disposeTerminal(id);
    return;
  }
  useViewerVM.getState().open({ path, mode: "terminal" }, { terminal: session });
}

/** 더블클릭: .exe 는 바로 실행, 스크립트는 확장자에 맞는 터미널로. */
export async function runDefault(path: string) {
  const kind = await call<TerminalKind | null>("terminal_default_kind", { path });
  if (kind === undefined) return;
  if (kind === null) await useTreeVM.getState().run(path);
  else await launchTerminal(path, kind);
}

type SettingsVM = {
  open: boolean;
  saved: TerminalPaths | null;
  detected: TerminalPaths | null;
  show: () => Promise<void>;
  hide: () => void;
  save: (paths: TerminalPaths) => Promise<boolean>;
};

export const useTerminalSettingsVM = create<SettingsVM>((set) => ({
  open: false,
  saved: null,
  detected: null,
  show: async () => {
    const result = await call<[TerminalPaths, TerminalPaths]>("terminal_paths");
    if (result) set({ open: true, saved: result[0], detected: result[1] });
  },
  hide: () => set({ open: false }),
  save: async (paths) => {
    const ok = (await call("terminal_save_paths", { paths })) !== undefined;
    if (ok) set({ open: false, saved: paths });
    return ok;
  },
}));

import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import type { LinkTarget } from "../generated/entity/LinkTarget";
import type { SavedTab } from "../generated/entity/SavedTab";
import type { TerminalSession } from "../generated/entity/TerminalSession";
import type { ViewerMode } from "../generated/entity/ViewerMode";
import type { ViewerRequest } from "../generated/entity/ViewerRequest";
import type { TextContent } from "../generated/entity/TextContent";
import { call } from "./invoke";

/** diff 탭은 어느 브랜치 기준인지 함께 기억한다. */
export type DiffContext = { root: string; branch: string | null; untracked: boolean };

export type ViewerTab = ViewerRequest & {
  key: string;
  name: string;
  diff?: DiffContext;
  terminal?: TerminalSession;
  /** 링크로 연 줄 (1부터). reveal 이 바뀔 때마다 그 줄로 다시 스크롤한다. */
  line?: number;
  reveal?: number;
};

type Extra = { diff?: DiffContext; terminal?: TerminalSession; line?: number };

type ViewerVM = {
  tabs: ViewerTab[];
  activeKey: string | null;
  /** 왼쪽 클릭: 확장자에 맞는 기본 모드(.md/.html 은 소스)로 연다. */
  openDefault: (path: string) => Promise<void>;
  open: (request: ViewerRequest, extra?: Extra) => void;
  /** 앱 시작 때 지난번 탭을 되살린다. 그 전까지는 탭 목록을 저장하지 않는다. */
  restore: () => Promise<void>;
  /** 탭을 그 자리에서 다른 모드로 바꾼다 (.md / .html 의 소스 ↔ 미리 보기). */
  switchMode: (key: string, mode: ViewerMode) => void;
  activate: (key: string) => void;
  close: (key: string) => void;
  closeOthers: (key: string) => void;
  closeAll: () => void;
  closeUnder: (root: string) => void;
};

const TERMINAL_LABEL = { cmd: "cmd", powerShell: "PowerShell", gitBash: "git bash", macTerminal: "Terminal" } as const;

const tabKey = (r: ViewerRequest, extra?: Extra) => {
  if (extra?.terminal) return `terminal|${extra.terminal.id}`;
  return `${r.mode}|${extra?.diff ? `${extra.diff.branch ?? ""}|` : ""}${r.path}`;
};
const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;

function makeTab(request: ViewerRequest, extra?: Extra): ViewerTab {
  const name = extra?.terminal ? `${baseName(request.path)} · ${TERMINAL_LABEL[extra.terminal.kind]}` : baseName(request.path);
  return { ...request, key: tabKey(request, extra), name, ...extra };
}

/** 복원이 끝나기 전에 저장하면 지난번 탭 목록을 빈 목록으로 덮어쓴다. */
let restored = false;

/** 닫히는 터미널 탭은 프로세스도 끝낸다. */
function closeTerminals(tabs: ViewerTab[]) {
  for (const tab of tabs) if (tab.terminal) void call("terminal_close", { id: tab.terminal.id });
}

export const useViewerVM = create<ViewerVM>((set, get) => ({
  tabs: [],
  activeKey: null,

  openDefault: async (path) => {
    const mode = await call<ViewerMode>("viewer_resolve_mode", { path });
    if (mode) get().open({ path, mode });
  },

  // 탭은 (경로, 모드) 쌍으로 구분하므로 같은 .md 의 소스 탭과 미리보기 탭이 따로 열린다.
  open: (request, extra) => {
    const key = tabKey(request, extra);
    const { tabs } = get();
    const reveal = extra?.line ? { line: extra.line, reveal: Date.now() } : {};
    if (!tabs.some((t) => t.key === key)) {
      set({ tabs: [...tabs, { ...makeTab(request, extra), ...reveal }] });
    } else if (extra?.line) {
      set({ tabs: tabs.map((t) => (t.key === key ? { ...t, ...reveal } : t)) });
    }
    set({ activeKey: key });
  },

  switchMode: (key, mode) => {
    const { tabs, activeKey } = get();
    const tab = tabs.find((t) => t.key === key);
    if (!tab || tab.mode === mode) return;
    const nextKey = tabKey({ path: tab.path, mode });
    // 같은 파일의 그 모드 탭이 따로 열려 있으면 합친다.
    const next = tabs
      .filter((t) => t.key !== nextKey)
      .map((t) => (t.key === key ? { ...t, mode, key: nextKey, line: undefined, reveal: undefined } : t));
    set({ tabs: next, activeKey: activeKey === key ? nextKey : activeKey });
  },

  restore: async () => {
    const saved = await call<[SavedTab[], number | null]>("viewer_restore_tabs");
    if (saved) {
      const [list, active] = saved;
      const revived = list.map((t) => makeTab({ path: t.path, mode: t.mode }, t.diff ? { diff: t.diff } : undefined));
      // 불러오는 사이에 AI 나 사용자가 연 탭은 뒤에 붙인다.
      const opened = get().tabs.filter((t) => !revived.some((r) => r.key === t.key));
      set({
        tabs: [...revived, ...opened],
        activeKey: get().activeKey ?? (active === null ? null : (revived[active]?.key ?? null)),
      });
    }
    restored = true;
  },

  activate: (activeKey) => set({ activeKey }),

  close: (key) => {
    const { tabs, activeKey } = get();
    const index = tabs.findIndex((t) => t.key === key);
    closeTerminals(tabs.filter((t) => t.key === key));
    const next = tabs.filter((t) => t.key !== key);
    const neighbour = next[Math.min(index, next.length - 1)];
    set({ tabs: next, activeKey: activeKey === key ? (neighbour?.key ?? null) : activeKey });
  },

  closeOthers: (key) => {
    closeTerminals(get().tabs.filter((t) => t.key !== key));
    set({ tabs: get().tabs.filter((t) => t.key === key), activeKey: key });
  },

  closeAll: () => {
    closeTerminals(get().tabs);
    set({ tabs: [], activeKey: null });
  },

  closeUnder: (root) => {
    const { tabs, activeKey } = get();
    const under = (t: ViewerTab) => t.path.startsWith(root) || t.diff?.root === root;
    closeTerminals(tabs.filter(under));
    const next = tabs.filter((t) => !under(t));
    set({ tabs: next, activeKey: next.some((t) => t.key === activeKey) ? activeKey : (next[0]?.key ?? null) });
  },
}));

/** 대화·마크다운 속 링크를 파일로 푼다. 상대 경로는 baseDir 기준. 실패하면 토스트 후 undefined. */
export const resolveLink = (baseDir: string, href: string) => call<LinkTarget>("viewer_resolve_link", { baseDir, href });

export const readText = (path: string) => call<TextContent>("viewer_read_text", { path });
export const readBytes = (path: string) => call<ArrayBuffer>("viewer_read_bytes", { path });

/** AI 가 응답에 넣은 <viewer open=.../> 지시문을 백엔드가 풀어 보내 준다. */
export function startViewerEvents() {
  const unlisten = listen<ViewerRequest>("viewer://open", ({ payload }) => useViewerVM.getState().open(payload));
  void useViewerVM.getState().restore();

  // 탭이 바뀌면 잠시 뒤 저장한다. 터미널 탭은 백엔드가 뺀다.
  let timer: number | undefined;
  const unsubscribe = useViewerVM.subscribe((state, prev) => {
    if (!restored || (state.tabs === prev.tabs && state.activeKey === prev.activeKey)) return;
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      const { tabs, activeKey } = useViewerVM.getState();
      const saved: SavedTab[] = tabs.map((t) => ({ path: t.path, mode: t.mode, diff: t.diff ?? null }));
      const index = tabs.findIndex((t) => t.key === activeKey);
      void call("viewer_save_tabs", { tabs: saved, active: index < 0 ? null : index });
    }, 300);
  });
  return () => {
    unsubscribe();
    window.clearTimeout(timer);
    void unlisten.then((f) => f());
  };
}

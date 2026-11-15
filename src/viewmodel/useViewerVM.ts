import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { askConfirm } from "../app/PromptDialog";
import { useToasts } from "../app/toast";
import type { DefinitionHit } from "../generated/entity/DefinitionHit";
import type { LinkTarget } from "../generated/entity/LinkTarget";
import type { SavedLayout } from "../generated/entity/SavedLayout";
import type { SavedTab } from "../generated/entity/SavedTab";
import type { TerminalSession } from "../generated/entity/TerminalSession";
import type { ViewerMode } from "../generated/entity/ViewerMode";
import type { ViewerRequest } from "../generated/entity/ViewerRequest";
import type { TextContent } from "../generated/entity/TextContent";
import { call } from "./invoke";
import { disposeTerminalInstance } from "./terminalInstances";

/** diff 탭은 어느 브랜치(또는 커밋) 기준인지 함께 기억한다. */
export type DiffContext = { root: string; branch: string | null; untracked: boolean; commit?: string | null };

/** 인터넷 탭. id 는 자식 웹뷰 라벨이고, 페이지 안에서 이동해도 바뀌지 않는다. 현재 URL 은 탭의 path. */
export type WebInfo = { id: string; title: string | null; loading: boolean };

export type ViewerTab = ViewerRequest & {
  key: string;
  name: string;
  diff?: DiffContext;
  terminal?: TerminalSession;
  web?: WebInfo;
  /** 링크로 연 줄 (1부터). reveal 이 바뀔 때마다 그 줄로 다시 스크롤한다. */
  line?: number;
  reveal?: number;
  /** 소스 탭을 편집 중인가. dirty 는 저장하지 않은 변경이 있는가. */
  editing?: boolean;
  dirty?: boolean;
  /** 머리글의 저장 버튼을 누를 때마다 올린다. 편집기가 보고 저장한다. */
  saveNonce?: number;
};

type Extra = { diff?: DiffContext; terminal?: TerminalSession; line?: number; web?: WebInfo };

/** 뷰어 분할 하나: 탭 바 + 본문. */
export type Pane = { id: string; tabs: ViewerTab[]; activeKey: string | null };

/** 프로젝트 하나의 뷰어 화면. panes 는 1~2개이고 0번이 왼쪽. ratio 는 왼쪽 분할 비율(%). */
export type Layout = { panes: Pane[]; focused: string; ratio: number; sessionHidden: boolean };

/**
 * 탭을 어느 분할에 열지. focused: 마지막으로 클릭한 분할(트리·뷰어에서 연 것),
 * left: 왼쪽 분할(세션 대화에서 연 것, 포커스는 그대로), 또는 특정 분할 id.
 */
export type OpenTarget = "focused" | "left" | { pane: string };
type OpenOptions = { target?: OpenTarget; root?: string };

/** F12 결과가 여럿일 때 고르는 목록. */
export type Definitions = { name: string; hits: DefinitionHit[] };

type ViewerVM = {
  /** 프로젝트 루트("" = 프로젝트 없음) → 레이아웃. 없으면 BLANK 로 본다. */
  layouts: Record<string, Layout>;
  current: string;
  /** 끌고 있는 탭. 이 동안 분할 본문 위에 드롭 영역이 뜨고 인터넷 탭은 숨는다. */
  dragging: { pane: string; key: string } | null;
  definitions: Definitions | null;

  /** 왼쪽 클릭: 확장자에 맞는 기본 모드(.md/.html 은 소스)로 연다. */
  openDefault: (path: string, options?: OpenOptions) => Promise<void>;
  open: (request: ViewerRequest, extra?: Extra, options?: OpenOptions) => void;
  /** URL 을 고쳐(스킴 없으면 https://) 인터넷 탭으로 연다. http(s) 가 아니면 거부한다. */
  openWeb: (url: string, pane?: string) => void;
  /** 앱 시작 때 지난번 레이아웃을 되살린다. 그 전까지는 저장하지 않는다. */
  restore: () => Promise<void>;
  /** 탭을 그 자리에서 다른 모드로 바꾼다 (.md / .html 의 소스 ↔ 미리 보기). 저장 안 한 변경은 확인 뒤 버린다. */
  switchMode: (key: string, mode: ViewerMode) => Promise<void>;
  activate: (pane: string, key: string) => void;
  focusPane: (pane: string) => void;
  /** 저장하지 않은 변경이 있으면 확인을 받는다. */
  close: (key: string) => Promise<void>;
  closeOthers: (key: string) => Promise<void>;
  /** 분할의 탭을 모두 닫는다. 분할이 둘이면 그 분할도 사라진다. */
  closePane: (pane: string) => Promise<void>;

  /** 분할이 하나면 그 쪽에 새 분할을 만들어 탭을 옮기고, 둘이면 그 쪽 분할로 옮긴다. */
  splitWith: (key: string, side: "left" | "right") => void;
  /** 다른 분할로 옮기거나 같은 분할에서 순서를 바꾼다. before 가 없으면 맨 뒤. */
  moveTab: (key: string, toPane: string, before?: string | null) => void;
  setRatio: (ratio: number) => void;
  startDrag: (pane: string, key: string) => void;
  endDrag: () => void;

  /** 가운데 세션 대화창 숨김 (현재 프로젝트). */
  toggleSession: () => void;
  setSessionHidden: (hidden: boolean) => void;
  /** 세션을 고르면 그 프로젝트의 대화창을 보인다. */
  showSession: (root: string) => void;

  /** 프로젝트를 고르면 그 프로젝트의 레이아웃으로 바꾼다. */
  switchProject: (root: string) => void;
  /** 프로젝트를 닫으면 그 레이아웃을 버린다. 터미널은 끝내고 인터넷 탭은 없앤다. */
  dropProject: (root: string) => void;
  /** 웹뷰가 알려 준 주소·제목·로딩 상태를 탭에 반영한다. */
  updateWeb: (id: string, patch: { url?: string | null; title?: string | null; loading?: boolean | null }) => void;

  /** 소스 탭 편집 켜기. 끌 때 저장 안 한 변경이 있으면 확인을 받고 버린다. */
  startEditing: (key: string) => void;
  stopEditing: (key: string) => Promise<void>;
  setDirty: (key: string, dirty: boolean) => void;
  requestSave: (key: string) => void;
  /** F12 / Ctrl+클릭: 프로젝트에서 이름의 선언·정의를 찾아 연다. 여럿이면 고르게 한다. */
  goToDefinition: (from: string, name: string, line?: number) => Promise<void>;
  pickDefinition: (hit: DefinitionHit) => void;
  dismissDefinitions: () => void;
};

const TERMINAL_LABEL = { cmd: "cmd", powerShell: "PowerShell", gitBash: "git bash", macTerminal: "Terminal" } as const;

const tabKey = (r: ViewerRequest, extra?: Extra) => {
  if (extra?.terminal) return `terminal|${extra.terminal.id}`;
  if (extra?.web) return `web|${extra.web.id}`;
  const diff = extra?.diff;
  return `${r.mode}|${diff ? `${diff.commit ? `@${diff.commit}` : (diff.branch ?? "")}|` : ""}${r.path}`;
};
const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;
const hostOf = (url: string) => {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
};
const webName = (url: string, title: string | null) => title?.trim() || hostOf(url);

function makeTab(request: ViewerRequest, extra?: Extra): ViewerTab {
  const name = extra?.web
    ? webName(request.path, extra.web.title)
    : extra?.terminal
      ? `${baseName(request.path)} · ${TERMINAL_LABEL[extra.terminal.kind]}`
      : extra?.diff?.commit
        ? `${baseName(request.path)} @${extra.diff.commit.slice(0, 7)}`
        : baseName(request.path);
  return { ...request, key: tabKey(request, extra), name, ...extra };
}

const FIRST_PANE = "p0";
const emptyLayout = (): Layout => ({
  panes: [{ id: FIRST_PANE, tabs: [], activeKey: null }],
  focused: FIRST_PANE,
  ratio: 50,
  sessionHidden: false,
});
/** 아직 레이아웃이 없는 프로젝트가 보는 빈 화면. 셀렉터가 매번 같은 객체를 돌려주도록 하나만 둔다. */
export const BLANK: Layout = emptyLayout();

export const selectLayout = (s: { layouts: Record<string, Layout>; current: string }) => s.layouts[s.current] ?? BLANK;

/** 분할이 둘인데 하나가 비면 없애고, 활성 탭·포커스가 없는 것을 가리키지 않게 맞춘다. */
function normalize(layout: Layout): Layout {
  let panes = layout.panes.map((p) => ({
    ...p,
    activeKey: p.tabs.some((t) => t.key === p.activeKey) ? p.activeKey : (p.tabs[0]?.key ?? null),
  }));
  if (panes.length > 1) {
    const filled = panes.filter((p) => p.tabs.length > 0);
    panes = filled.length > 0 ? filled : [panes[0]];
  }
  const focused = panes.some((p) => p.id === layout.focused) ? layout.focused : panes[0].id;
  return { ...layout, panes, focused };
}

function locate(layout: Layout, key: string) {
  for (const pane of layout.panes) {
    const index = pane.tabs.findIndex((t) => t.key === key);
    if (index >= 0) return { pane, index, tab: pane.tabs[index] };
  }
  return null;
}

/** 탭을 빼고 그 분할의 활성 탭을 이웃으로 옮긴다. */
function withoutTab(pane: Pane, key: string): Pane {
  const index = pane.tabs.findIndex((t) => t.key === key);
  if (index < 0) return pane;
  const tabs = pane.tabs.filter((t) => t.key !== key);
  const activeKey = pane.activeKey === key ? (tabs[Math.min(index, tabs.length - 1)]?.key ?? null) : pane.activeKey;
  return { ...pane, tabs, activeKey };
}

const mapPane = (layout: Layout, id: string, fn: (pane: Pane) => Pane): Layout => ({
  ...layout,
  panes: layout.panes.map((p) => (p.id === id ? fn(p) : p)),
});

/** 복원이 끝나기 전에 저장하면 지난번 레이아웃을 빈 것으로 덮어쓴다. */
let restored = false;

/** 닫히는 터미널 탭은 프로세스를, 인터넷 탭은 웹뷰를 끝낸다. */
function disposeTabs(tabs: ViewerTab[]) {
  for (const tab of tabs) {
    if (tab.terminal) {
      void call("terminal_close", { id: tab.terminal.id });
      disposeTerminalInstance(tab.terminal.id);
    }
    if (tab.web) void call("web_close", { id: tab.web.id });
  }
}

/** 저장하지 않은 변경이 있는 탭을 버려도 되는지 묻는다. 없으면 바로 true. */
async function confirmDiscard(tabs: ViewerTab[]): Promise<boolean> {
  const dirty = tabs.filter((t) => t.dirty);
  if (dirty.length === 0) return true;
  const label =
    dirty.length === 1
      ? `${dirty[0].name} 에 저장하지 않은 변경이 있습니다. 버릴까요?`
      : `저장하지 않은 변경이 있는 탭이 ${dirty.length}개 있습니다. 모두 버릴까요?`;
  return askConfirm("변경 버리기", label);
}

/** 스킴이 없으면 https:// 를 붙인다. http(s) 가 아니면 null. */
export function normalizeWebUrl(input: string): string | null {
  const text = input.trim();
  if (!text) return null;
  const url = /^[a-z][a-z0-9+.-]*:/i.test(text) ? text : `https://${text}`;
  return /^https?:\/\/[^\s]+$/i.test(url) ? url : null;
}

export const useViewerVM = create<ViewerVM>((set, get) => {
  /** 레이아웃 하나(기본은 현재 프로젝트)를 고친다. 없으면 빈 레이아웃에서 시작한다. */
  const edit = (fn: (layout: Layout) => Layout, root = get().current) => {
    const { layouts } = get();
    set({ layouts: { ...layouts, [root]: normalize(fn(layouts[root] ?? emptyLayout())) } });
  };
  const current = () => selectLayout(get());
  const patchTab = (key: string, patch: Partial<ViewerTab>) =>
    edit((l) => ({ ...l, panes: l.panes.map((p) => ({ ...p, tabs: p.tabs.map((t) => (t.key === key ? { ...t, ...patch } : t)) })) }));

  return {
    layouts: {},
    current: "",
    dragging: null,
    definitions: null,

    openDefault: async (path, options) => {
      const mode = await call<ViewerMode>("viewer_resolve_mode", { path });
      if (mode) get().open({ path, mode }, undefined, options);
    },

    // 탭은 (경로, 모드) 쌍으로 구분하므로 같은 .md 의 소스 탭과 미리보기 탭이 따로 열린다.
    // 한 레이아웃 안에서 키는 하나뿐이다. 이미 열려 있으면 그 분할에서 활성화한다.
    open: (request, extra, options) => {
      const key = tabKey(request, extra);
      const target = options?.target ?? "focused";
      const reveal = extra?.line ? { line: extra.line, reveal: Date.now() } : {};
      edit((l) => {
        const found = locate(l, key);
        if (found) {
          const next = mapPane(l, found.pane.id, (p) => ({
            ...p,
            activeKey: key,
            tabs: extra?.line ? p.tabs.map((t) => (t.key === key ? { ...t, ...reveal } : t)) : p.tabs,
          }));
          return { ...next, focused: target === "left" ? l.focused : found.pane.id };
        }
        const dest =
          target === "left"
            ? l.panes[0]
            : typeof target === "object"
              ? (l.panes.find((p) => p.id === target.pane) ?? l.panes[0])
              : (l.panes.find((p) => p.id === l.focused) ?? l.panes[0]);
        const next = mapPane(l, dest.id, (p) => ({ ...p, tabs: [...p.tabs, { ...makeTab(request, extra), ...reveal }], activeKey: key }));
        return { ...next, focused: target === "left" ? l.focused : dest.id };
      }, options?.root ?? get().current);
    },

    openWeb: (input, pane) => {
      const url = normalizeWebUrl(input);
      if (!url) {
        useToasts.getState().push(`http(s) 주소만 열 수 있습니다: ${input}`);
        return;
      }
      const web: WebInfo = { id: crypto.randomUUID(), title: null, loading: true };
      get().open({ path: url, mode: "web" }, { web }, { target: pane ? { pane } : "focused" });
    },

    switchMode: async (key, mode) => {
      const found = locate(current(), key);
      if (!found || found.tab.mode === mode || !(await confirmDiscard([found.tab]))) return;
      const nextKey = tabKey({ path: found.tab.path, mode });
      edit((l) => {
        // 같은 파일의 그 모드 탭이 따로 열려 있으면 합친다.
        const panes = l.panes.map((p) => {
          const tabs = p.tabs
            .filter((t) => t.key !== nextKey)
            .map((t) =>
              t.key === key
                ? { ...t, mode, key: nextKey, line: undefined, reveal: undefined, editing: false, dirty: false }
                : t,
            );
          return { ...p, tabs, activeKey: p.activeKey === key ? nextKey : p.activeKey };
        });
        return { ...l, panes };
      });
    },

    restore: async () => {
      const saved = await call<Record<string, SavedLayout | undefined>>("viewer_restore_layouts");
      if (saved) {
        const layouts = { ...get().layouts };
        for (const [root, layout] of Object.entries(saved)) {
          if (!layout) continue;
          const panes: Pane[] = layout.panes.map((p, i) => {
            const tabs = p.tabs.map(revive);
            return { id: i === 0 ? FIRST_PANE : crypto.randomUUID(), tabs, activeKey: p.active === null ? null : (tabs[p.active]?.key ?? null) };
          });
          // 불러오는 사이에 AI 나 사용자가 연 탭은 왼쪽 분할 뒤에 붙인다.
          const early = layouts[root];
          if (early) {
            const known = new Set(panes.flatMap((p) => p.tabs.map((t) => t.key)));
            const extra = early.panes.flatMap((p) => p.tabs).filter((t) => !known.has(t.key));
            panes[0] = { ...panes[0], tabs: [...panes[0].tabs, ...extra], activeKey: extra.at(-1)?.key ?? panes[0].activeKey };
          }
          layouts[root] = normalize({
            panes,
            focused: panes[layout.focused]?.id ?? FIRST_PANE,
            ratio: layout.ratio,
            sessionHidden: layout.sessionHidden,
          });
        }
        set({ layouts });
      }
      restored = true;
    },

    activate: (pane, key) => edit((l) => ({ ...mapPane(l, pane, (p) => ({ ...p, activeKey: key })), focused: pane })),

    focusPane: (pane) => {
      if (current().focused !== pane) edit((l) => ({ ...l, focused: pane }));
    },

    close: async (key) => {
      const found = locate(current(), key);
      if (!found || !(await confirmDiscard([found.tab]))) return;
      disposeTabs([found.tab]);
      edit((l) => ({ ...l, panes: l.panes.map((p) => withoutTab(p, key)) }));
    },

    closeOthers: async (key) => {
      const found = locate(current(), key);
      if (!found) return;
      const others = found.pane.tabs.filter((t) => t.key !== key);
      if (!(await confirmDiscard(others))) return;
      disposeTabs(others);
      edit((l) => mapPane(l, found.pane.id, (p) => ({ ...p, tabs: p.tabs.filter((t) => t.key === key), activeKey: key })));
    },

    closePane: async (pane) => {
      const tabs = current().panes.find((p) => p.id === pane)?.tabs ?? [];
      if (!(await confirmDiscard(tabs))) return;
      disposeTabs(tabs);
      const keys = new Set(tabs.map((t) => t.key));
      edit((l) => mapPane(l, pane, (p) => ({ ...p, tabs: p.tabs.filter((t) => !keys.has(t.key)), activeKey: null })));
    },

    splitWith: (key, side) => {
      const l = current();
      const found = locate(l, key);
      if (!found) return;
      if (l.panes.length > 1) {
        get().moveTab(key, l.panes[side === "left" ? 0 : 1].id);
        return;
      }
      // 탭이 하나뿐이면 나눌 것이 없다 (같은 탭을 두 분할에 둘 수 없다).
      if (found.pane.tabs.length < 2) return;
      const created: Pane = { id: crypto.randomUUID(), tabs: [found.tab], activeKey: key };
      edit((layout) => {
        const rest = withoutTab(layout.panes[0], key);
        return { ...layout, panes: side === "left" ? [created, rest] : [rest, created], focused: created.id, ratio: 50 };
      });
    },

    moveTab: (key, toPane, before) => {
      const l = current();
      const found = locate(l, key);
      if (!found || before === key) return;
      edit((layout) => {
        const panes = layout.panes.map((p) => {
          let next = p.id === found.pane.id && p.id !== toPane ? withoutTab(p, key) : p;
          if (p.id === toPane) {
            const tabs = next.tabs.filter((t) => t.key !== key);
            const at = before ? tabs.findIndex((t) => t.key === before) : -1;
            tabs.splice(at < 0 ? tabs.length : at, 0, found.tab);
            next = { ...next, tabs, activeKey: key };
          }
          return next;
        });
        return { ...layout, panes, focused: toPane };
      });
    },

    setRatio: (ratio) => {
      const rounded = Math.round(ratio * 10) / 10;
      if (current().panes.length > 1 && Math.abs(current().ratio - rounded) >= 0.1) edit((l) => ({ ...l, ratio: rounded }));
    },

    startDrag: (pane, key) => set({ dragging: { pane, key } }),
    endDrag: () => set({ dragging: null }),

    toggleSession: () => edit((l) => ({ ...l, sessionHidden: !l.sessionHidden })),
    setSessionHidden: (hidden) => {
      if (current().sessionHidden !== hidden) edit((l) => ({ ...l, sessionHidden: hidden }));
    },
    showSession: (root) => {
      if ((get().layouts[root] ?? BLANK).sessionHidden) edit((l) => ({ ...l, sessionHidden: false }), root);
    },

    switchProject: (root) => {
      if (get().current !== root) set({ current: root, dragging: null, definitions: null });
    },

    dropProject: (root) => {
      const layout = get().layouts[root];
      if (!layout) return;
      disposeTabs(layout.panes.flatMap((p) => p.tabs));
      const { [root]: _, ...rest } = get().layouts;
      set({ layouts: rest });
    },

    updateWeb: (id, patch) => {
      const { layouts } = get();
      const key = `web|${id}`;
      let changed = false;
      const next: Record<string, Layout> = {};
      for (const [root, layout] of Object.entries(layouts)) {
        const found = locate(layout, key);
        if (!found) {
          next[root] = layout;
          continue;
        }
        changed = true;
        const web = found.tab.web!;
        const path = patch.url ?? found.tab.path;
        const title = patch.title !== undefined && patch.title !== null ? patch.title : patch.url && patch.url !== found.tab.path ? null : web.title;
        const loading = patch.loading ?? web.loading;
        const tab: ViewerTab = { ...found.tab, path, name: webName(path, title), web: { ...web, title, loading } };
        next[root] = mapPane(layout, found.pane.id, (p) => ({ ...p, tabs: p.tabs.map((t) => (t.key === key ? tab : t)) }));
      }
      if (changed) set({ layouts: next });
    },

    startEditing: (key) => patchTab(key, { editing: true }),

    stopEditing: async (key) => {
      const found = locate(current(), key);
      if (!found || !(await confirmDiscard([found.tab]))) return;
      patchTab(key, { editing: false, dirty: false });
    },

    setDirty: (key, dirty) => {
      const found = locate(current(), key);
      if (found && !!found.tab.dirty !== dirty) patchTab(key, { dirty });
    },

    requestSave: (key) => {
      const found = locate(current(), key);
      if (found) patchTab(key, { saveNonce: (found.tab.saveNonce ?? 0) + 1 });
    },

    goToDefinition: async (from, name, line) => {
      const found = await call<DefinitionHit[]>("viewer_find_definitions", { from, name });
      if (!found) return;
      // 커서가 이미 정의 위에 있으면 그 줄은 뺀다.
      const hits = found.filter((h) => !(h.line === line && samePath(h.path, from)));
      if (hits.length === 0) {
        useToasts.getState().push(`'${name}' 의 정의를 찾지 못했습니다`, "info");
      } else if (hits.length === 1) {
        get().pickDefinition(hits[0]);
      } else {
        set({ definitions: { name, hits } });
      }
    },

    pickDefinition: (hit) => {
      set({ definitions: null });
      get().open({ path: hit.path, mode: "source" }, { line: hit.line });
    },

    dismissDefinitions: () => set({ definitions: null }),
  };
});

function revive(t: SavedTab): ViewerTab {
  if (t.mode === "web") return makeTab({ path: t.path, mode: "web" }, { web: { id: crypto.randomUUID(), title: null, loading: false } });
  return makeTab({ path: t.path, mode: t.mode }, t.diff ? { diff: t.diff } : undefined);
}

function toSaved(layout: Layout): SavedLayout {
  return {
    panes: layout.panes.map((p) => {
      const tabs: SavedTab[] = p.tabs.map((t) => ({
        path: t.path,
        mode: t.mode,
        diff: t.diff ? { ...t.diff, commit: t.diff.commit ?? null } : null,
      }));
      const index = p.tabs.findIndex((t) => t.key === p.activeKey);
      return { tabs, active: index < 0 ? null : index };
    }),
    focused: Math.max(0, layout.panes.findIndex((p) => p.id === layout.focused)),
    ratio: layout.ratio,
    sessionHidden: layout.sessionHidden,
  };
}

const samePath = (a: string, b: string) => a.replace(/\\/g, "/").toLowerCase() === b.replace(/\\/g, "/").toLowerCase();

/** 대화·마크다운 속 링크를 파일로 푼다. 상대 경로는 baseDir 기준. 실패하면 토스트 후 undefined. */
export const resolveLink = (baseDir: string, href: string) => call<LinkTarget>("viewer_resolve_link", { baseDir, href });

export const readText = (path: string) => call<TextContent>("viewer_read_text", { path });
/** 편집한 텍스트를 저장한다. 실패하면 토스트 후 undefined. */
export const writeText = (path: string, text: string) => call<null>("viewer_write_text", { path, text });
export const readBytes = (path: string) => call<ArrayBuffer>("viewer_read_bytes", { path });

type WebChanged = { id: string; url: string | null; title: string | null; loading: boolean | null };

/**
 * AI 가 응답에 넣은 <viewer open=.../> 지시문을 백엔드가 풀어 보내 준다. 세션에서 연 것이므로 왼쪽 분할에,
 * 그 파일이 속한 프로젝트(rootOf)의 레이아웃에 연다.
 */
export function startViewerEvents(rootOf: (path: string) => string | undefined) {
  const unlistenOpen = listen<ViewerRequest>("viewer://open", ({ payload }) =>
    useViewerVM.getState().open(payload, undefined, { target: "left", root: rootOf(payload.path) }),
  );
  const unlistenWeb = listen<WebChanged>("web://changed", ({ payload }) =>
    useViewerVM.getState().updateWeb(payload.id, payload),
  );
  void useViewerVM.getState().restore();

  // 레이아웃이 바뀌면 잠시 뒤 저장한다. 터미널 탭은 백엔드가 뺀다.
  let timer: number | undefined;
  const unsubscribe = useViewerVM.subscribe((state, prev) => {
    if (!restored || state.layouts === prev.layouts) return;
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      const layouts = Object.fromEntries(Object.entries(useViewerVM.getState().layouts).map(([root, l]) => [root, toSaved(l)]));
      void call("viewer_save_layouts", { layouts });
    }, 300);
  });
  return () => {
    unsubscribe();
    window.clearTimeout(timer);
    void unlistenOpen.then((f) => f());
    void unlistenWeb.then((f) => f());
  };
}

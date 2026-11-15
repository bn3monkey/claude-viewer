import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { create } from "zustand";
import * as ContextMenu from "@radix-ui/react-context-menu";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { Globe, X } from "lucide-react";
import { askText } from "../../app/PromptDialog";
import type { ViewerMode } from "../../generated/entity/ViewerMode";
import { useTreeVM } from "../../viewmodel/useTreeVM";
import { selectLayout, useViewerVM, type Layout, type Pane, type ViewerTab } from "../../viewmodel/useViewerVM";
import { DiffView } from "./DiffView";
import { FindBar } from "./FindBar";
import { previewModeOf } from "./LinkMenu";
import { DocxView, HtmlView, ImageView, VideoView } from "./MediaViews";
import { PdfView } from "./PdfView";
import { TerminalView } from "./TerminalView";
import { MarkdownView, SourceView } from "./TextViews";
import { WebHeader, WebView } from "./WebView";

const MODE_LABEL: Record<ViewerMode, string> = {
  source: "소스",
  markdown: "Markdown 렌더링",
  html: "HTML 렌더링 (sandbox)",
  pdf: "PDF 렌더링",
  docx: "DOCX 렌더링",
  image: "이미지",
  video: "동영상",
  diff: "git diff",
  terminal: "터미널",
  web: "인터넷",
};

/** 본문 텍스트가 DOM 에 있어 찾기가 되는 모드. PDF 는 canvas, HTML 은 iframe 이라 빠진다. */
const FINDABLE: ViewerMode[] = ["source", "markdown", "diff", "docx"];

const TAB_SUFFIX: Partial<Record<ViewerMode, string>> = { markdown: " · 미리보기", html: " · 렌더링", diff: " · diff" };

/** 본문 양쪽 가장자리 드롭 영역의 폭 (본문 폭 대비). */
const EDGE = 0.22;
/** 이만큼(px) 움직여야 클릭이 아니라 끌기로 본다. */
const DRAG_THRESHOLD = 5;

/** 끄는 탭이 지금 놓일 자리. 탭 바 위면 before 탭 앞(null 이면 끝), 본문 위면 zone. */
type DropTarget = { pane: string; before: string | null } | { pane: string; zone: "left" | "center" | "right" };
const useDropTarget = create<{ target: DropTarget | null }>(() => ({ target: null }));

function dropTargetAt(x: number, y: number, canSplit: boolean): DropTarget | null {
  const el = document.elementFromPoint(x, y);
  if (!el) return null;
  const bar = el.closest<HTMLElement>("[data-tabs-pane]");
  if (bar) {
    const tab = el.closest<HTMLElement>("[data-tab-key]");
    return { pane: bar.dataset.tabsPane!, before: tab?.dataset.tabKey ?? null };
  }
  const main = el.closest<HTMLElement>("[data-main-pane]");
  if (!main) return null;
  const rect = main.getBoundingClientRect();
  const ratio = (x - rect.left) / rect.width;
  const zone = canSplit && ratio < EDGE ? "left" : canSplit && ratio > 1 - EDGE ? "right" : "center";
  return { pane: main.dataset.mainPane!, zone };
}

/**
 * 탭 끌기. WebView2 에서 HTML5 드래그 앤 드롭은 시작하자마자 취소되는 일이 있어 포인터 이벤트로 직접 한다.
 * 포인터를 잡아(capture) 창 밖에서 놓아도 끝이 보장된다. Esc·창 포커스 잃음은 취소.
 */
function beginTabDrag(e: ReactPointerEvent<HTMLElement>, pane: string, key: string) {
  if (e.button !== 0 || (e.target as HTMLElement).closest(".x")) return;
  const el = e.currentTarget;
  const { pointerId, clientX: sx, clientY: sy } = e;
  const vm = useViewerVM.getState;
  let started = false;
  let done = false;

  const move = (ev: PointerEvent) => {
    if (!started) {
      if (Math.hypot(ev.clientX - sx, ev.clientY - sy) < DRAG_THRESHOLD) return;
      started = true;
      try {
        el.setPointerCapture(pointerId);
      } catch {
        // 이미 놓인 포인터면 잡지 못한다. 그래도 창 안에서는 동작한다.
      }
      document.body.classList.add("tab-dragging");
      vm().startDrag(pane, key);
    }
    const layout = selectLayout(vm());
    const canSplit = layout.panes.length > 1 || (layout.panes.find((p) => p.id === pane)?.tabs.length ?? 0) > 1;
    useDropTarget.setState({ target: dropTargetAt(ev.clientX, ev.clientY, canSplit) });
  };
  const finish = (drop: boolean) => {
    if (done) return;
    done = true;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("pointercancel", cancel);
    window.removeEventListener("blur", cancel);
    window.removeEventListener("keydown", esc, true);
    const target = useDropTarget.getState().target;
    useDropTarget.setState({ target: null });
    if (!started) return;
    document.body.classList.remove("tab-dragging");
    vm().endDrag();
    if (!drop || !target) return;
    if ("before" in target) {
      if (target.before !== key) vm().moveTab(key, target.pane, target.before);
    } else if (target.zone === "center") {
      if (target.pane !== pane) vm().moveTab(key, target.pane);
    } else {
      vm().splitWith(key, target.zone);
    }
  };
  const up = () => finish(true);
  const cancel = () => finish(false);
  const esc = (ev: KeyboardEvent) => {
    if (ev.key === "Escape") cancel();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("pointercancel", cancel);
  window.addEventListener("blur", cancel);
  window.addEventListener("keydown", esc, true);
}

/**
 * ⑤ File Viewer. 현재 프로젝트의 레이아웃을 보여 준다. 분할은 1~2개(좌·우)이고 탭을 끌어 나누거나 옮긴다.
 * 소스 탭은 "편집" 을 누르면 그 자리에서 고치고 Ctrl+S 로 저장한다. F12 / Ctrl+클릭은 정의로 간다.
 */
export function ViewerPanel() {
  const layout = useViewerVM(selectLayout);
  const current = useViewerVM((s) => s.current);
  const setRatio = useViewerVM((s) => s.setRatio);
  const [left, right] = layout.panes;

  if (!right) {
    return (
      <div className="viewer">
        <ViewerPane pane={left} layout={layout} />
      </div>
    );
  }
  return (
    <div className="viewer">
      {/* 프로젝트·분할 구성이 바뀌면 저장된 비율로 다시 시작한다. */}
      <PanelGroup key={`${current}|${left.id}|${right.id}`} direction="horizontal" onLayout={(sizes) => setRatio(sizes[0])}>
        <Panel id="pane-left" order={1} defaultSize={layout.ratio} minSize={20}>
          <ViewerPane pane={left} layout={layout} />
        </Panel>
        <PanelResizeHandle className="resize-h" />
        <Panel id="pane-right" order={2} defaultSize={100 - layout.ratio} minSize={20}>
          <ViewerPane pane={right} layout={layout} />
        </Panel>
      </PanelGroup>
    </div>
  );
}

function ViewerPane({ pane, layout }: { pane: Pane; layout: Layout }) {
  const vm = useViewerVM();
  const { activate, close, closeOthers, closePane, splitWith, moveTab, focusPane, openWeb } = vm;
  const dragging = useViewerVM((s) => s.dragging);
  const split = layout.panes.length > 1;
  const focused = layout.focused === pane.id;
  const other = layout.panes.find((p) => p.id !== pane.id) ?? null;
  const active = pane.tabs.find((t) => t.key === pane.activeKey) ?? null;
  const [fitImage, setFitImage] = useState(true);
  const [menuTab, setMenuTab] = useState<string | null>(null);
  const drop = useDropTarget((s) => (s.target?.pane === pane.id ? s.target : null));
  const dropBefore = drop && "before" in drop ? drop.before : undefined;
  const tabBarTarget = drop !== null && "before" in drop && drop.before === null;
  const root = useRef<HTMLDivElement>(null);
  const [body, setBody] = useState<HTMLDivElement | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findFocus, setFindFocus] = useState(0);

  // Ctrl+F 는 마지막으로 클릭한 곳이 이 분할일 때만 가로챈다. 터미널·인터넷 탭은 그쪽에 넘긴다.
  const activeMode = active?.mode;
  useEffect(() => {
    let inside = false;
    const onPointer = (e: PointerEvent) => {
      inside = !!root.current?.contains(e.target as Node);
    };
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || e.code !== "KeyF") return;
      if (!activeMode || activeMode === "terminal" || activeMode === "web") return;
      if (!inside && !root.current?.contains(document.activeElement)) return;
      e.preventDefault();
      setFindOpen(true);
      setFindFocus((n) => n + 1);
    };
    document.addEventListener("pointerdown", onPointer, true);
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("pointerdown", onPointer, true);
      document.removeEventListener("keydown", onKey, true);
    };
  }, [activeMode]);

  const askWeb = async () => {
    const url = await askText("인터넷 탭 열기", "URL (예: notion.so, https://github.com)");
    if (url) openWeb(url, pane.id);
  };

  return (
    <div className={`pane ${split && focused ? "focused" : ""}`} ref={root} onPointerDownCapture={() => focusPane(pane.id)}>
      <ContextMenu.Root onOpenChange={(open) => !open && setMenuTab(null)}>
        <ContextMenu.Trigger asChild>
          <div className={`tabs ${tabBarTarget ? "drop-end" : ""}`} data-tabs-pane={pane.id}>
            {pane.tabs.map((tab) => (
              <div
                key={tab.key}
                className={`tab ${tab.key === pane.activeKey ? "active" : ""} ${dragging && dropBefore === tab.key ? "drop-before" : ""} ${dragging?.key === tab.key ? "dragged" : ""}`}
                title={tab.path}
                data-tab-key={tab.key}
                onPointerDown={(e) => beginTabDrag(e, pane.id, tab.key)}
                onClick={() => activate(pane.id, tab.key)}
                onAuxClick={(e) => e.button === 1 && close(tab.key)}
                onContextMenu={() => setMenuTab(tab.key)}
              >
                {tab.dirty && <span className="dirty" title="저장하지 않은 변경">●</span>}
                {tab.web && <Globe size={12} className={tab.web.loading ? "loading" : ""} />}
                <span className="tab-name">
                  {tab.name}
                  {TAB_SUFFIX[tab.mode]}
                </span>
                <button
                  className="x"
                  title="닫기"
                  onClick={(e) => {
                    e.stopPropagation();
                    close(tab.key);
                  }}
                >
                  <X size={12} />
                </button>
              </div>
            ))}
            <span className="tabs-fill" />
            {split && (
              <button className="pane-close" title="분할 닫기 (이 분할의 탭을 모두 닫음)" onClick={() => closePane(pane.id)}>
                <X size={13} />
              </button>
            )}
          </div>
        </ContextMenu.Trigger>
        <ContextMenu.Portal>
          <ContextMenu.Content className="menu">
            <ContextMenu.Item className="it" disabled={!menuTab} onSelect={() => menuTab && close(menuTab)}>
              닫기
            </ContextMenu.Item>
            <ContextMenu.Item className="it" disabled={!menuTab} onSelect={() => menuTab && closeOthers(menuTab)}>
              다른 탭 모두 닫기
            </ContextMenu.Item>
            <ContextMenu.Item className="it" disabled={pane.tabs.length === 0} onSelect={() => closePane(pane.id)}>
              {split ? "이 분할의 탭 모두 닫기" : "모든 탭 닫기"}
            </ContextMenu.Item>
            <ContextMenu.Separator className="sep" />
            {split ? (
              <ContextMenu.Item className="it" disabled={!menuTab || !other} onSelect={() => menuTab && other && moveTab(menuTab, other.id)}>
                다른 분할로 이동
              </ContextMenu.Item>
            ) : (
              <>
                <ContextMenu.Item className="it" disabled={!menuTab || pane.tabs.length < 2} onSelect={() => menuTab && splitWith(menuTab, "right")}>
                  오른쪽으로 분할
                </ContextMenu.Item>
                <ContextMenu.Item className="it" disabled={!menuTab || pane.tabs.length < 2} onSelect={() => menuTab && splitWith(menuTab, "left")}>
                  왼쪽으로 분할
                </ContextMenu.Item>
              </>
            )}
            <ContextMenu.Separator className="sep" />
            <ContextMenu.Item className="it" onSelect={() => void askWeb()}>
              인터넷 탭 열기…
            </ContextMenu.Item>
          </ContextMenu.Content>
        </ContextMenu.Portal>
      </ContextMenu.Root>
      <div className="pane-main" data-main-pane={pane.id}>
        {active ? (
          <>
            {active.mode === "web" ? (
              <WebHeader tab={active} />
            ) : (
              active.mode !== "pdf" && (
                <div className="vhead">
                  <span className="path" title={active.path}>{active.path}</span>
                  <span className="spacer" />
                  {active.mode === "image" && (
                    <button className="icon" onClick={() => setFitImage(!fitImage)}>
                      {fitImage ? "원본 크기" : "창에 맞춤"}
                    </button>
                  )}
                  {active.mode === "diff" && active.diff && (
                    <span>
                      {active.diff.commit
                        ? `커밋 ${active.diff.commit.slice(0, 7)} 의 변경`
                        : active.diff.branch
                          ? `브랜치 ${active.diff.branch} 기준`
                          : "작업 트리 · 마지막 커밋 기준"}
                    </span>
                  )}
                  <ModeLabel tab={active} onSwitch={(mode) => vm.switchMode(active.key, mode)} />
                  {active.mode === "source" &&
                    (active.editing ? (
                      <>
                        <button className="icon" title="Ctrl+S" disabled={!active.dirty} onClick={() => vm.requestSave(active.key)}>
                          저장
                        </button>
                        <button className="icon" onClick={() => vm.stopEditing(active.key)}>
                          편집 끝
                        </button>
                      </>
                    ) : (
                      <button className="icon" title="이 자리에서 고친다. F12 / Ctrl+클릭은 정의로 이동" onClick={() => vm.startEditing(active.key)}>
                        편집
                      </button>
                    ))}
                  {active.mode !== "terminal" && active.mode !== "source" && (
                    <span className="hint" style={{ marginLeft: 8 }}>읽기 전용</span>
                  )}
                </div>
              )
            )}
            {focused && <DefinitionPicker />}
            {findOpen && active.mode !== "terminal" && active.mode !== "web" && (
              <FindBar
                container={body}
                supported={FINDABLE.includes(active.mode)}
                focusNonce={findFocus}
                onClose={() => setFindOpen(false)}
              />
            )}
            <div
              className={`vbody ${active.mode === "html" ? "frame" : ""} ${active.mode === "terminal" ? "terminal" : ""} ${active.mode === "web" ? "web" : ""}`}
              key={active.key}
              ref={setBody}
            >
              <Body tab={active} fitImage={fitImage} />
            </div>
          </>
        ) : (
          <div className="empty" style={{ marginTop: 40 }}>
            트리에서 파일을 선택하면 여기에 표시됩니다.
            <br />
            탭 바를 우클릭하면 인터넷 탭을 열 수 있습니다.
          </div>
        )}
        {dragging && <DropZones pane={pane} layout={layout} />}
      </div>
    </div>
  );
}

/**
 * 탭을 끄는 동안 본문 위에 뜨는 드롭 영역. 양쪽 가장자리는 그 쪽 분할(없으면 새로 만듦),
 * 가운데는 이 분할로 옮긴다(같은 분할이면 아무 일 없음).
 */
function DropZones({ pane, layout }: { pane: Pane; layout: Layout }) {
  const dragging = useViewerVM((s) => s.dragging)!;
  const zone = useDropTarget((s) => (s.target?.pane === pane.id && "zone" in s.target ? s.target.zone : null));
  const split = layout.panes.length > 1;
  const source = layout.panes.find((p) => p.id === dragging.pane);
  // 분할이 하나이고 탭도 하나면 나눌 수 없다.
  const canSplit = split || (source?.tabs.length ?? 0) > 1;

  const label = (side: "left" | "right") => (split ? (side === "left" ? "왼쪽 분할로" : "오른쪽 분할로") : side === "left" ? "왼쪽에 분할" : "오른쪽에 분할");

  // 판정은 beginTabDrag 가 elementFromPoint 로 하고, 여기는 표시만 한다.
  return (
    <div className="drop-zones">
      {canSplit && <div className={`zone edge ${zone === "left" ? "on" : ""}`}>{zone === "left" && label("left")}</div>}
      <div className={`zone center ${zone === "center" && dragging.pane !== pane.id ? "on" : ""}`}>
        {zone === "center" && dragging.pane !== pane.id && "이 분할로 이동"}
      </div>
      {canSplit && <div className={`zone edge ${zone === "right" ? "on" : ""}`}>{zone === "right" && label("right")}</div>}
    </div>
  );
}

/** .md / .html 은 누를 때마다 소스 ↔ 미리 보기를 오가는 버튼, 나머지는 모드 이름. */
function ModeLabel({ tab, onSwitch }: { tab: ViewerTab; onSwitch: (mode: ViewerMode) => void }) {
  const preview = previewModeOf(tab.path);
  if (!preview || (tab.mode !== "source" && tab.mode !== preview)) return <span>{MODE_LABEL[tab.mode]}</span>;
  const showingSource = tab.mode === "source";
  return (
    <button
      className="icon mode-toggle"
      title={showingSource ? "누르면 미리 보기로 전환" : "누르면 소스로 전환"}
      onClick={() => onSwitch(showingSource ? preview : "source")}
    >
      {showingSource ? "소스" : "미리 보기"}
    </button>
  );
}

/** F12 결과가 여럿일 때 고르는 목록. 포커스된 분할의 머리글 아래에 떠 있고 Esc·바깥 클릭으로 닫힌다. */
function DefinitionPicker() {
  const { definitions, pickDefinition, dismissDefinitions } = useViewerVM();
  const treeRoot = useTreeVM((s) => s.root);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!definitions) return;
    box.current?.focus();
    const onPointer = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node)) dismissDefinitions();
    };
    document.addEventListener("pointerdown", onPointer, true);
    return () => document.removeEventListener("pointerdown", onPointer, true);
  }, [definitions, dismissDefinitions]);

  if (!definitions) return null;
  const relative = (path: string) =>
    treeRoot && path.startsWith(treeRoot) ? path.slice(treeRoot.length).replace(/^[\\/]/, "") : path;
  return (
    <div className="menu defs" ref={box} tabIndex={-1} onKeyDown={(e) => e.key === "Escape" && dismissDefinitions()}>
      <div className="defs-head">
        <span>
          '{definitions.name}' 의 정의 {definitions.hits.length}곳
        </span>
        <button className="x" title="닫기" onClick={dismissDefinitions}>
          <X size={12} />
        </button>
      </div>
      {definitions.hits.map((hit) => (
        <div key={`${hit.path}:${hit.line}`} className="it" onClick={() => pickDefinition(hit)}>
          <span className="loc">
            {relative(hit.path)}:{hit.line}
          </span>
          <span className="src">{hit.text}</span>
        </div>
      ))}
    </div>
  );
}

function Body({ tab, fitImage }: { tab: ViewerTab; fitImage: boolean }) {
  switch (tab.mode) {
    case "source":
      return <SourceView tab={tab} />;
    case "markdown":
      return <MarkdownView path={tab.path} />;
    case "html":
      return <HtmlView path={tab.path} />;
    case "pdf":
      return <PdfView path={tab.path} />;
    case "docx":
      return <DocxView path={tab.path} />;
    case "image":
      return <ImageView path={tab.path} fit={fitImage} />;
    case "video":
      return <VideoView path={tab.path} />;
    case "diff":
      return tab.diff ? <DiffView path={tab.path} diff={tab.diff} /> : null;
    case "terminal":
      return tab.terminal ? <TerminalView id={tab.terminal.id} /> : null;
    case "web":
      return tab.web ? <WebView tab={tab} /> : null;
  }
}

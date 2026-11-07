import { useEffect, useRef, useState } from "react";
import * as ContextMenu from "@radix-ui/react-context-menu";
import { X } from "lucide-react";
import type { ViewerMode } from "../../generated/entity/ViewerMode";
import { useTreeVM } from "../../viewmodel/useTreeVM";
import { useViewerVM, type ViewerTab } from "../../viewmodel/useViewerVM";
import { DiffView } from "./DiffView";
import { FindBar } from "./FindBar";
import { previewModeOf } from "./LinkMenu";
import { DocxView, HtmlView, ImageView, VideoView } from "./MediaViews";
import { PdfView } from "./PdfView";
import { TerminalView } from "./TerminalView";
import { MarkdownView, SourceView } from "./TextViews";

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
};

/** 본문 텍스트가 DOM 에 있어 찾기가 되는 모드. PDF 는 canvas, HTML 은 iframe 이라 빠진다. */
const FINDABLE: ViewerMode[] = ["source", "markdown", "diff", "docx"];

const TAB_SUFFIX: Partial<Record<ViewerMode, string>> = { markdown: " · 미리보기", html: " · 렌더링", diff: " · diff" };

/** ⑤ File Viewer. 소스 탭은 "편집" 을 누르면 그 자리에서 고치고 Ctrl+S 로 저장한다. F12 / Ctrl+클릭은 정의로 간다. */
export function ViewerPanel() {
  const { tabs, activeKey, activate, close, closeOthers, closeAll, switchMode, startEditing, stopEditing, requestSave } =
    useViewerVM();
  const active = tabs.find((t) => t.key === activeKey) ?? null;
  const [fitImage, setFitImage] = useState(true);
  const [menuTab, setMenuTab] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const [body, setBody] = useState<HTMLDivElement | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findFocus, setFindFocus] = useState(0);

  // Ctrl+F 는 마지막으로 클릭한 곳이 이 패널일 때만 가로챈다. 터미널 탭은 셸에 넘긴다.
  const activeMode = active?.mode;
  useEffect(() => {
    let inside = false;
    const onPointer = (e: PointerEvent) => {
      inside = !!root.current?.contains(e.target as Node);
    };
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || e.code !== "KeyF") return;
      if (!activeMode || activeMode === "terminal") return;
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

  return (
    <div className="viewer" ref={root}>
      <ContextMenu.Root onOpenChange={(open) => !open && setMenuTab(null)}>
        <ContextMenu.Trigger asChild disabled={tabs.length === 0}>
          <div className="tabs">
            {tabs.map((tab) => (
              <div
                key={tab.key}
                className={`tab ${tab.key === activeKey ? "active" : ""}`}
                title={tab.path}
                onClick={() => activate(tab.key)}
                onAuxClick={(e) => e.button === 1 && close(tab.key)}
                onContextMenu={() => setMenuTab(tab.key)}
              >
                {tab.dirty && <span className="dirty" title="저장하지 않은 변경">●</span>}
                {tab.name}
                {TAB_SUFFIX[tab.mode]}
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
            <ContextMenu.Separator className="sep" />
            <ContextMenu.Item className="it" onSelect={closeAll}>
              모든 탭 닫기
            </ContextMenu.Item>
          </ContextMenu.Content>
        </ContextMenu.Portal>
      </ContextMenu.Root>
      {active ? (
        <>
          {active.mode !== "pdf" && (
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
              <ModeLabel tab={active} onSwitch={(mode) => switchMode(active.key, mode)} />
              {active.mode === "source" &&
                (active.editing ? (
                  <>
                    <button className="icon" title="Ctrl+S" disabled={!active.dirty} onClick={() => requestSave(active.key)}>
                      저장
                    </button>
                    <button className="icon" onClick={() => stopEditing(active.key)}>
                      편집 끝
                    </button>
                  </>
                ) : (
                  <button className="icon" title="이 자리에서 고친다. F12 / Ctrl+클릭은 정의로 이동" onClick={() => startEditing(active.key)}>
                    편집
                  </button>
                ))}
              {active.mode !== "terminal" && active.mode !== "source" && (
                <span className="hint" style={{ marginLeft: 8 }}>읽기 전용</span>
              )}
            </div>
          )}
          <DefinitionPicker />
          {findOpen && active.mode !== "terminal" && (
            <FindBar
              container={body}
              supported={FINDABLE.includes(active.mode)}
              focusNonce={findFocus}
              onClose={() => setFindOpen(false)}
            />
          )}
          {active.mode !== "terminal" && (
            <div className={`vbody ${active.mode === "html" ? "frame" : ""}`} key={active.key} ref={setBody}>
              <Body tab={active} fitImage={fitImage} />
            </div>
          )}
        </>
      ) : (
        <div className="empty" style={{ marginTop: 40 }}>
          트리에서 파일을 선택하면 여기에 표시됩니다.
        </div>
      )}
      {tabs
        .filter((t) => t.terminal)
        .map((t) => (
          <div key={t.key} className="vbody terminal" style={{ display: t.key === activeKey ? undefined : "none" }}>
            <TerminalView id={t.terminal!.id} active={t.key === activeKey} />
          </div>
        ))}
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

/** F12 결과가 여럿일 때 고르는 목록. 머리글 아래에 떠 있고 Esc·바깥 클릭으로 닫힌다. */
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
      return null;
  }
}

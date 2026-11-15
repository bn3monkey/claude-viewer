import { useEffect, useRef } from "react";
import { Panel, PanelGroup, PanelResizeHandle, type ImperativePanelHandle } from "react-resizable-panels";
import { ConnectionBar } from "../features/connection/ConnectionBar";
import { ProjectPanel } from "../features/projects/ProjectPanel";
import { TreePanel } from "../features/tree/TreePanel";
import { PullDialog } from "../features/tree/PullDialog";
import { startSubagentEvents } from "../viewmodel/useSubagentVM";
import { SessionPanel } from "../features/session/SessionPanel";
import { ViewerPanel } from "../features/viewer/ViewerPanel";
import { useConnectionVM } from "../viewmodel/useConnectionVM";
import { projectRootOf, startProjectEvents, useProjectVM } from "../viewmodel/useProjectVM";
import { startSessionEvents, useSessionVM } from "../viewmodel/useSessionVM";
import { startTreeEvents, useTreeVM } from "../viewmodel/useTreeVM";
import { selectLayout, startViewerEvents, useViewerVM } from "../viewmodel/useViewerVM";
import { PromptDialog } from "./PromptDialog";
import { SessionToggle } from "./SessionToggle";
import { SettingsDialog } from "./SettingsDialog";
import { StatusBar } from "./StatusBar";
import { Toasts } from "./toast";

export function App() {
  useEffect(() => {
    useConnectionVM.getState().init();
    useProjectVM.getState().load();
    useSessionVM.getState().loadDrafts();
    const stops = [
      startProjectEvents(),
      startSessionEvents(),
      startTreeEvents(),
      startViewerEvents(projectRootOf),
      startSubagentEvents(),
    ];
    return () => stops.forEach((stop) => stop());
  }, []);

  // 프로젝트나 그 아래 세션을 고르면 그 프로젝트의 트리와, 그 프로젝트에서 보던 뷰어 화면을 보여 준다.
  const selectedRoot = useProjectVM((s) => s.selection?.root ?? null);
  useEffect(() => {
    useTreeVM.getState().show(selectedRoot);
    useViewerVM.getState().switchProject(selectedRoot ?? "");
  }, [selectedRoot]);

  // 세션 대화창 숨김은 프로젝트마다 따로 기억한다. 패널을 직접 끌어 접은 것도 같은 값에 반영한다.
  const sessionHidden = useViewerVM((s) => selectLayout(s).sessionHidden);
  const sessionPanel = useRef<ImperativePanelHandle>(null);
  useEffect(() => {
    const panel = sessionPanel.current;
    if (!panel) return;
    if (sessionHidden && !panel.isCollapsed()) panel.collapse();
    if (!sessionHidden && panel.isCollapsed()) panel.expand();
  }, [sessionHidden]);
  const setSessionHidden = useViewerVM((s) => s.setSessionHidden);

  return (
    <div className="app" onContextMenu={(e) => e.preventDefault()}>
      <ConnectionBar />
      <PanelGroup direction="horizontal" className="app-main" autoSaveId="layout-main">
        <Panel id="left" order={1} defaultSize={20} minSize={12}>
          <div className="left-column">
            <PanelGroup direction="vertical" autoSaveId="layout-left">
              <Panel defaultSize={20} minSize={10}>
                <ProjectPanel />
              </Panel>
              <PanelResizeHandle className="resize-v" />
              <Panel minSize={20}>
                <TreePanel />
              </Panel>
            </PanelGroup>
            <SessionToggle />
          </div>
        </Panel>
        <PanelResizeHandle className="resize-h" />
        <Panel
          id="session"
          order={2}
          ref={sessionPanel}
          defaultSize={40}
          minSize={20}
          collapsible
          collapsedSize={0}
          onCollapse={() => setSessionHidden(true)}
          onExpand={() => setSessionHidden(false)}
        >
          {/* 접혀도 언마운트하지 않는다. 대화 스트림·입력 초안·권한 질문이 그대로 남는다. */}
          <SessionPanel />
        </Panel>
        <PanelResizeHandle className={`resize-h ${sessionHidden ? "gone" : ""}`} />
        <Panel id="viewer" order={3} defaultSize={40} minSize={15}>
          <ViewerPanel />
        </Panel>
      </PanelGroup>
      <StatusBar />
      <PromptDialog />
      <PullDialog />
      <SettingsDialog />
      <Toasts />
    </div>
  );
}

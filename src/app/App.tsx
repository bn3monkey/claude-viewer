import { useEffect } from "react";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { ConnectionBar } from "../features/connection/ConnectionBar";
import { ProjectPanel } from "../features/projects/ProjectPanel";
import { TreePanel } from "../features/tree/TreePanel";
import { SessionPanel } from "../features/session/SessionPanel";
import { ViewerPanel } from "../features/viewer/ViewerPanel";
import { useConnectionVM } from "../viewmodel/useConnectionVM";
import { startProjectEvents, useProjectVM } from "../viewmodel/useProjectVM";
import { startSessionEvents, useSessionVM } from "../viewmodel/useSessionVM";
import { startTreeEvents, useTreeVM } from "../viewmodel/useTreeVM";
import { startViewerEvents } from "../viewmodel/useViewerVM";
import { PromptDialog } from "./PromptDialog";
import { SettingsDialog } from "./SettingsDialog";
import { StatusBar } from "./StatusBar";
import { Toasts } from "./toast";

export function App() {
  useEffect(() => {
    useConnectionVM.getState().init();
    useProjectVM.getState().load();
    useSessionVM.getState().loadDrafts();
    const stops = [startProjectEvents(), startSessionEvents(), startTreeEvents(), startViewerEvents()];
    return () => stops.forEach((stop) => stop());
  }, []);

  // 프로젝트나 그 아래 세션을 고르면 그 프로젝트의 트리를 보여준다.
  const selectedRoot = useProjectVM((s) => s.selection?.root ?? null);
  useEffect(() => {
    useTreeVM.getState().show(selectedRoot);
  }, [selectedRoot]);

  return (
    <div className="app" onContextMenu={(e) => e.preventDefault()}>
      <ConnectionBar />
      <PanelGroup direction="horizontal" className="app-main" autoSaveId="layout-main">
        <Panel defaultSize={20} minSize={12}>
          <PanelGroup direction="vertical" autoSaveId="layout-left">
            <Panel defaultSize={20} minSize={10}>
              <ProjectPanel />
            </Panel>
            <PanelResizeHandle className="resize-v" />
            <Panel minSize={20}>
              <TreePanel />
            </Panel>
          </PanelGroup>
        </Panel>
        <PanelResizeHandle className="resize-h" />
        <Panel defaultSize={40} minSize={20}>
          <SessionPanel />
        </Panel>
        <PanelResizeHandle className="resize-h" />
        <Panel defaultSize={40} minSize={15}>
          <ViewerPanel />
        </Panel>
      </PanelGroup>
      <StatusBar />
      <PromptDialog />
      <SettingsDialog />
      <Toasts />
    </div>
  );
}

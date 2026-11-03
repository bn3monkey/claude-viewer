import { useState } from "react";
import * as ContextMenu from "@radix-ui/react-context-menu";
import { ChevronDown, ChevronRight, Plus } from "lucide-react";
import type { Project } from "../../generated/entity/Project";
import type { Session } from "../../generated/entity/Session";
import { ClaudeIcon, ProjectIcon } from "../../app/icons";
import { useProjectVM } from "../../viewmodel/useProjectVM";
import { useViewerVM } from "../../viewmodel/useViewerVM";
import { NewSessionDialog } from "./NewSessionDialog";

type Target = { kind: "project"; project: Project } | { kind: "session"; project: Project; session: Session };

/** 세션 없음 · 전부 완료 → 초록, 완료 0 → 빨강, 일부 완료 → 노랑 */
function tagColor(p: Project) {
  if (p.total === 0) return "green";
  if (p.done === 0) return "red";
  return p.done < p.total ? "yellow" : "green";
}

/** ② Projects & Sessions */
export function ProjectPanel() {
  const { projects, selection, collapsed, select, toggle, pickAndOpen, close, closeSession } = useProjectVM();
  const [target, setTarget] = useState<Target | null>(null);
  const [dialogRoot, setDialogRoot] = useState<string | null>(null);

  const closeProject = async (root: string) => {
    await close(root);
    useViewerVM.getState().closeUnder(root);
  };

  return (
    <div className="panel">
      <h3>Projects &amp; Sessions</h3>
      <ContextMenu.Root onOpenChange={(open) => !open && setTarget(null)}>
        <ContextMenu.Trigger asChild disabled={false}>
          <div className="body">
            {projects.map((project) => {
              const open = !collapsed[project.root];
              return (
                <div key={project.root}>
                  <div
                    className={`row ${selection?.root === project.root && !selection.sessionId ? "selected" : ""}`}
                    title={project.root}
                    onClick={() => select({ root: project.root, sessionId: null })}
                    onContextMenu={() => setTarget({ kind: "project", project })}
                  >
                    <span
                      className="ico"
                      onClick={(e) => {
                        e.stopPropagation();
                        toggle(project.root);
                      }}
                    >
                      {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    </span>
                    <span
                      className="ico project-btn"
                      title="프로젝트 폴더 열기 (File Picker)"
                      onClick={(e) => {
                        e.stopPropagation();
                        pickAndOpen();
                      }}
                    >
                      <ProjectIcon />
                    </span>
                    <span className="name">{project.name}</span>
                    <span className={`tag ${tagColor(project)}`} title="완료된 세션 / 전체 세션">
                      {project.done} / {project.total}
                    </span>
                  </div>
                  {open &&
                    project.sessions.map((session) => (
                      <div
                        key={session.id}
                        className={`row session ${selection?.sessionId === session.id ? "selected" : ""}`}
                        onClick={() => select({ root: project.root, sessionId: session.id })}
                        onContextMenu={() => setTarget({ kind: "session", project, session })}
                      >
                        <span className="ico claude">
                          <ClaudeIcon />
                        </span>
                        <span className={`name ${session.messageCount === 0 ? "muted" : ""}`} title={session.title}>
                          {session.title}
                        </span>
                        <span
                          className={`state ${session.state}`}
                          title={session.state === "running" ? "진행 중" : "완료"}
                        />
                      </div>
                    ))}
                </div>
              );
            })}
            <div className="row" style={{ color: "var(--fg2)" }} onClick={pickAndOpen}>
              <span className="ico">
                <Plus size={14} />
              </span>
              <span className="name">프로젝트 열기…</span>
            </div>
          </div>
        </ContextMenu.Trigger>
        {target && (
          <ContextMenu.Portal>
            <ContextMenu.Content className="menu">
              {target.kind === "project" ? (
                <>
                  <ContextMenu.Item className="it" onSelect={() => setDialogRoot(target.project.root)}>
                    새 활성화 세션…
                  </ContextMenu.Item>
                  <ContextMenu.Separator className="sep" />
                  <ContextMenu.Item className="it dangerous" onSelect={() => closeProject(target.project.root)}>
                    Project 닫기
                  </ContextMenu.Item>
                </>
              ) : (
                <>
                  <ContextMenu.Item
                    className="it"
                    onSelect={() => select({ root: target.project.root, sessionId: target.session.id })}
                  >
                    Session 열기
                  </ContextMenu.Item>
                  <ContextMenu.Separator className="sep" />
                  <ContextMenu.Item
                    className="it dangerous"
                    onSelect={() => closeSession(target.project.root, target.session.id)}
                  >
                    Session 닫기
                  </ContextMenu.Item>
                </>
              )}
            </ContextMenu.Content>
          </ContextMenu.Portal>
        )}
      </ContextMenu.Root>
      <NewSessionDialog root={dialogRoot} onClose={() => setDialogRoot(null)} />
    </div>
  );
}

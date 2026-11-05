import { useEffect, useRef, useState } from "react";
import * as ContextMenu from "@radix-ui/react-context-menu";
import { Tree, type NodeApi, type NodeRendererProps, type TreeApi } from "react-arborist";
import { AppWindow, SquareTerminal, ChevronDown, ChevronRight, File, FileCode, FileImage, FileText, FileVideo, Folder, FolderOpen, Globe } from "lucide-react";
import type { FsNode } from "../../generated/entity/FsNode";
import { askConfirm, askText } from "../../app/PromptDialog";
import { useTreeVM } from "../../viewmodel/useTreeVM";
import { useViewerVM } from "../../viewmodel/useViewerVM";
import { IS_WINDOWS, launchTerminal, openShell, runDefault, SHELL_MENU, TERMINAL_MENU } from "../../viewmodel/useTerminalVM";
import { GitPanel } from "./GitPanel";

const ROW_HEIGHT = 22;
const extensionOf = (name: string) => name.split(".").pop()?.toLowerCase() ?? "";
const isExecutable = (node: FsNode) => node.kind === "file" && extensionOf(node.name) === "exe";
const RUNNABLE = ["exe", "bat", "cmd", "ps1", "sh", "bash"];
const isRunnable = (node: FsNode) => node.kind === "file" && RUNNABLE.includes(extensionOf(node.name));
const parentOf = (path: string) => path.replace(/[\\/][^\\/]*$/, "");
const joinPath = (dir: string, rel: string) => `${dir}${dir.includes("\\") ? "\\" : "/"}${rel.replace(/[\\/]/g, dir.includes("\\") ? "\\" : "/")}`;

function FileIcon({ name }: { name: string }) {
  const ext = extensionOf(name);
  if (["png", "jpg", "jpeg", "webp", "bmp", "gif", "svg"].includes(ext)) return <FileImage size={14} />;
  if (ext === "mp4") return <FileVideo size={14} />;
  if (ext === "exe") return <AppWindow size={14} />;
  if (["bat", "cmd", "ps1", "sh", "bash"].includes(ext)) return <SquareTerminal size={14} />;
  if (ext === "html" || ext === "htm") return <Globe size={14} />;
  if (["md", "txt", "pdf", "docx"].includes(ext)) return <FileText size={14} />;
  if (["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "cs", "json", "toml", "yaml", "yml", "css", "sh", "ps1"].includes(ext)) {
    return <FileCode size={14} />;
  }
  return <File size={14} />;
}

/** 실제 너비/높이를 알아야 가상 스크롤 트리를 그릴 수 있다. */
function useSize(active: boolean) {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) =>
      setSize({ width: entry.contentRect.width, height: entry.contentRect.height }),
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [active]);
  return { ref, ...size };
}

type Target = { kind: "dir" | "file"; node: FsNode } | { kind: "empty" };

/** ③ Project Tree */
export function TreePanel() {
  const vm = useTreeVM();
  const { root, tree, selectedFile } = vm;
  const viewer = useViewerVM();
  const [view, setView] = useState<"tree" | "git">("tree");
  const { ref, width, height } = useSize(view === "tree");
  const treeApi = useRef<TreeApi<FsNode>>();
  const [target, setTarget] = useState<Target | null>(null);

  const openFile = (node: FsNode) => {
    vm.selectFile(node.path);
    // 실행 파일은 뷰어로 볼 내용이 없다. 더블클릭이나 우클릭 메뉴로 실행한다.
    // 스크립트는 한 번 클릭하면 소스를 보여 준다.
    if (!isExecutable(node)) viewer.openDefault(node.path);
  };

  const createIn = async (dir: string, kind: "file" | "dir") => {
    const name = await askText(kind === "file" ? "새 파일" : "새 폴더", `${dir} 아래에 만들 이름`);
    if (!name) return;
    if (kind === "file") await vm.createFile(dir, name);
    else await vm.createDir(dir, name);
  };

  const remove = async (node: FsNode) => {
    const what = node.kind === "dir" ? "Directory" : "File";
    if (await askConfirm(`${what} 삭제`, `${node.path} 을(를) 휴지통으로 옮길까요?`)) await vm.remove(node.path);
  };

  const moveTo = async (node: FsNode) => {
    if (!root) return;
    const dest = await askText("이동", "옮길 폴더 (프로젝트 기준 상대 경로, 비우면 루트는 '.')", ".");
    if (dest === null) return;
    await vm.move(node.path, dest === "." ? root : joinPath(root, dest));
  };

  const rename = (node: FsNode) => treeApi.current?.get(node.path)?.edit();

  const onKeyDown = (e: React.KeyboardEvent) => {
    const focused = treeApi.current?.focusedNode?.data;
    if (!focused || treeApi.current?.isEditing) return;
    if (e.key === "F2") rename(focused);
    if (e.key === "Delete") remove(focused);
  };

  return (
    <div className="panel">
      <h3>
        <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
          {view === "tree" ? "Project Tree" : "Git"}
          {tree ? ` · ${tree.name}` : ""}
        </span>
        <span className="toggle">
          <button className={view === "tree" ? "on" : ""} onClick={() => setView("tree")} title="디렉터리 트리">
            Tree
          </button>
          <button className={view === "git" ? "on" : ""} onClick={() => setView("git")} title="git 브랜치와 변경 파일">
            Git
          </button>
        </span>
      </h3>
      {view === "git" && (root ? <GitPanel root={root} /> : <div className="empty">왼쪽에서 프로젝트나 세션을 선택하세요.</div>)}
      {view === "tree" && (
      <ContextMenu.Root onOpenChange={(open) => !open && setTarget(null)}>
        <ContextMenu.Trigger asChild>
          <div
            className="tree-body"
            ref={ref}
            onKeyDown={onKeyDown}
            onContextMenuCapture={(e) => {
              if (!tree) return;
              const el = (e.target as HTMLElement).closest<HTMLElement>("[data-path]");
              const node = el ? treeApi.current?.get(el.dataset.path!)?.data : undefined;
              setTarget(node ? { kind: node.kind, node } : { kind: "empty" });
            }}
          >
            {!root && <div className="empty">왼쪽에서 프로젝트나 세션을 선택하세요.</div>}
            {root && !tree && <div className="empty">불러오는 중…</div>}
            {tree && width > 0 && (
              <Tree<FsNode>
                ref={treeApi}
                data={tree.children ?? []}
                idAccessor="path"
                childrenAccessor={(node) => node.children ?? null}
                openByDefault={false}
                width={width}
                height={height}
                rowHeight={ROW_HEIGHT}
                indent={14}
                selection={selectedFile ?? undefined}
                disableMultiSelection
                onActivate={(node) => node.data.kind === "file" && openFile(node.data)}
                onRename={({ id, name }) => vm.rename(id, name)}
                onMove={({ dragIds, parentId }) => {
                  const dest = parentId ?? root!;
                  dragIds.filter((id) => parentOf(id) !== dest).forEach((id) => vm.move(id, dest));
                }}
              >
                {Row}
              </Tree>
            )}
          </div>
        </ContextMenu.Trigger>
        {target && root && (
          <ContextMenu.Portal>
            {/* 메뉴가 닫히며 트리로 포커스를 돌려주면 인라인 이름 입력창이 포커스를 잃는다 */}
            <ContextMenu.Content className="menu" onCloseAutoFocus={(e) => e.preventDefault()}>
              {target.kind === "empty" && (
                <>
                  <Item onSelect={() => createIn(root, "file")}>새 파일 생성</Item>
                  <Item onSelect={() => createIn(root, "dir")}>새 폴더 생성</Item>
                  <ContextMenu.Separator className="sep" />
                  <ShellItems dir={root} />
                </>
              )}
              {target.kind === "dir" && (
                <>
                  <Item onSelect={() => createIn(target.node.path, "file")}>새 파일 생성</Item>
                  <Item onSelect={() => createIn(target.node.path, "dir")}>아래에 새 Directory 생성</Item>
                  <ContextMenu.Separator className="sep" />
                  <Item onSelect={() => rename(target.node)} shortcut="F2">이름 바꾸기</Item>
                  <Item onSelect={() => moveTo(target.node)} shortcut="드래그 가능">이동…</Item>
                  <Item disabled shortcut="v1">클립보드에 복사</Item>
                  <ContextMenu.Separator className="sep" />
                  <PathItems node={target.node} />
                  <ContextMenu.Separator className="sep" />
                  <ShellItems dir={target.node.path} />
                  <ContextMenu.Separator className="sep" />
                  <Item danger onSelect={() => remove(target.node)} shortcut="Del">Directory 삭제</Item>
                </>
              )}
              {target.kind === "file" && (
                <>
                  {isRunnable(target.node) && (
                    <>
                      <Item onSelect={() => runDefault(target.node.path)} shortcut="더블클릭">실행하기</Item>
                      {TERMINAL_MENU.map((t) => (
                        <Item
                          key={t.kind}
                          disabled={t.windows !== null && t.windows !== IS_WINDOWS}
                          onSelect={() => launchTerminal(target.node.path, t.kind)}
                        >
                          {t.label}
                        </Item>
                      ))}
                      <ContextMenu.Separator className="sep" />
                    </>
                  )}
                  {extensionOf(target.node.name) === "md" && (
                    <Item onSelect={() => viewer.open({ path: target.node.path, mode: "markdown" })}>Markdown 미리보기로 열기</Item>
                  )}
                  {["html", "htm"].includes(extensionOf(target.node.name)) && (
                    <Item onSelect={() => viewer.open({ path: target.node.path, mode: "html" })}>HTML 렌더링으로 열기</Item>
                  )}
                  {["md", "html", "htm"].includes(extensionOf(target.node.name)) && <ContextMenu.Separator className="sep" />}
                  <Item onSelect={() => rename(target.node)} shortcut="F2">이름 바꾸기</Item>
                  <Item onSelect={() => moveTo(target.node)} shortcut="드래그 가능">이동…</Item>
                  <Item disabled shortcut="v1">복사</Item>
                  <ContextMenu.Separator className="sep" />
                  <PathItems node={target.node} />
                  <ContextMenu.Separator className="sep" />
                  <ShellItems dir={parentOf(target.node.path)} />
                  <ContextMenu.Separator className="sep" />
                  <Item danger onSelect={() => remove(target.node)} shortcut="Del">File 삭제</Item>
                </>
              )}
            </ContextMenu.Content>
          </ContextMenu.Portal>
        )}
      </ContextMenu.Root>
      )}
    </div>
  );
}

function PathItems({ node }: { node: FsNode }) {
  const vm = useTreeVM();
  return (
    <>
      <Item onSelect={() => vm.reveal(node.path)}>탐색기로 열기</Item>
      <Item onSelect={() => vm.copyPath(node.path, false)}>절대 경로를 클립보드에 복사</Item>
      <Item onSelect={() => vm.copyPath(node.path, true)}>상대 경로를 클립보드에 복사</Item>
    </>
  );
}

/** 그 폴더(파일이면 들어 있는 폴더)를 위치로 셸을 뷰어 탭에 연다. */
function ShellItems({ dir }: { dir: string }) {
  return (
    <>
      {SHELL_MENU.map((t) => (
        <Item key={t.kind} disabled={t.windows !== null && t.windows !== IS_WINDOWS} onSelect={() => openShell(dir, t.kind)}>
          {t.label}
        </Item>
      ))}
    </>
  );
}

function Item(props: { children: React.ReactNode; onSelect?: () => void; shortcut?: string; danger?: boolean; disabled?: boolean }) {
  return (
    <ContextMenu.Item className={`it ${props.danger ? "dangerous" : ""}`} onSelect={props.onSelect} disabled={props.disabled}>
      {props.children}
      {props.shortcut && <span className="k">{props.shortcut}</span>}
    </ContextMenu.Item>
  );
}

function Row({ node, style, dragHandle }: NodeRendererProps<FsNode>) {
  const isDir = node.data.kind === "dir";
  return (
    <div
      ref={dragHandle}
      style={style}
      data-path={node.data.path}
      className={`tree-row ${node.isSelected ? "selected" : ""} ${node.willReceiveDrop ? "drop" : ""}`}
      onClick={() => isDir && node.toggle()}
      onDoubleClick={() => isRunnable(node.data) && runDefault(node.data.path)}
    >
      <span className="ico">{isDir ? node.isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} /> : null}</span>
      <span className={`ico ${isDir ? "" : "file"}`}>
        {isDir ? node.isOpen ? <FolderOpen size={14} /> : <Folder size={14} /> : <FileIcon name={node.data.name} />}
      </span>
      {node.isEditing ? <RenameInput node={node} /> : <span className="name">{node.data.name}</span>}
    </div>
  );
}

function RenameInput({ node }: { node: NodeApi<FsNode> }) {
  const name = node.data.name;
  const input = useRef<HTMLInputElement>(null);
  // react-arborist 가 편집 시작 직후 포커스를 트리 항목으로 되돌리므로 한 프레임 뒤에 가져온다.
  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      const el = input.current;
      if (!el) return;
      el.focus();
      const dot = name.lastIndexOf(".");
      el.setSelectionRange(0, dot > 0 && node.data.kind === "file" ? dot : name.length);
    });
    return () => cancelAnimationFrame(frame);
  }, [name, node.data.kind]);
  return (
    <input
      ref={input}
      defaultValue={name}
      onClick={(e) => e.stopPropagation()}
      onBlur={() => node.reset()}
      onKeyDown={(e) => {
        if (e.key === "Escape") node.reset();
        if (e.key === "Enter") {
          const value = e.currentTarget.value.trim();
          if (value && value !== name) node.submit(value);
          else node.reset();
        }
      }}
    />
  );
}

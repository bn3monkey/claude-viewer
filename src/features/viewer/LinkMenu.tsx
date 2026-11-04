import { useState, type ReactNode } from "react";
import * as ContextMenu from "@radix-ui/react-context-menu";
import type { LinkTarget } from "../../generated/entity/LinkTarget";
import type { TerminalKind } from "../../generated/entity/TerminalKind";
import type { TerminalPaths } from "../../generated/entity/TerminalPaths";
import type { ViewerMode } from "../../generated/entity/ViewerMode";
import { call } from "../../viewmodel/invoke";
import { IS_WINDOWS, launchTerminal, runDefault, TERMINAL_MENU } from "../../viewmodel/useTerminalVM";
import { useTreeVM } from "../../viewmodel/useTreeVM";
import { resolveLink, useViewerVM } from "../../viewmodel/useViewerVM";

const extensionOf = (path: string) => path.split(/[\\/]/).pop()?.split(".").pop()?.toLowerCase() ?? "";
const RUNNABLE = ["exe", "bat", "cmd", "ps1", "sh", "bash"];

/** .md / .html 의 미리 보기 모드. 그 밖의 파일은 null. */
export function previewModeOf(path: string): ViewerMode | null {
  const ext = extensionOf(path);
  if (ext === "md") return "markdown";
  if (ext === "html" || ext === "htm") return "html";
  return null;
}

const openTarget = (target: LinkTarget, mode?: ViewerMode) =>
  useViewerVM.getState().open(mode ? { ...target.request, mode } : target.request, { line: target.line ?? undefined });

/** 링크 왼쪽 클릭: .exe 는 바로 실행, 나머지는 뷰어 탭으로. */
export async function activateLink(baseDir: string, href: string) {
  const target = await resolveLink(baseDir, href);
  if (!target) return;
  if (extensionOf(target.request.path) === "exe") await useTreeVM.getState().run(target.request.path);
  else openTarget(target);
}

/** 이 플랫폼에서 경로가 잡힌(설정 또는 자동 감지) 터미널만. */
async function availableTerminals(): Promise<TerminalKind[]> {
  const paths = await call<[TerminalPaths, TerminalPaths]>("terminal_paths");
  if (!paths) return [];
  const [saved, detected] = paths;
  return TERMINAL_MENU.filter((t) => t.windows === null || t.windows === IS_WINDOWS)
    .filter((t) => saved[t.kind] || detected[t.kind])
    .map((t) => t.kind);
}

const terminalLabel = (kind: TerminalKind) =>
  TERMINAL_MENU.find((t) => t.kind === kind)!.label.replace(/\s*\(.*\)$/, "");

/** 대화·마크다운 속 파일 링크의 우클릭 메뉴. 메뉴를 열 때 링크를 풀어 파일 종류에 맞는 항목을 보여 준다. */
export function LinkMenu({ baseDir, href, children }: { baseDir: string; href: string; children: ReactNode }) {
  const [menu, setMenu] = useState<{ target: LinkTarget | null; terminals: TerminalKind[] } | null>(null);

  const load = async () => {
    const target = await resolveLink(baseDir, href);
    const runnable = !!target && RUNNABLE.includes(extensionOf(target.request.path));
    setMenu({ target: target ?? null, terminals: runnable ? await availableTerminals() : [] });
  };

  return (
    <ContextMenu.Root onOpenChange={(open) => (open ? load() : setMenu(null))}>
      <ContextMenu.Trigger asChild>{children}</ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Content className="menu">
          {!menu ? (
            <ContextMenu.Item className="it" disabled>
              불러오는 중…
            </ContextMenu.Item>
          ) : !menu.target ? (
            <ContextMenu.Item className="it" disabled>
              열 수 없는 링크입니다
            </ContextMenu.Item>
          ) : (
            <TargetItems target={menu.target} terminals={menu.terminals} />
          )}
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}

function TargetItems({ target, terminals }: { target: LinkTarget; terminals: TerminalKind[] }) {
  const path = target.request.path;
  const ext = extensionOf(path);
  const preview = previewModeOf(path);
  const runnable = RUNNABLE.includes(ext);
  const { reveal, copyPath } = useTreeVM.getState();
  return (
    <>
      {runnable && (
        <>
          <Item onSelect={() => runDefault(path)}>실행하기</Item>
          {terminals.map((kind) => (
            <Item key={kind} onSelect={() => launchTerminal(path, kind)}>
              {terminalLabel(kind)}
            </Item>
          ))}
          {ext !== "exe" && <Item onSelect={() => openTarget(target, "source")}>소스 보기</Item>}
        </>
      )}
      {preview && (
        <>
          <Item onSelect={() => openTarget(target, "source")}>텍스트로 띄우기</Item>
          <Item onSelect={() => openTarget(target, preview)}>미리보기로 띄우기</Item>
        </>
      )}
      {!runnable && !preview && <Item onSelect={() => openTarget(target)}>뷰어에서 열기</Item>}
      <ContextMenu.Separator className="sep" />
      <Item onSelect={() => reveal(path)}>탐색기로 열기</Item>
      <Item onSelect={() => copyPath(path, false)}>절대 경로 복사</Item>
      <Item onSelect={() => copyPath(path, true)}>상대 경로 복사</Item>
    </>
  );
}

function Item({ onSelect, children }: { onSelect: () => void; children: ReactNode }) {
  return (
    <ContextMenu.Item className="it" onSelect={onSelect}>
      {children}
    </ContextMenu.Item>
  );
}

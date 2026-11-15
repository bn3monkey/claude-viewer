import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { call } from "./invoke";
import { attachTerminal, disposeTerminal } from "./useTerminalVM";

/**
 * 터미널 화면(xterm)은 탭을 다른 분할로 옮기거나 프로젝트를 바꿔 화면에서 내려가도 스크롤백을 잃지 않도록
 * 탭이 닫힐 때까지 여기 둔다. TerminalView 는 붙을 때 이 화면의 DOM 을 자기 자리로 옮긴다.
 */
export type TerminalInstance = { term: Terminal; fit: FitAddon };

const IS_MAC = navigator.userAgent.includes("Mac");
const instances = new Map<string, TerminalInstance & { dispose: () => void }>();

export function terminalInstance(id: string): TerminalInstance {
  const existing = instances.get(id);
  if (existing) return existing;

  const term = new Terminal({
    fontFamily: 'ui-monospace, Consolas, "Cascadia Mono", monospace',
    fontSize: 13,
    cursorBlink: true,
    scrollback: 5000,
    theme: { background: "#1e1e1e", foreground: "#d4d4d4" },
  });
  const fit = new FitAddon();
  term.loadAddon(fit);

  // WebView 에서는 paste 이벤트가 xterm 의 숨은 입력란까지 오지 않을 수 있어 클립보드를 백엔드로 직접 읽는다.
  // Ctrl+C 는 선택이 있을 때만 복사하고, 없으면 셸에 ^C 로 보내 실행 중인 명령을 멈춘다.
  term.attachCustomKeyEventHandler((e) => {
    if (e.type !== "keydown") return true;
    const mod = IS_MAC ? e.metaKey : e.ctrlKey;
    const key = e.key.toLowerCase();
    if ((mod && !e.altKey && key === "v") || (e.shiftKey && e.key === "Insert")) {
      e.preventDefault();
      void pasteInto(id);
      return false;
    }
    if (mod && !e.altKey && key === "c" && (e.shiftKey || term.hasSelection())) {
      e.preventDefault();
      void copyFrom(id);
      return false;
    }
    return true;
  });

  const detach = attachTerminal(id, (data) => term.write(data));
  const input = term.onData((data) => void call("terminal_write", { id, data }));
  const resize = term.onResize(({ cols, rows }) => void call("terminal_resize", { id, cols, rows }));
  const instance = {
    term,
    fit,
    dispose: () => {
      input.dispose();
      resize.dispose();
      detach();
      term.dispose();
      disposeTerminal(id);
    },
  };
  instances.set(id, instance);
  return instance;
}

/** 탭이 닫힐 때. 출력 구독도 함께 정리한다. */
export function disposeTerminalInstance(id: string) {
  instances.get(id)?.dispose();
  instances.delete(id);
  disposeTerminal(id);
}

export async function pasteInto(id: string) {
  const instance = instances.get(id);
  const text = await call<string>("terminal_paste");
  // paste() 가 줄바꿈 변환과 bracketed paste 를 처리한다.
  if (instance && text) instance.term.paste(text);
}

export async function copyFrom(id: string) {
  const instance = instances.get(id);
  const text = instance?.term.getSelection();
  if (!instance || !text) return;
  if ((await call("terminal_copy", { text })) !== undefined) instance.term.clearSelection();
}

/** 우클릭: Windows Terminal 처럼 선택이 있으면 복사, 없으면 붙여넣기. */
export function rightClick(id: string) {
  if (instances.get(id)?.term.hasSelection()) void copyFrom(id);
  else void pasteInto(id);
}

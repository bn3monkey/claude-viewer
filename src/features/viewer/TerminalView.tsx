import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { call } from "../../viewmodel/invoke";
import { attachTerminal, disposeTerminal } from "../../viewmodel/useTerminalVM";

/** PTY 로 띄운 셸 화면. 탭을 오가도 내용이 남도록 탭이 닫힐 때까지 마운트를 유지한다. */
export function TerminalView({ id, active }: { id: string; active: boolean }) {
  const host = useRef<HTMLDivElement>(null);
  const term = useRef<Terminal | null>(null);
  const fit = useRef<FitAddon | null>(null);

  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const terminal = new Terminal({
      fontFamily: 'ui-monospace, Consolas, "Cascadia Mono", monospace',
      fontSize: 13,
      cursorBlink: true,
      scrollback: 5000,
      theme: { background: "#1e1e1e", foreground: "#d4d4d4" },
    });
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(el);
    term.current = terminal;
    fit.current = fitAddon;

    const detach = attachTerminal(id, (data) => terminal.write(data));
    const input = terminal.onData((data) => void call("terminal_write", { id, data }));
    const resize = terminal.onResize(({ cols, rows }) => void call("terminal_resize", { id, cols, rows }));
    const observer = new ResizeObserver(() => {
      if (el.offsetWidth > 0 && el.offsetHeight > 0) fitAddon.fit();
    });
    observer.observe(el);
    return () => {
      observer.disconnect();
      input.dispose();
      resize.dispose();
      detach();
      terminal.dispose();
      // 터미널 화면은 탭이 닫힐 때만 사라지므로 출력 구독도 함께 정리한다.
      disposeTerminal(id);
    };
  }, [id]);

  // 다시 보이면 크기를 맞추고 입력을 받는다.
  useEffect(() => {
    if (!active) return;
    requestAnimationFrame(() => {
      fit.current?.fit();
      term.current?.focus();
    });
  }, [active]);

  return <div className="terminal-host" ref={host} />;
}

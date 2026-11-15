import { useEffect, useRef } from "react";
import "@xterm/xterm/css/xterm.css";
import { rightClick, terminalInstance } from "../../viewmodel/terminalInstances";

/**
 * PTY 로 띄운 셸 화면. xterm 은 탭이 닫힐 때까지 terminalInstances 에 살아 있고,
 * 이 컴포넌트는 그 화면을 자기 자리에 붙인다. 분할을 옮기거나 프로젝트를 바꿔도 스크롤백이 남는다.
 */
export function TerminalView({ id }: { id: string }) {
  const host = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const { term, fit } = terminalInstance(id);
    if (!term.element) term.open(el);
    else el.appendChild(term.element);

    const refit = () => {
      if (el.offsetWidth > 0 && el.offsetHeight > 0) fit.fit();
    };
    const observer = new ResizeObserver(refit);
    observer.observe(el);
    requestAnimationFrame(() => {
      refit();
      term.refresh(0, term.rows - 1);
      term.focus();
    });
    return () => observer.disconnect();
  }, [id]);

  return (
    <div
      className="terminal-host"
      ref={host}
      onContextMenu={(e) => {
        e.preventDefault();
        rightClick(id);
      }}
    />
  );
}

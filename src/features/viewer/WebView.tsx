import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, ArrowRight, ExternalLink, RotateCw } from "lucide-react";
import { useOverlayOpen } from "../../app/useOverlayOpen";
import { useToasts } from "../../app/toast";
import { call } from "../../viewmodel/invoke";
import { normalizeWebUrl, useViewerVM, type ViewerTab } from "../../viewmodel/useViewerVM";

type Bounds = { x: number; y: number; width: number; height: number };

/**
 * 인터넷 탭 본문. 실제 페이지는 백엔드가 메인 창 위에 겹쳐 띄우는 자식 웹뷰이고, 이 컴포넌트는 그 자리만 잡는다.
 * 자리(위치·크기)를 매 프레임 확인해 바뀌면 옮긴다. 화면에서 내려가거나, 탭을 끄는 중이거나,
 * 메뉴·다이얼로그·토스트가 떠 있으면 웹뷰가 그것들을 가리지 않게 숨긴다. 숨겨도 페이지는 살아 있다.
 */
export function WebView({ tab }: { tab: ViewerTab }) {
  const id = tab.web!.id;
  const host = useRef<HTMLDivElement>(null);
  const url = useRef(tab.path);
  url.current = tab.path;
  const overlay = useOverlayOpen();
  const dragging = useViewerVM((s) => !!s.dragging);
  const toasts = useToasts((s) => s.toasts.length > 0);
  const visible = !overlay && !dragging && !toasts;

  useEffect(() => {
    if (!visible) return;
    let last = "";
    let frame = 0;
    // 만들기가 끝나기 전에 옮기기를 보내지 않도록 차례로 보낸다.
    let queue: Promise<unknown> = Promise.resolve();
    let shown = false;
    const tick = () => {
      const rect = host.current?.getBoundingClientRect();
      if (rect && rect.width > 0 && rect.height > 0) {
        const bounds: Bounds = { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
        const key = `${bounds.x},${bounds.y},${bounds.width},${bounds.height}`;
        if (key !== last) {
          last = key;
          if (!shown) {
            shown = true;
            queue = queue.then(() => call("web_show", { id, url: url.current, bounds }));
          } else {
            queue = queue.then(() => invoke("web_set_bounds", { id, bounds }).catch(() => {}));
          }
        }
      }
      frame = requestAnimationFrame(tick);
    };
    tick();
    return () => {
      cancelAnimationFrame(frame);
      void queue.then(() => invoke("web_hide", { id }).catch(() => {}));
    };
  }, [id, visible]);

  return (
    <div className="web-host" ref={host}>
      {!visible && <div className="empty">페이지는 메뉴·창이 닫히면 다시 보입니다.</div>}
    </div>
  );
}

/** 인터넷 탭 머리글: 뒤로 · 앞으로 · 새로 고침 · 주소 입력 · 외부 브라우저로 열기. */
export function WebHeader({ tab }: { tab: ViewerTab }) {
  const id = tab.web!.id;
  const [address, setAddress] = useState(tab.path);
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) setAddress(tab.path);
  }, [tab.path]);

  const go = () => {
    const url = normalizeWebUrl(address);
    if (!url) {
      useToasts.getState().push(`http(s) 주소만 열 수 있습니다: ${address}`);
      return;
    }
    editing.current = false;
    void call("web_navigate", { id, url });
  };

  return (
    <div className="vhead web-head">
      <button className="icon" title="뒤로" onClick={() => void call("web_history", { id, forward: false })}>
        <ArrowLeft size={13} />
      </button>
      <button className="icon" title="앞으로" onClick={() => void call("web_history", { id, forward: true })}>
        <ArrowRight size={13} />
      </button>
      <button className="icon" title="새로 고침" onClick={() => void call("web_reload", { id })}>
        <RotateCw size={13} className={tab.web?.loading ? "spin" : ""} />
      </button>
      <input
        className="address"
        value={address}
        spellCheck={false}
        onFocus={(e) => {
          editing.current = true;
          e.currentTarget.select();
        }}
        onBlur={() => {
          editing.current = false;
          setAddress(tab.path);
        }}
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            go();
            e.currentTarget.blur();
          } else if (e.key === "Escape") {
            e.currentTarget.blur();
          }
        }}
      />
      <button className="icon" title="기본 브라우저로 열기" onClick={() => void call("viewer_open_url", { url: tab.path })}>
        <ExternalLink size={13} />
      </button>
    </div>
  );
}

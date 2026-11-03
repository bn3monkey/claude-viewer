import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { renderAsync } from "docx-preview";
import { readBytes } from "../../viewmodel/useViewerVM";
import { useTreeChanged } from "../../viewmodel/useTreeVM";

export function DocxView({ path }: { path: string }) {
  const body = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<"loading" | "ok" | string>("loading");

  useEffect(() => {
    const el = body.current;
    if (!el) return;
    el.innerHTML = "";
    setState("loading");
    readBytes(path).then(async (bytes) => {
      if (!bytes) return;
      try {
        await renderAsync(bytes, el, undefined, { inWrapper: true, ignoreLastRenderedPageBreak: true });
        setState("ok");
      } catch (e) {
        setState(String(e));
      }
    });
  }, [path]);

  return (
    <div className="docx">
      {state === "loading" && <div className="empty">불러오는 중…</div>}
      {state !== "loading" && state !== "ok" && <div className="empty">DOCX 를 열 수 없습니다: {state}</div>}
      <div ref={body} />
    </div>
  );
}

/**
 * HTML 을 앱 안에서 sandbox iframe 으로 렌더링한다. 스크립트는 돌지만 origin 이 없어 앱에 닿지 못한다.
 * 백엔드의 `htmlview` 스킴이 파일 경로 구조를 유지한 URL 로 서빙하므로 상대 경로 css·이미지가 같은 폴더에서 읽힌다.
 */
export function HtmlView({ path }: { path: string }) {
  const nonce = useTreeChanged((s) => s.nonce);
  // convertFileSrc 는 경로 전체를 한 조각으로 인코딩하므로 base 만 빌려 쓰고 경로는 직접 붙인다.
  const base = convertFileSrc("", "htmlview");
  const src = base + encodeURI(path.replace(/\\/g, "/").replace(/^\//, "")) + `?v=${nonce}`;
  return <iframe className="html-frame" sandbox="allow-scripts" src={src} title={path} />;
}

export function ImageView({ path, fit }: { path: string; fit: boolean }) {
  return (
    <div className={`media ${fit ? "" : "original"}`}>
      <img className={fit ? "fit" : ""} src={convertFileSrc(path)} alt={path} />
    </div>
  );
}

export function VideoView({ path }: { path: string }) {
  return (
    <div className="media">
      <video controls src={convertFileSrc(path)} />
    </div>
  );
}

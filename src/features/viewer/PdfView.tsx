import { useEffect, useRef, useState } from "react";
import * as pdfjs from "pdfjs-dist/legacy/build/pdf.mjs";
import type { PDFDocumentProxy } from "pdfjs-dist/types/src/display/api";
import workerUrl from "pdfjs-dist/legacy/build/pdf.worker.min.mjs?url";
import { ZoomIn, ZoomOut } from "lucide-react";
import { readBytes } from "../../viewmodel/useViewerVM";

// 구버전 WKWebView 에서도 돌도록 legacy 빌드를 쓴다.
pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export function PdfView({ path }: { path: string }) {
  const [doc, setDoc] = useState<PDFDocumentProxy | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [scale, setScale] = useState(1.25);
  const [page, setPage] = useState(1);
  const container = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let loaded: PDFDocumentProxy | null = null;
    setDoc(null);
    setError(null);
    readBytes(path).then(async (bytes) => {
      if (!bytes) return;
      try {
        loaded = await pdfjs.getDocument({ data: new Uint8Array(bytes) }).promise;
        setDoc(loaded);
      } catch (e) {
        setError(String(e));
      }
    });
    return () => void loaded?.destroy();
  }, [path]);

  const goTo = (n: number) => {
    if (!doc) return;
    const target = Math.min(Math.max(1, n), doc.numPages);
    container.current?.querySelector(`[data-page="${target}"]`)?.scrollIntoView();
    setPage(target);
  };

  if (error) return <div className="empty">PDF 를 열 수 없습니다: {error}</div>;
  if (!doc) return <div className="empty">불러오는 중…</div>;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="vhead">
        <button className="icon" onClick={() => goTo(page - 1)}>‹</button>
        <input
          style={{ width: 48, textAlign: "center", padding: "0 4px" }}
          value={page}
          onChange={(e) => setPage(Number(e.target.value) || 1)}
          onKeyDown={(e) => e.key === "Enter" && goTo(page)}
        />
        <span>/ {doc.numPages}</span>
        <button className="icon" onClick={() => goTo(page + 1)}>›</button>
        <span className="spacer" />
        <button className="icon" onClick={() => setScale((s) => Math.max(0.25, s - 0.25))}><ZoomOut size={14} /></button>
        <span>{Math.round(scale * 100)}%</span>
        <button className="icon" onClick={() => setScale((s) => Math.min(4, s + 0.25))}><ZoomIn size={14} /></button>
      </div>
      <div className="vbody" ref={container}>
        <div className="pdf">
          {Array.from({ length: doc.numPages }, (_, i) => (
            <PdfPage key={i} doc={doc} number={i + 1} scale={scale} onVisible={setPage} />
          ))}
        </div>
      </div>
    </div>
  );
}

/** 화면에 들어온 페이지만 그린다. */
function PdfPage({ doc, number, scale, onVisible }: { doc: PDFDocumentProxy; number: number; scale: number; onVisible: (n: number) => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    doc.getPage(number).then((p) => {
      const viewport = p.getViewport({ scale });
      setSize({ width: viewport.width, height: viewport.height });
    });
  }, [doc, number, scale]);

  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        setVisible(true);
        onVisible(number);
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [number, onVisible]);

  useEffect(() => {
    if (!visible || !size || !canvas.current) return;
    const el = canvas.current;
    let task: { cancel: () => void } | null = null;
    doc.getPage(number).then((p) => {
      const ratio = window.devicePixelRatio || 1;
      const viewport = p.getViewport({ scale: scale * ratio });
      el.width = viewport.width;
      el.height = viewport.height;
      task = p.render({ canvasContext: el.getContext("2d")!, viewport });
    });
    return () => task?.cancel();
  }, [doc, number, scale, visible, size]);

  return (
    <canvas
      ref={canvas}
      data-page={number}
      className="page"
      style={{ width: size?.width ?? 600, height: size?.height ?? 800 }}
    />
  );
}

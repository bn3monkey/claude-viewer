import { memo, useEffect, useState } from "react";
import { Check, Copy } from "lucide-react";
import { useToasts } from "../../app/toast";
import { highlightFence } from "./highlighter";
import { renderMermaid } from "./mermaid";

/**
 * 마크다운 코드 블록. 펜스의 언어로 shiki 구문 강조를 하고, 끝나기 전·모르는 언어는 평문으로 보여 준다.
 * ```mermaid 는 다이어그램으로 그리고, 머리말 버튼으로 소스와 오간다.
 */
export const CodeBlock = memo(function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const isMermaid = lang.toLowerCase() === "mermaid";
  const [html, setHtml] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [showSource, setShowSource] = useState(false);

  useEffect(() => {
    setHtml(null);
    if (!lang || isMermaid) return;
    let alive = true;
    highlightFence(code, lang).then((h) => alive && setHtml(h));
    return () => {
      alive = false;
    };
  }, [code, lang, isMermaid]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      useToasts.getState().push("클립보드에 복사하지 못했습니다");
    }
  };

  return (
    <div className="codeblock">
      <div className="cb-head">
        <span>{lang || "text"}</span>
        <span className="cb-actions">
          {isMermaid && (
            <button className="icon" onClick={() => setShowSource(!showSource)}>
              {showSource ? "다이어그램 보기" : "소스 보기"}
            </button>
          )}
          <button className="icon" title="코드 복사" onClick={copy}>
            {copied ? <Check size={12} /> : <Copy size={12} />}
            {copied ? "복사됨" : "복사"}
          </button>
        </span>
      </div>
      {isMermaid && !showSource ? (
        <MermaidView code={code} />
      ) : html ? (
        // shiki 가 코드 내용을 이스케이프한 HTML 을 만든다.
        <div className="cb-body" dangerouslySetInnerHTML={{ __html: html }} />
      ) : (
        <pre className="cb-body">
          <code>{code}</code>
        </pre>
      )}
    </div>
  );
});

function MermaidView({ code }: { code: string }) {
  const [result, setResult] = useState<{ svg: string } | { error: string } | null>(null);

  useEffect(() => {
    let alive = true;
    setResult(null);
    renderMermaid(code).then((r) => alive && setResult(r));
    return () => {
      alive = false;
    };
  }, [code]);

  if (!result) return <div className="mermaid-view hint">다이어그램 그리는 중…</div>;
  if ("error" in result) {
    return (
      <div className="mermaid-view error">
        <div>다이어그램을 그리지 못했습니다</div>
        <pre>{result.error}</pre>
      </div>
    );
  }
  // mermaid 가 securityLevel strict 로 라벨을 정리한 SVG 다.
  return <div className="mermaid-view" dangerouslySetInnerHTML={{ __html: result.svg }} />;
}

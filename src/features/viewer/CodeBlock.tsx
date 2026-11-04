import { memo, useEffect, useState } from "react";
import { Check, Copy } from "lucide-react";
import { useToasts } from "../../app/toast";
import { highlightFence } from "./highlighter";

/** 마크다운 코드 블록. 펜스의 언어로 shiki 구문 강조를 하고, 끝나기 전·모르는 언어는 평문으로 보여 준다. */
export const CodeBlock = memo(function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const [html, setHtml] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    setHtml(null);
    if (!lang) return;
    let alive = true;
    highlightFence(code, lang).then((h) => alive && setHtml(h));
    return () => {
      alive = false;
    };
  }, [code, lang]);

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
        <button className="icon" title="코드 복사" onClick={copy}>
          {copied ? <Check size={12} /> : <Copy size={12} />}
          {copied ? "복사됨" : "복사"}
        </button>
      </div>
      {html ? (
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

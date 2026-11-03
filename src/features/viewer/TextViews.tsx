import { Fragment, useEffect, useRef, useState } from "react";
import type { TextContent } from "../../generated/entity/TextContent";
import { useTreeChanged } from "../../viewmodel/useTreeVM";
import { readText } from "../../viewmodel/useViewerVM";
import { highlight } from "./highlighter";
import { Markdown } from "./Markdown";

/** 이보다 큰 파일은 구문 강조 없이 보여 준다. */
const HIGHLIGHT_LIMIT = 400 * 1024;

/** 파일이 바뀌면 다시 읽는다. */
function useText(path: string) {
  const nonce = useTreeChanged((s) => s.nonce);
  const [content, setContent] = useState<TextContent | null>(null);
  useEffect(() => {
    let alive = true;
    readText(path).then((c) => alive && c && setContent(c));
    return () => {
      alive = false;
    };
  }, [path, nonce]);
  return content;
}

function Notice({ content }: { content: TextContent }) {
  if (!content.truncated) return null;
  return <div className="hint" style={{ padding: "4px 14px" }}>파일이 커서 앞부분만 표시합니다.</div>;
}

export function SourceView({ path, line, reveal }: { path: string; line?: number; reveal?: number }) {
  const content = useText(path);
  const [html, setHtml] = useState<string | null>(null);
  const code = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setHtml(null);
    if (!content || content.binary || content.text.length > HIGHLIGHT_LIMIT) return;
    let alive = true;
    highlight(content.text, path).then((h) => alive && setHtml(h));
    return () => {
      alive = false;
    };
  }, [content, path]);

  // 링크로 연 줄을 가운데로 가져오고 강조한다. 구문 강조가 끝나 DOM 이 바뀌면 다시 맞춘다.
  useEffect(() => {
    const root = code.current;
    if (!root || !line) return;
    root.querySelectorAll(".line.target").forEach((el) => el.classList.remove("target"));
    const target = root.querySelectorAll(".line")[line - 1];
    target?.classList.add("target");
    target?.scrollIntoView({ block: "center" });
  }, [content, html, line, reveal]);

  if (!content) return <div className="empty">불러오는 중…</div>;
  if (content.binary) return <div className="empty">텍스트로 볼 수 없는 파일입니다.</div>;
  return (
    <div className="code" ref={code}>
      <Notice content={content} />
      {html ? (
        // shiki 가 코드 내용을 이스케이프한 HTML 을 만든다.
        <div dangerouslySetInnerHTML={{ __html: html }} />
      ) : (
        <pre className="plain">
          {content.text.split("\n").map((line, i) => (
            <Fragment key={i}>
              <span className="line">{line}</span>
              {"\n"}
            </Fragment>
          ))}
        </pre>
      )}
    </div>
  );
}

export function MarkdownView({ path }: { path: string }) {
  const content = useText(path);
  if (!content) return <div className="empty">불러오는 중…</div>;
  return (
    <div className="md padded">
      <Notice content={content} />
      <Markdown text={content.text} baseDir={path.replace(/[\\/][^\\/]*$/, "")} />
    </div>
  );
}

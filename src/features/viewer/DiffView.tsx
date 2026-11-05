import { useEffect, useState } from "react";
import type { GitDiff } from "../../generated/entity/GitDiff";
import { readDiff } from "../../viewmodel/useGitVM";
import { useTreeChanged } from "../../viewmodel/useTreeVM";
import type { DiffContext } from "../../viewmodel/useViewerVM";
import { highlight } from "./highlighter";

/** 변경된 파일의 unified diff. 파일이 바뀌면 다시 읽는다. */
export function DiffView({ path, diff }: { path: string; diff: DiffContext }) {
  const nonce = useTreeChanged((s) => s.nonce);
  const [content, setContent] = useState<GitDiff | null>(null);
  const [html, setHtml] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    readDiff(diff.root, diff.branch, diff.commit ?? null, path, diff.untracked).then((d) => alive && d && setContent(d));
    return () => {
      alive = false;
    };
  }, [path, diff.root, diff.branch, diff.commit, diff.untracked, nonce]);

  useEffect(() => {
    setHtml(null);
    if (!content || content.unsupported || !content.text) return;
    let alive = true;
    highlight(content.text, "x.diff").then((h) => alive && setHtml(h));
    return () => {
      alive = false;
    };
  }, [content]);

  if (!content) return <div className="empty">불러오는 중…</div>;
  if (content.unsupported) return <div className="empty">텍스트·소스·html·md 파일만 diff 를 보여줍니다.</div>;
  if (!content.text.trim()) return <div className="empty">변경 내용이 없습니다. (이미 커밋했거나 내용이 같음)</div>;
  return (
    <div className="code diff">
      {html ? (
        <div dangerouslySetInnerHTML={{ __html: html }} />
      ) : (
        <pre className="plain">
          {content.text.split("\n").map((line, i) => (
            <span className={`line ${line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : ""}`} key={i}>
              {line}
              {"\n"}
            </span>
          ))}
        </pre>
      )}
    </div>
  );
}

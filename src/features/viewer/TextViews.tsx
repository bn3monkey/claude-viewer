import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import type { TextContent } from "../../generated/entity/TextContent";
import { useTreeChanged } from "../../viewmodel/useTreeVM";
import { readText, useViewerVM, writeText, type ViewerTab } from "../../viewmodel/useViewerVM";
import { eolOf, handleEditorKey, indentUnitOf, normalizeEol, symbolAtCaret, symbolAtSelection } from "./editing";
import { highlight } from "./highlighter";
import { Markdown } from "./Markdown";

/** 이보다 큰 파일은 구문 강조 없이 보여 준다. */
const HIGHLIGHT_LIMIT = 400 * 1024;
/** 타이핑 뒤 이만큼 쉬면 강조를 다시 그린다. 입력 자체는 textarea 가 바로 받는다. */
const REHIGHLIGHT_DELAY = 150;
/** theme.css 의 `12px/1.5` 와 `padding: 8px 0`. 편집 중 textarea 가 강조 레이어보다 길어지지 않게 높이를 맞춘다. */
const LINE_HEIGHT = 18;
const CODE_PADDING = 16;

/** 파일이 바뀌면 다시 읽는다. `frozen` 이면 (저장 안 한 편집 중) 바깥 변경을 무시한다. */
function useText(path: string, frozen = false, reload = 0) {
  const nonce = useTreeChanged((s) => s.nonce);
  const [content, setContent] = useState<TextContent | null>(null);
  const frozenRef = useRef(frozen);
  frozenRef.current = frozen;
  useEffect(() => {
    if (frozenRef.current) return;
    let alive = true;
    readText(path).then((c) => alive && c && setContent(c));
    return () => {
      alive = false;
    };
  }, [path, nonce, reload]);
  return [content, setContent] as const;
}

function Notice({ content, editing }: { content: TextContent; editing?: boolean }) {
  if (!content.truncated) return null;
  return (
    <div className="hint" style={{ padding: "4px 14px" }}>
      {editing ? "파일이 커서 앞부분만 읽었으므로 편집할 수 없습니다." : "파일이 커서 앞부분만 표시합니다."}
    </div>
  );
}

/** 구문 강조 HTML 또는 평문 pre. 두 경우 모두 줄마다 `.line` 이 있어 줄 찾기·F12 가 같은 DOM 을 본다. */
function Highlighted({ text, html }: { text: string; html: string | null }) {
  if (html) {
    // shiki 가 코드 내용을 이스케이프한 HTML 을 만든다.
    return <div dangerouslySetInnerHTML={{ __html: html }} />;
  }
  return (
    <pre className="plain">
      {text.split("\n").map((line, i) => (
        <Fragment key={i}>
          <span className="line">{line}</span>
          {"\n"}
        </Fragment>
      ))}
    </pre>
  );
}

const countLines = (text: string) => text.split("\n").length;

export function SourceView({ tab }: { tab: ViewerTab }) {
  const { path, line, reveal, editing = false, saveNonce = 0 } = tab;
  const { setDirty, goToDefinition } = useViewerVM();
  const code = useRef<HTMLDivElement>(null);
  const area = useRef<HTMLTextAreaElement>(null);

  // 편집 중인 텍스트. 파일을 (다시) 읽으면 그 내용으로 맞춘다.
  const [text, setText] = useState("");
  const [reload, setReload] = useState(0);
  const [content, setContent] = useText(path, editing && tab.dirty, reload);
  const indent = useMemo(() => (content ? indentUnitOf(normalizeEol(content.text)) : "    "), [content]);
  const editable = !!content && !content.binary && !content.truncated;
  const dirty = editing && editable && text !== normalizeEol(content.text);
  useEffect(() => {
    if (content && !content.binary) setText(normalizeEol(content.text));
  }, [content]);
  useEffect(() => setDirty(tab.key, dirty), [tab.key, dirty, setDirty]);

  // 편집을 끝내면 (버린 변경이 있을 수 있으므로) 디스크 내용으로 되돌린다.
  const wasEditing = useRef(editing);
  useEffect(() => {
    if (wasEditing.current && !editing) setReload((n) => n + 1);
    wasEditing.current = editing;
  }, [editing]);

  // 강조는 타이핑이 멈춘 뒤에 다시 그린다. 읽기 모드에서는 파일 내용 그대로.
  const [shown, setShown] = useState("");
  useEffect(() => {
    if (!content) return;
    if (!editing) {
      setShown(content.text);
      return;
    }
    const timer = window.setTimeout(() => setShown(text), REHIGHLIGHT_DELAY);
    return () => window.clearTimeout(timer);
  }, [content, editing, text]);

  const [html, setHtml] = useState<string | null>(null);
  useEffect(() => {
    if (!content || content.binary || shown.length > HIGHLIGHT_LIMIT) {
      setHtml(null);
      return;
    }
    let alive = true;
    highlight(shown, path).then((h) => alive && setHtml(h));
    return () => {
      alive = false;
    };
  }, [content, shown, path]);

  // 링크로 연 줄을 가운데로 가져오고 강조한다. 구문 강조가 끝나 DOM 이 바뀌면 다시 맞춘다.
  useEffect(() => {
    const root = code.current;
    if (!root || !line) return;
    root.querySelectorAll(".line.target").forEach((el) => el.classList.remove("target"));
    const target = root.querySelectorAll(".line")[line - 1];
    target?.classList.add("target");
    target?.scrollIntoView({ block: "center" });
  }, [content, html, line, reveal]);

  const save = useCallback(async () => {
    if (!content || !editable) return;
    const eol = eolOf(content.text);
    const out = eol === "\n" ? text : text.replace(/\n/g, eol);
    if ((await writeText(path, out)) !== undefined) setContent({ ...content, text: out });
  }, [content, editable, text, path, setContent]);

  // 머리글의 저장 버튼.
  const savedNonce = useRef(saveNonce);
  useEffect(() => {
    if (saveNonce !== savedNonce.current) {
      savedNonce.current = saveNonce;
      void save();
    }
  }, [saveNonce, save]);

  useEffect(() => {
    if (editing) area.current?.focus();
  }, [editing]);

  // F12: 커서(편집 중) 또는 마지막으로 클릭한 곳(읽기 모드)의 식별자.
  const jump = useCallback(() => {
    const root = code.current;
    if (!root) return false;
    const symbol = area.current && document.activeElement === area.current ? symbolAtCaret(area.current) : symbolAtSelection(root);
    if (!symbol) return false;
    void goToDefinition(path, symbol.name, symbol.line);
    return true;
  }, [path, goToDefinition]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "F12" || e.ctrlKey || e.altKey || e.metaKey || e.shiftKey) return;
      if (jump()) e.preventDefault();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [jump]);

  const onClick = (e: MouseEvent) => {
    if ((e.ctrlKey || e.metaKey) && jump()) e.preventDefault();
  };

  if (!content) return <div className="empty">불러오는 중…</div>;
  if (content.binary) return <div className="empty">텍스트로 볼 수 없는 파일입니다.</div>;
  return (
    <div className={`code ${editing ? "editing" : ""}`} ref={code} onClick={onClick}>
      <Notice content={content} editing={editing} />
      <div className="layers" style={editing ? { minHeight: countLines(text) * LINE_HEIGHT + CODE_PADDING } : undefined}>
        <Highlighted text={shown} html={html} />
        {editing && editable && (
          <textarea
            ref={area}
            className="edit-layer"
            value={text}
            wrap="off"
            spellCheck={false}
            autoCapitalize="off"
            autoCorrect="off"
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if ((e.ctrlKey || e.metaKey) && !e.altKey && e.code === "KeyS") {
                e.preventDefault();
                void save();
              } else if (handleEditorKey(e, indent)) {
                e.preventDefault();
              }
            }}
            // textarea 가 스스로 스크롤하면 아래 강조 레이어와 어긋난다. 바깥 본문이 대신 스크롤한다.
            onScroll={(e) => {
              const self = e.currentTarget;
              const body = self.closest(".vbody");
              if (body && (self.scrollTop || self.scrollLeft)) {
                body.scrollTop += self.scrollTop;
                body.scrollLeft += self.scrollLeft;
                self.scrollTop = 0;
                self.scrollLeft = 0;
              }
            }}
          />
        )}
      </div>
    </div>
  );
}

export function MarkdownView({ path }: { path: string }) {
  const [content] = useText(path);
  if (!content) return <div className="empty">불러오는 중…</div>;
  return (
    <div className="md padded">
      <Notice content={content} />
      <Markdown text={content.text} baseDir={path.replace(/[\\/][^\\/]*$/, "")} />
    </div>
  );
}

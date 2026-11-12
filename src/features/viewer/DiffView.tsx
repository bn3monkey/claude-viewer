import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ArrowDown, ArrowUp, Columns2, Rows2 } from "lucide-react";
import type { GitDiff } from "../../generated/entity/GitDiff";
import { readDiff } from "../../viewmodel/useGitVM";
import { useTreeChanged } from "../../viewmodel/useTreeVM";
import type { DiffContext } from "../../viewmodel/useViewerVM";
import { parseUnifiedDiff, toSideBySide, type DiffLine, type ParsedDiff } from "./diffModel";
import { tokenizeLines, type CodeToken } from "./highlighter";

type Layout = "side" | "inline";
const LAYOUT_KEY = "diff.layout";
/** 이보다 긴 diff 는 색을 입히지 않는다. 토큰화가 화면을 멈추게 한다. */
const MAX_HIGHLIGHT_LINES = 6000;

type Tokens = { old: CodeToken[][] | null; new: CodeToken[][] | null };

/** VS Code 처럼 그리는 diff. 나란히 / 한 줄 보기, 양쪽 줄 번호, 문법 색, 바뀐 단어 강조. */
export function DiffView({ path, diff }: { path: string; diff: DiffContext }) {
  const nonce = useTreeChanged((s) => s.nonce);
  const [content, setContent] = useState<GitDiff | null>(null);
  const [layout, setLayoutState] = useState<Layout>(() => (localStorage.getItem(LAYOUT_KEY) === "inline" ? "inline" : "side"));
  const setLayout = (l: Layout) => {
    localStorage.setItem(LAYOUT_KEY, l);
    setLayoutState(l);
  };

  useEffect(() => {
    let alive = true;
    readDiff(diff.root, diff.branch, diff.commit ?? null, path, diff.untracked).then((d) => alive && d && setContent(d));
    return () => {
      alive = false;
    };
  }, [path, diff.root, diff.branch, diff.commit, diff.untracked, nonce]);

  const parsed = useMemo(() => (content && !content.unsupported ? parseUnifiedDiff(content.text) : null), [content]);

  const [tokens, setTokens] = useState<Tokens>({ old: null, new: null });
  useEffect(() => {
    setTokens({ old: null, new: null });
    if (!parsed) return;
    const lines = parsed.hunks.reduce((n, h) => n + h.lines.length, 0);
    if (lines > MAX_HIGHLIGHT_LINES) return;
    let alive = true;
    Promise.all([tokenizeLines(parsed.oldText, path), tokenizeLines(parsed.newText, path)]).then(
      ([o, n]) => alive && setTokens({ old: o, new: n }),
    );
    return () => {
      alive = false;
    };
  }, [parsed, path]);

  if (!content) return <div className="empty">불러오는 중…</div>;
  if (content.unsupported) return <div className="empty">텍스트·소스·html·md 파일만 diff 를 보여줍니다.</div>;
  if (!parsed || (parsed.hunks.length === 0 && parsed.notes.length === 0))
    return <div className="empty">변경 내용이 없습니다. (이미 커밋했거나 내용이 같음)</div>;
  return <DiffBody parsed={parsed} tokens={tokens} layout={layout} setLayout={setLayout} />;
}

function DiffBody({
  parsed,
  tokens,
  layout,
  setLayout,
}: {
  parsed: ParsedDiff;
  tokens: Tokens;
  layout: Layout;
  setLayout: (l: Layout) => void;
}) {
  const scroller = useRef<HTMLDivElement>(null);
  const [marks, setMarks] = useState<{ top: number; height: number; kind: string }[]>([]);

  // 오른쪽 개요 눈금: 바뀐 줄이 전체에서 어디쯤인지.
  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const measure = () => {
      const total = el.scrollHeight || 1;
      const rows = el.querySelectorAll<HTMLElement>("tr.dchange");
      setMarks(
        Array.from(rows, (row) => ({
          top: (row.offsetTop / total) * 100,
          height: Math.max((row.offsetHeight / total) * 100, 0.4),
          kind: row.dataset.kind ?? "mod",
        })),
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el.firstElementChild ?? el);
    return () => observer.disconnect();
  }, [parsed, layout, tokens]);

  /** 다음 / 이전 변경 묶음으로 간다. */
  const jump = (dir: 1 | -1) => {
    const el = scroller.current;
    if (!el) return;
    const starts = Array.from(el.querySelectorAll<HTMLElement>("tr.dstart"), (r) => r.offsetTop);
    const now = el.scrollTop + 4;
    const target = dir > 0 ? starts.find((t) => t > now + 4) : [...starts].reverse().find((t) => t < now - 4);
    if (target !== undefined) el.scrollTo({ top: Math.max(0, target - 40), behavior: "smooth" });
  };

  return (
    <div className="dview">
      <div className="dtoolbar">
        <span className="dstat add">+{parsed.added}</span>
        <span className="dstat del">−{parsed.removed}</span>
        {parsed.notes.map((n) => (
          <span key={n} className="hint">
            {n}
          </span>
        ))}
        <span className="spacer" />
        <button className="icon" title="이전 변경" onClick={() => jump(-1)}>
          <ArrowUp size={13} />
        </button>
        <button className="icon" title="다음 변경" onClick={() => jump(1)}>
          <ArrowDown size={13} />
        </button>
        <span className="dsep" />
        <button className={`icon${layout === "side" ? " on" : ""}`} title="나란히 보기" onClick={() => setLayout("side")}>
          <Columns2 size={13} />
        </button>
        <button className={`icon${layout === "inline" ? " on" : ""}`} title="한 줄로 보기" onClick={() => setLayout("inline")}>
          <Rows2 size={13} />
        </button>
      </div>
      <div className="dmain">
        <div className="dscroll" ref={scroller}>
          {layout === "side" ? <SideTable parsed={parsed} tokens={tokens} /> : <InlineTable parsed={parsed} tokens={tokens} />}
        </div>
        <div
          className="druler"
          onClick={(e) => {
            const el = scroller.current;
            if (!el) return;
            const ratio = (e.clientY - e.currentTarget.getBoundingClientRect().top) / e.currentTarget.clientHeight;
            el.scrollTo({ top: ratio * el.scrollHeight - el.clientHeight / 2 });
          }}
        >
          {marks.map((m, i) => (
            <div key={i} className={`dmark ${m.kind}`} style={{ top: `${m.top}%`, height: `${m.height}%` }} />
          ))}
        </div>
      </div>
    </div>
  );
}

/** 숨겨진 줄 띠. VS Code 의 접힌 영역처럼 보인다. */
function HunkBand({ hidden, section, cols }: { hidden: number; section: string; cols: number }) {
  if (hidden === 0 && !section) return null;
  return (
    <tr className="dhunk">
      <td colSpan={cols}>
        {hidden > 0 ? `⋯ 변경 없는 줄 ${hidden}개` : "⋯"}
        {section && <span className="dsection">{section}</span>}
      </td>
    </tr>
  );
}

function SideTable({ parsed, tokens }: { parsed: ParsedDiff; tokens: Tokens }) {
  return (
    <table className="dtable side">
      <colgroup>
        <col className="dnum" />
        <col />
        <col className="dnum" />
        <col />
      </colgroup>
      <tbody>
        {parsed.hunks.map((hunk, h) => (
          <Fragment key={h}>
            <HunkBand hidden={hunk.hidden} section={hunk.section} cols={4} />
            {toSideBySide(hunk.lines).map((row, i, rows) => {
              const changed = row.left?.kind !== "ctx";
              const prevChanged = i > 0 && rows[i - 1].left?.kind !== "ctx";
              const kind = !row.left ? "add" : !row.right ? "del" : "mod";
              return (
                <tr
                  key={i}
                  className={changed ? `dchange${prevChanged ? "" : " dstart"}` : undefined}
                  data-kind={changed ? kind : undefined}
                >
                  <SideCells line={row.left} side="old" tokens={tokens.old} />
                  <SideCells line={row.right} side="new" tokens={tokens.new} />
                </tr>
              );
            })}
          </Fragment>
        ))}
      </tbody>
    </table>
  );
}

function SideCells({ line, side, tokens }: { line: DiffLine | null; side: "old" | "new"; tokens: CodeToken[][] | null }) {
  if (!line) {
    return (
      <>
        <td className="dno filler" />
        <td className="dcode filler" />
      </>
    );
  }
  const no = side === "old" ? line.oldNo : line.newNo;
  const idx = side === "old" ? line.oldIdx : line.newIdx;
  const cls = line.kind === "ctx" ? "" : line.kind;
  return (
    <>
      <td className={`dno ${cls}`}>{no}</td>
      <td className={`dcode ${cls}`}>
        <Code text={line.text} tokens={idx !== null ? tokens?.[idx] : undefined} changes={line.changes} />
      </td>
    </>
  );
}

function InlineTable({ parsed, tokens }: { parsed: ParsedDiff; tokens: Tokens }) {
  return (
    <table className="dtable inline">
      <colgroup>
        <col className="dnum" />
        <col className="dnum" />
        <col className="dsign" />
        <col />
      </colgroup>
      <tbody>
        {parsed.hunks.map((hunk, h) => (
          <Fragment key={h}>
            <HunkBand hidden={hunk.hidden} section={hunk.section} cols={4} />
            {hunk.lines.map((line, i) => {
              const changed = line.kind !== "ctx";
              const prevChanged = i > 0 && hunk.lines[i - 1].kind !== "ctx";
              const cls = changed ? line.kind : "";
              const lineTokens = line.kind === "del" ? tokens.old?.[line.oldIdx!] : tokens.new?.[line.newIdx!];
              return (
                <tr key={i} className={changed ? `dchange${prevChanged ? "" : " dstart"}` : undefined} data-kind={changed ? line.kind : undefined}>
                  <td className={`dno ${cls}`}>{line.oldNo}</td>
                  <td className={`dno ${cls}`}>{line.newNo}</td>
                  <td className={`dsigncell ${cls}`}>{line.kind === "add" ? "+" : line.kind === "del" ? "−" : ""}</td>
                  <td className={`dcode ${cls}`}>
                    <Code text={line.text} tokens={lineTokens} changes={line.changes} />
                  </td>
                </tr>
              );
            })}
          </Fragment>
        ))}
      </tbody>
    </table>
  );
}

/** 문법 색 조각을 바뀐 글자 범위에서 잘라, 바뀐 부분만 진한 배경으로 감싼다. */
function Code({ text, tokens, changes }: { text: string; tokens?: CodeToken[]; changes?: [number, number][] }) {
  const pieces = tokens && tokens.map((t) => t.content).join("") === text ? tokens : [{ content: text } as CodeToken];
  if (!text) return <>{"​"}</>;
  const out: ReactNode[] = [];
  let pos = 0;
  pieces.forEach((tok, t) => {
    const style = tokenStyle(tok);
    let start = 0;
    while (start < tok.content.length) {
      const abs = pos + start;
      const range = changes?.find(([a, b]) => abs >= a && abs < b);
      const nextEdge = range ? range[1] : (changes?.map(([a]) => a).find((a) => a > abs) ?? Infinity);
      const end = Math.min(tok.content.length, nextEdge - pos);
      const piece = tok.content.slice(start, end);
      out.push(
        <span key={`${t}-${start}`} style={style} className={range ? "dword" : undefined}>
          {piece}
        </span>,
      );
      start = end;
    }
    pos += tok.content.length;
  });
  return <>{out}</>;
}

function tokenStyle(tok: CodeToken): React.CSSProperties | undefined {
  if (!tok.color && !tok.fontStyle) return undefined;
  const f = tok.fontStyle ?? 0;
  return {
    color: tok.color,
    fontStyle: f & 1 ? "italic" : undefined,
    fontWeight: f & 2 ? "bold" : undefined,
    textDecoration: f & 4 ? "underline" : undefined,
  };
}

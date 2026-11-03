import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowDown, ArrowUp, CaseSensitive, X } from "lucide-react";

const ALL = "viewer-find";
const CURRENT = "viewer-find-current";
const MAX_MATCHES = 5000;

/**
 * 활성 탭 본문 안에서만 찾는다. DOM 을 고치지 않고 CSS Custom Highlight 로 칠한다.
 * container 가 null 이거나 supported 가 false 면 (PDF·HTML·이미지 등) 입력을 막는다.
 */
export function FindBar({
  container,
  supported,
  focusNonce,
  onClose,
}: {
  container: HTMLElement | null;
  supported: boolean;
  focusNonce: number;
  onClose: () => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [index, setIndex] = useState(0);
  // 검색어를 바꾸거나 이동할 때만 스크롤한다. DOM 이 바뀌어 다시 찾은 경우에는 제자리.
  const scrollPending = useRef(false);
  // 파일을 비동기로 읽고 구문 강조가 끝나면 DOM 이 바뀌므로 그때 다시 찾는다.
  const [domVersion, setDomVersion] = useState(0);

  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, [focusNonce]);

  useEffect(() => {
    if (!container) return;
    let timer: number | undefined;
    const observer = new MutationObserver(() => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setDomVersion((v) => v + 1), 120);
    });
    observer.observe(container, { childList: true, subtree: true, characterData: true });
    return () => {
      observer.disconnect();
      window.clearTimeout(timer);
    };
  }, [container]);

  const ranges = useMemo(
    () => (container && supported ? findRanges(container, query, caseSensitive) : []),
    [container, supported, query, caseSensitive, domVersion],
  );

  const current = ranges.length ? Math.min(index, ranges.length - 1) : -1;

  useEffect(() => {
    CSS.highlights.set(ALL, new Highlight(...ranges));
    if (current >= 0) CSS.highlights.set(CURRENT, new Highlight(ranges[current]));
    else CSS.highlights.delete(CURRENT);
  }, [ranges, current]);

  useEffect(() => {
    if (!scrollPending.current || current < 0) return;
    scrollPending.current = false;
    ranges[current].startContainer.parentElement?.scrollIntoView({ block: "center", inline: "nearest" });
  }, [ranges, current]);

  useEffect(
    () => () => {
      CSS.highlights.delete(ALL);
      CSS.highlights.delete(CURRENT);
    },
    [],
  );

  // 검색 조건이 바뀌면 처음 결과로 간다.
  const search = (nextQuery: string, nextCase: boolean) => {
    scrollPending.current = true;
    setIndex(0);
    setQuery(nextQuery);
    setCaseSensitive(nextCase);
  };

  const step = (delta: number) => {
    if (!ranges.length) return;
    scrollPending.current = true;
    setIndex((current + delta + ranges.length) % ranges.length);
  };

  return (
    <div className="findbar">
      <input
        ref={input}
        value={query}
        disabled={!supported}
        placeholder={supported ? "이 탭에서 찾기" : "이 형식은 찾기를 지원하지 않습니다"}
        onChange={(e) => search(e.target.value, caseSensitive)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === "F3") {
            e.preventDefault();
            step(e.shiftKey ? -1 : 1);
          } else if (e.key === "Escape") {
            onClose();
          }
        }}
      />
      <span className={`count ${query && !ranges.length ? "none" : ""}`}>
        {query ? (ranges.length ? `${current + 1} / ${ranges.length}${ranges.length >= MAX_MATCHES ? "+" : ""}` : "결과 없음") : ""}
      </span>
      <button
        className={`icon ${caseSensitive ? "on" : ""}`}
        title="대소문자 구분"
        disabled={!supported}
        onClick={() => search(query, !caseSensitive)}
      >
        <CaseSensitive size={14} />
      </button>
      <button className="icon" title="이전 (Shift+Enter)" disabled={!ranges.length} onClick={() => step(-1)}>
        <ArrowUp size={14} />
      </button>
      <button className="icon" title="다음 (Enter)" disabled={!ranges.length} onClick={() => step(1)}>
        <ArrowDown size={14} />
      </button>
      <button className="icon" title="닫기 (Esc)" onClick={onClose}>
        <X size={14} />
      </button>
    </div>
  );
}

/** 텍스트 노드를 이어 붙여 찾으므로 구문 강조로 여러 span 에 걸친 검색어도 잡는다. */
function findRanges(root: HTMLElement, query: string, caseSensitive: boolean): Range[] {
  if (!query) return [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const nodes: Text[] = [];
  const starts: number[] = [];
  let text = "";
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    nodes.push(node as Text);
    starts.push(text.length);
    text += node.nodeValue ?? "";
  }
  const haystack = caseSensitive ? text : text.toLowerCase();
  const needle = caseSensitive ? query : query.toLowerCase();

  const ranges: Range[] = [];
  let first = 0;
  for (let at = haystack.indexOf(needle); at >= 0 && ranges.length < MAX_MATCHES; at = haystack.indexOf(needle, at + needle.length)) {
    const end = at + needle.length;
    while (first + 1 < nodes.length && starts[first + 1] <= at) first++;
    let last = first;
    while (last + 1 < nodes.length && starts[last + 1] < end) last++;
    const range = document.createRange();
    range.setStart(nodes[first], at - starts[first]);
    range.setEnd(nodes[last], end - starts[last]);
    ranges.push(range);
  }
  return ranges;
}

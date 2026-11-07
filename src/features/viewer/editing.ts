import type { KeyboardEvent } from "react";

/** 줄 끝을 LF 로 맞춘다. textarea 는 CRLF 를 LF 로 바꿔 버리므로 비교·저장 전에 같은 꼴로 만든다. */
export const normalizeEol = (text: string) => text.replace(/\r\n?/g, "\n");

/** 파일이 쓰던 줄 끝. 저장할 때 되돌린다. */
export const eolOf = (text: string) => (text.includes("\r\n") ? "\r\n" : "\n");

/** 파일이 쓰는 들여쓰기 한 단. 탭이 보이면 탭, 아니면 첫 들여쓴 줄의 공백 수 (2 또는 4 쪽으로), 없으면 4칸. */
export function indentUnitOf(text: string): string {
  for (const line of text.split("\n")) {
    if (line.startsWith("\t")) return "\t";
    const spaces = /^( +)\S/.exec(line)?.[1].length ?? 0;
    if (spaces > 0) return " ".repeat(spaces % 4 === 0 ? 4 : 2);
  }
  return "    ";
}

const isIdentifierChar = (c: string) => /[A-Za-z0-9_$]/.test(c);

/** `offset` 위치에 걸친 식별자. 식별자 위가 아니면 null. */
export function identifierAt(text: string, offset: number): string | null {
  let start = offset;
  while (start > 0 && isIdentifierChar(text[start - 1])) start--;
  let end = offset;
  while (end < text.length && isIdentifierChar(text[end])) end++;
  return end > start ? text.slice(start, end) : null;
}

export const isIdentifier = (text: string) => /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(text);

/** F12 대상: 식별자와 그 줄 번호 (1부터). */
export type Symbol = { name: string; line: number };

/** 편집 중일 때: textarea 커서 위치의 식별자. */
export function symbolAtCaret(area: HTMLTextAreaElement): Symbol | null {
  const { selectionStart, selectionEnd, value } = area;
  const selected = value.slice(selectionStart, selectionEnd);
  const name = selected && isIdentifier(selected) ? selected : identifierAt(value, selectionStart);
  if (!name) return null;
  return { name, line: value.slice(0, selectionStart).split("\n").length };
}

/** 읽기 모드일 때: 마지막으로 클릭한 곳(selection anchor)이 `root` 안이면 그 식별자. */
export function symbolAtSelection(root: HTMLElement): Symbol | null {
  const selection = window.getSelection();
  const anchor = selection?.anchorNode;
  if (!selection || !anchor || !root.contains(anchor)) return null;
  const element = anchor instanceof Element ? anchor : anchor.parentElement;
  const lineEl = element?.closest(".line");
  if (!lineEl) return null;
  const range = document.createRange();
  range.setStart(lineEl, 0);
  range.setEnd(anchor, selection.anchorOffset);
  const selected = selection.toString().trim();
  const name = selected && isIdentifier(selected) ? selected : identifierAt(lineEl.textContent ?? "", range.toString().length);
  if (!name) return null;
  const lines = Array.from(root.querySelectorAll(".line"));
  return { name, line: lines.indexOf(lineEl) + 1 };
}

/** 되돌리기(Ctrl+Z) 기록이 남도록 execCommand 로 넣고, 막혀 있으면 setRangeText 로 대신한다. */
function insert(area: HTMLTextAreaElement, text: string) {
  if (document.execCommand("insertText", false, text)) return;
  area.setRangeText(text, area.selectionStart, area.selectionEnd, "end");
  area.dispatchEvent(new Event("input", { bubbles: true }));
}

/**
 * Tab 은 들여쓰기 한 단을 넣고, Shift+Tab 은 커서 앞의 한 단을 지운다. Enter 는 윗줄 들여쓰기를 이어 간다.
 * 처리했으면 true (호출 쪽이 preventDefault).
 */
export function handleEditorKey(e: KeyboardEvent<HTMLTextAreaElement>, indent: string): boolean {
  const area = e.currentTarget;
  if (e.key === "Tab" && !e.ctrlKey && !e.altKey && !e.metaKey) {
    if (!e.shiftKey) {
      insert(area, indent);
      return true;
    }
    const { selectionStart, value } = area;
    const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
    const leading = value.slice(lineStart, selectionStart);
    if (leading.endsWith(indent)) {
      area.setSelectionRange(selectionStart - indent.length, selectionStart);
      if (!document.execCommand("delete")) {
        area.setRangeText("", selectionStart - indent.length, selectionStart, "end");
        area.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }
    return true;
  }
  if (e.key === "Enter" && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey && !e.nativeEvent.isComposing) {
    const { selectionStart, value } = area;
    const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
    const leading = /^[ \t]*/.exec(value.slice(lineStart, selectionStart))?.[0] ?? "";
    insert(area, `\n${leading}`);
    return true;
  }
  return false;
}

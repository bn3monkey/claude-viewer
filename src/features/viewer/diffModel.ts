/** unified diff 를 VS Code 처럼 그리기 위한 모양. 화면과 상관없는 순수 함수만 둔다. */

export type DiffLine = {
  kind: "ctx" | "add" | "del";
  text: string;
  oldNo: number | null;
  newNo: number | null;
  /** 옛 쪽(ctx·del) / 새 쪽(ctx·add) 텍스트를 이어 붙였을 때 몇 번째 줄인지. 색 입히기에 쓴다. */
  oldIdx: number | null;
  newIdx: number | null;
  /** 짝지은 반대쪽 줄과 다른 글자 범위 [시작, 끝) */
  changes?: [number, number][];
};

export type Hunk = {
  /** `@@ ... @@` 뒤에 붙는 함수 이름 등 */
  section: string;
  /** 앞 hunk (또는 파일 처음) 과의 사이에 숨겨진 변경 없는 줄 수 */
  hidden: number;
  lines: DiffLine[];
};

export type ParsedDiff = {
  hunks: Hunk[];
  /** new file mode, rename, Binary files differ 같은 머리말 */
  notes: string[];
  added: number;
  removed: number;
  oldText: string;
  newText: string;
};

const HUNK = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@ ?(.*)$/;

export function parseUnifiedDiff(text: string): ParsedDiff {
  const hunks: Hunk[] = [];
  const notes: string[] = [];
  const oldLines: string[] = [];
  const newLines: string[] = [];
  let added = 0;
  let removed = 0;
  let hunk: Hunk | null = null;
  let oldNo = 0;
  let newNo = 0;
  let oldLeft = 0;
  let newLeft = 0;
  let lastNewEnd = 1;

  for (const raw of text.replace(/\r\n/g, "\n").split("\n")) {
    const m = HUNK.exec(raw);
    if (m) {
      oldNo = Number(m[1]);
      newNo = Number(m[3]);
      oldLeft = m[2] === undefined ? 1 : Number(m[2]);
      newLeft = m[4] === undefined ? 1 : Number(m[4]);
      const start = Math.max(newNo, 1);
      hunk = { section: m[5].trim(), hidden: Math.max(0, start - lastNewEnd), lines: [] };
      hunks.push(hunk);
      continue;
    }
    if (!hunk || (oldLeft <= 0 && newLeft <= 0 && raw[0] !== "\\")) {
      hunk = null;
      if (/^(new file|deleted file|rename|similarity|Binary|old mode|new mode)/.test(raw)) notes.push(raw);
      continue;
    }
    // 빈 문맥 줄은 앞 공백이 잘려 올 때가 있다. hunk 머리말의 줄 수로 끝을 알기에 파일 끝 빈 줄과 헷갈리지 않는다.
    const sign = raw === "" ? " " : raw[0];
    const body = raw.slice(1);
    if (sign === " ") {
      oldLeft--;
      newLeft--;
      hunk.lines.push({ kind: "ctx", text: body, oldNo: oldNo++, newNo: newNo++, oldIdx: oldLines.length, newIdx: newLines.length });
      oldLines.push(body);
      newLines.push(body);
    } else if (sign === "-") {
      hunk.lines.push({ kind: "del", text: body, oldNo: oldNo++, newNo: null, oldIdx: oldLines.length, newIdx: null });
      oldLines.push(body);
      oldLeft--;
      removed++;
    } else if (sign === "+") {
      hunk.lines.push({ kind: "add", text: body, oldNo: null, newNo: newNo++, oldIdx: null, newIdx: newLines.length });
      newLines.push(body);
      newLeft--;
      added++;
    } else if (sign === "\\") {
      continue; // "\ No newline at end of file"
    } else {
      hunk = null;
    }
    lastNewEnd = newNo;
  }

  for (const h of hunks) markWordChanges(h.lines);
  return { hunks, notes, added, removed, oldText: oldLines.join("\n"), newText: newLines.join("\n") };
}

/** 나란히 보기의 한 줄. 한쪽이 비면 빗금 칸. */
export type SideRow = { left: DiffLine | null; right: DiffLine | null };

/** 지운 줄 묶음과 더한 줄 묶음을 위에서부터 짝지어 양옆에 놓는다. */
export function toSideBySide(lines: DiffLine[]): SideRow[] {
  const rows: SideRow[] = [];
  let i = 0;
  while (i < lines.length) {
    if (lines[i].kind === "ctx") {
      rows.push({ left: lines[i], right: lines[i] });
      i++;
      continue;
    }
    const dels: DiffLine[] = [];
    const adds: DiffLine[] = [];
    while (i < lines.length && lines[i].kind === "del") dels.push(lines[i++]);
    while (i < lines.length && lines[i].kind === "add") adds.push(lines[i++]);
    for (let k = 0; k < Math.max(dels.length, adds.length); k++) rows.push({ left: dels[k] ?? null, right: adds[k] ?? null });
  }
  return rows;
}

/** 한 변경 묶음 안에서 지운 줄과 더한 줄을 짝지어 바뀐 단어 범위를 표시한다. */
function markWordChanges(lines: DiffLine[]) {
  let i = 0;
  while (i < lines.length) {
    if (lines[i].kind === "ctx") {
      i++;
      continue;
    }
    const dels: DiffLine[] = [];
    const adds: DiffLine[] = [];
    while (i < lines.length && lines[i].kind === "del") dels.push(lines[i++]);
    while (i < lines.length && lines[i].kind === "add") adds.push(lines[i++]);
    for (let k = 0; k < Math.min(dels.length, adds.length); k++) {
      const pair = wordDiff(dels[k].text, adds[k].text);
      if (pair) [dels[k].changes, adds[k].changes] = pair;
    }
  }
}

const WORD = /\w+|\s+|[^\w\s]/g;
const MAX_TOKENS = 400;

/** 단어 단위 LCS. 거의 다 바뀐 줄이면 줄 배경만으로 충분하므로 null. */
export function wordDiff(a: string, b: string): [[number, number][], [number, number][]] | null {
  const ta = a.match(WORD) ?? [];
  const tb = b.match(WORD) ?? [];
  if (ta.length > MAX_TOKENS || tb.length > MAX_TOKENS) return null;
  const n = ta.length;
  const m = tb.length;
  const dp: Uint16Array[] = Array.from({ length: n + 1 }, () => new Uint16Array(m + 1));
  for (let x = n - 1; x >= 0; x--)
    for (let y = m - 1; y >= 0; y--) dp[x][y] = ta[x] === tb[y] ? dp[x + 1][y + 1] + 1 : Math.max(dp[x + 1][y], dp[x][y + 1]);

  const keepA = new Array<boolean>(n).fill(false);
  const keepB = new Array<boolean>(m).fill(false);
  let commonChars = 0;
  for (let x = 0, y = 0; x < n && y < m; ) {
    if (ta[x] === tb[y]) {
      keepA[x++] = keepB[y++] = true;
      if (ta[x - 1].trim()) commonChars += ta[x - 1].length;
    } else if (dp[x + 1][y] >= dp[x][y + 1]) x++;
    else y++;
  }
  if (commonChars === 0) return null;
  return [ranges(ta, keepA), ranges(tb, keepB)];
}

function ranges(tokens: string[], keep: boolean[]): [number, number][] {
  const out: [number, number][] = [];
  let pos = 0;
  tokens.forEach((t, i) => {
    if (!keep[i]) {
      const last = out[out.length - 1];
      if (last && last[1] === pos) last[1] = pos + t.length;
      else out.push([pos, pos + t.length]);
    }
    pos += t.length;
  });
  return out;
}

import type { Mermaid } from "mermaid";

/** mermaid 는 크므로 다이어그램이 처음 나올 때 불러온다. */
let loader: Promise<Mermaid> | null = null;
let sequence = 0;

function load() {
  loader ??= import("mermaid").then(({ default: mermaid }) => {
    mermaid.initialize({
      startOnLoad: false,
      // 라벨 속 HTML·스크립트·클릭 이벤트를 막는다.
      securityLevel: "strict",
      theme: "dark",
      fontFamily: getComputedStyle(document.body).fontFamily,
    });
    return mermaid;
  });
  return loader;
}

export async function renderMermaid(code: string): Promise<{ svg: string } | { error: string }> {
  const id = `mermaid-${++sequence}`;
  try {
    const mermaid = await load();
    const { svg } = await mermaid.render(id, code);
    return { svg };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  } finally {
    // 실패하면 mermaid 가 그리던 임시 요소를 body 에 남길 수 있다.
    document.getElementById(id)?.remove();
    document.getElementById(`d${id}`)?.remove();
  }
}

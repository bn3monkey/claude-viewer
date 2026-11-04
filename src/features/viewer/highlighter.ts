import { createHighlighterCore, type HighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";

/** 확장자 → shiki 언어. 필요한 문법만 그때그때 불러온다. */
const LANGS: Record<string, () => Promise<unknown>> = {
  ts: () => import("@shikijs/langs/typescript"),
  tsx: () => import("@shikijs/langs/tsx"),
  js: () => import("@shikijs/langs/javascript"),
  jsx: () => import("@shikijs/langs/jsx"),
  json: () => import("@shikijs/langs/json"),
  md: () => import("@shikijs/langs/markdown"),
  html: () => import("@shikijs/langs/html"),
  css: () => import("@shikijs/langs/css"),
  scss: () => import("@shikijs/langs/scss"),
  rs: () => import("@shikijs/langs/rust"),
  py: () => import("@shikijs/langs/python"),
  go: () => import("@shikijs/langs/go"),
  java: () => import("@shikijs/langs/java"),
  kt: () => import("@shikijs/langs/kotlin"),
  swift: () => import("@shikijs/langs/swift"),
  c: () => import("@shikijs/langs/c"),
  cpp: () => import("@shikijs/langs/cpp"),
  cs: () => import("@shikijs/langs/csharp"),
  sh: () => import("@shikijs/langs/shellscript"),
  ps1: () => import("@shikijs/langs/powershell"),
  bat: () => import("@shikijs/langs/bat"),
  yaml: () => import("@shikijs/langs/yaml"),
  toml: () => import("@shikijs/langs/toml"),
  xml: () => import("@shikijs/langs/xml"),
  sql: () => import("@shikijs/langs/sql"),
  dockerfile: () => import("@shikijs/langs/docker"),
  ini: () => import("@shikijs/langs/ini"),
  lua: () => import("@shikijs/langs/lua"),
  rb: () => import("@shikijs/langs/ruby"),
  php: () => import("@shikijs/langs/php"),
  vue: () => import("@shikijs/langs/vue"),
  diff: () => import("@shikijs/langs/diff"),
  cmake: () => import("@shikijs/langs/cmake"),
  makefile: () => import("@shikijs/langs/makefile"),
};

const ALIASES: Record<string, string> = {
  mts: "ts", cts: "ts", mjs: "js", cjs: "js", jsonc: "json", json5: "json", markdown: "md", htm: "html",
  h: "c", hpp: "cpp", cc: "cpp", cxx: "cpp", bash: "sh", zsh: "sh", cmd: "bat", yml: "yaml", svg: "xml",
  csproj: "xml", kts: "kt", lock: "toml",
};

export function languageOf(path: string): string | null {
  const name = path.split(/[\\/]/).pop()!.toLowerCase();
  if (name === "dockerfile") return "dockerfile";
  if (name === "makefile") return "makefile";
  if (name === "cmakelists.txt") return "cmake";
  const ext = name.includes(".") ? name.split(".").pop()! : "";
  const lang = ALIASES[ext] ?? ext;
  return lang in LANGS ? lang : null;
}

let highlighter: Promise<HighlighterCore> | null = null;
const loaded = new Map<string, Promise<string>>();

/** 언어 id 를 돌려준다. 실패하면 null (평문으로 보여 준다). */
async function loadLanguage(h: HighlighterCore, lang: string): Promise<string | null> {
  if (!loaded.has(lang)) {
    loaded.set(
      lang,
      LANGS[lang]().then(async (mod) => {
        const grammars = (mod as { default: Parameters<HighlighterCore["loadLanguage"]>[0] }).default;
        await h.loadLanguage(grammars);
        const list = Array.isArray(grammars) ? grammars : [grammars];
        return (list[list.length - 1] as { name: string }).name;
      }),
    );
  }
  return loaded.get(lang)!.catch(() => null);
}

/** 마크다운 코드 펜스의 언어 이름 (```typescript, ```c++ 등) → LANGS 키. */
const FENCE_ALIASES: Record<string, string> = {
  typescript: "ts", javascript: "js", node: "js", rust: "rs", python: "py", py3: "py", golang: "go",
  kotlin: "kt", ruby: "rb", csharp: "cs", "c#": "cs", "c++": "cpp", shell: "sh", shellscript: "sh",
  console: "sh", terminal: "sh", powershell: "ps1", pwsh: "ps1", ps: "ps1", batch: "bat", docker: "dockerfile",
  make: "makefile", patch: "diff", html: "html", xhtml: "html",
};

export function languageOfFence(tag: string): string | null {
  const name = tag.trim().toLowerCase();
  const lang = FENCE_ALIASES[name] ?? ALIASES[name] ?? name;
  return lang in LANGS ? lang : null;
}

export function highlight(code: string, path: string): Promise<string | null> {
  return highlightLanguage(code, languageOf(path));
}

/** 마크다운 코드 블록. 모르는 언어면 null. */
export function highlightFence(code: string, tag: string): Promise<string | null> {
  return highlightLanguage(code, languageOfFence(tag));
}

async function highlightLanguage(code: string, lang: string | null): Promise<string | null> {
  if (!lang) return null;
  highlighter ??= createHighlighterCore({
    themes: [import("@shikijs/themes/dark-plus")],
    langs: [],
    engine: createJavaScriptRegexEngine(),
  });
  const h = await highlighter;
  const id = await loadLanguage(h, lang);
  if (!id) return null;
  return h.codeToHtml(code, { lang: id, theme: "dark-plus" });
}

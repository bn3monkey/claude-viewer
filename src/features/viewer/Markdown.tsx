import { createContext, memo, useContext } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Element, ElementContent } from "hast";
import { CodeBlock } from "./CodeBlock";
import { activateLink, LinkMenu } from "./LinkMenu";

/** 링크의 상대 경로 기준 폴더. 세션 대화는 프로젝트 루트를 넣는다. */
export const LinkBase = createContext<string | null>(null);

// `D:/a.rs`, `file:///…` 링크가 지워지지 않게 href 프로토콜 제한을 푼다.
// 링크는 href 없이 그리고 클릭을 직접 처리하므로 javascript: 같은 값이 실행되지 않는다.
const { href: _href, ...protocols } = defaultSchema.protocols ?? {};
const schema = { ...defaultSchema, protocols };

/** `src/a.rs`, `src/a.rs:42`, `C:\p\a.rs` 처럼 폴더 구분자가 있는 파일 경로. */
const CODE_PATH = /^(?:[A-Za-z]:)?[\w.@~\-\\/]*[\\/][\w.@\-]+\.[A-Za-z0-9]{1,10}(?::\d+(?:[:-]\d+)?)?$/;
const URL = /^[a-z][a-z0-9+.-]+:/i;

/** 세션 대화 출력과 .md 미리보기 공용. baseDir 이 있으면 상대 경로 이미지·링크를 그 폴더 기준으로 읽는다. */
export const Markdown = memo(function Markdown({ text, baseDir }: { text: string; baseDir?: string }) {
  const contextBase = useContext(LinkBase);
  const linkBase = baseDir ?? contextBase;
  const isFileLink = (href: string | undefined): href is string =>
    !!href && !!linkBase && !href.startsWith("#") && (!URL.test(href) || /^file:/i.test(href) || /^[A-Za-z]:[\\/]/.test(href));

  const components: Components = {
    // 파일 링크는 오른쪽 뷰어에 탭으로 연다 (.exe 는 실행). 우클릭하면 여는 방법을 고른다.
    // 웹 주소는 앱 화면이 바뀌지 않게 주소만 보여 준다.
    a: ({ children, href }) =>
      isFileLink(href) ? (
        <LinkMenu baseDir={linkBase!} href={href}>
          <a className="file-link" title={`뷰어에서 열기: ${href}`} onClick={() => activateLink(linkBase!, href)}>
            {children}
          </a>
        </LinkMenu>
      ) : (
        <a title={href}>{children}</a>
      ),
    code: ({ children, className }) => {
      const text = typeof children === "string" ? children : null;
      if (!className && text && linkBase && CODE_PATH.test(text)) {
        return (
          <LinkMenu baseDir={linkBase} href={text}>
            <code className="file-link" title="뷰어에서 열기" onClick={() => activateLink(linkBase, text)}>
              {text}
            </code>
          </LinkMenu>
        );
      }
      return <code className={className}>{children}</code>;
    },
    // 코드 블록(```lang) 은 펜스의 언어로 구문 강조한다.
    pre: ({ node, children }) => {
      const code = node?.children.find((c): c is Element => c.type === "element" && c.tagName === "code");
      if (!code) return <pre>{children}</pre>;
      const classes = (code.properties.className as string[] | undefined) ?? [];
      const lang = classes.find((c) => c.startsWith("language-"))?.slice("language-".length) ?? "";
      return <CodeBlock code={textOf(code).replace(/\n$/, "")} lang={lang} />;
    },
    img: ({ src, alt }) => <img alt={alt} src={resolveImage(src, baseDir)} />,
  };
  return (
    <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[[rehypeSanitize, schema]]} components={components}>
      {text}
    </ReactMarkdown>
  );
});

function textOf(node: ElementContent): string {
  if (node.type === "text") return node.value;
  return node.type === "element" ? node.children.map(textOf).join("") : "";
}

function resolveImage(src: string | undefined, baseDir: string | undefined) {
  if (!src || !baseDir || /^[a-z]+:/i.test(src)) return src;
  const sep = baseDir.includes("\\") ? "\\" : "/";
  return convertFileSrc(`${baseDir}${sep}${src.replace(/^\.\//, "").replace(/[\\/]/g, sep)}`);
}

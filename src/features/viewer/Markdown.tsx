import { memo } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize from "rehype-sanitize";
import { convertFileSrc } from "@tauri-apps/api/core";

/** 세션 대화 출력과 .md 미리보기 공용. baseDir 이 있으면 상대 경로 이미지를 그 폴더 기준으로 읽는다. */
export const Markdown = memo(function Markdown({ text, baseDir }: { text: string; baseDir?: string }) {
  const components: Components = {
    // 링크를 따라가면 앱 화면이 바뀌므로 주소만 보여 준다.
    a: ({ children, href }) => (
      <a title={href} onClick={(e) => e.preventDefault()} href={href}>
        {children}
      </a>
    ),
    img: ({ src, alt }) => <img alt={alt} src={resolveImage(src, baseDir)} />,
  };
  return (
    <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize]} components={components}>
      {text}
    </ReactMarkdown>
  );
});

function resolveImage(src: string | undefined, baseDir: string | undefined) {
  if (!src || !baseDir || /^[a-z]+:/i.test(src)) return src;
  const sep = baseDir.includes("\\") ? "\\" : "/";
  return convertFileSrc(`${baseDir}${sep}${src.replace(/^\.\//, "").replace(/[\\/]/g, sep)}`);
}

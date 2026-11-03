import { useEffect } from "react";
import { GitBranch as BranchIcon, RefreshCw } from "lucide-react";
import type { GitChange } from "../../generated/entity/GitChange";
import { useGitVM } from "../../viewmodel/useGitVM";
import { useTreeChanged } from "../../viewmodel/useTreeVM";
import { useViewerVM } from "../../viewmodel/useViewerVM";

const KIND_LABEL = { added: "A", modified: "M", deleted: "D", renamed: "R" } as const;
const relativeTime = (epochSec: number) => {
  const diff = Date.now() / 1000 - epochSec;
  if (diff < 3600) return `${Math.max(1, Math.round(diff / 60))}분 전`;
  if (diff < 86400) return `${Math.round(diff / 3600)}시간 전`;
  return `${Math.round(diff / 86400)}일 전`;
};
const pad = (n: number) => String(n).padStart(2, "0");
/** 커밋 시각: 올해면 MM-DD HH:mm, 아니면 YYYY-MM-DD */
const commitTime = (epochSec: number) => {
  const d = new Date(epochSec * 1000);
  return d.getFullYear() === new Date().getFullYear()
    ? `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
    : `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};
const SUBJECT_CHARS = 16;
const shortSubject = (s: string) => ([...s].length > SUBJECT_CHARS ? [...s].slice(0, SUBJECT_CHARS).join("") + "…" : s);

/** ③-b Git: 현재 브랜치, 브랜치 목록, 선택한 브랜치의 커밋과 변경 파일 */
export function GitPanel({ root }: { root: string }) {
  const { overview, branch, commits, changes, expanded, details, load, refresh, selectBranch, toggleCommit } = useGitVM();
  const nonce = useTreeChanged((s) => s.nonce);
  const open = useViewerVM((s) => s.open);
  const sep = root.includes("\\") ? "\\" : "/";

  useEffect(() => {
    load(root);
  }, [root, load]);
  // 파일이 바뀌면 상태도 다시 읽는다.
  useEffect(() => {
    if (nonce > 0) refresh();
  }, [nonce, refresh]);

  const openDiff = (change: GitChange) => {
    const path = root + sep + change.path.replace(/\//g, sep);
    open({ path, mode: "diff" }, { diff: { root, branch, untracked: change.untracked } });
  };

  if (!overview) return <div className="empty">불러오는 중…</div>;
  if (!overview.isRepo) return <div className="empty">git 저장소가 아닙니다.</div>;
  const current = overview.branches.find((b) => b.current);
  const selected = branch ?? current?.name ?? null;
  const isOther = !!branch && branch !== current?.name;

  return (
    <div className="git">
      <div className="git-head">
        <BranchIcon size={13} />
        <b title="현재 브랜치">{current?.name ?? "(detached)"}</b>
        <span className="spacer" />
        <button className="icon" title="새로 고침" onClick={refresh}>
          <RefreshCw size={12} />
        </button>
      </div>
      <div className="git-section">브랜치</div>
      <div className="git-branches">
        {overview.branches.map((b) => (
          <div
            key={b.name}
            className={`row ${b.name === selected ? "selected" : ""}`}
            title={`${b.subject}\n${new Date(b.committedAt * 1000).toLocaleString()}`}
            onClick={() => selectBranch(b.current ? null : b.name)}
          >
            <span className="ico">{b.current ? "●" : ""}</span>
            <span className="name">{b.name}</span>
            <span className="hint">{relativeTime(b.committedAt)}</span>
          </div>
        ))}
      </div>
      <div className="git-section">커밋 · {selected}</div>
      <div className="git-commits">
        {commits === null && <div className="empty">불러오는 중…</div>}
        {commits?.length === 0 && <div className="empty">커밋이 없습니다.</div>}
        {commits?.map((c) => (
          <div key={c.hash} className={`commit ${expanded === c.hash ? "open" : ""}`}>
            <div className="row" title={c.subject} onClick={() => toggleCommit(c.hash)}>
              <span className="hash">{c.hash.slice(0, 6)}</span>
              <span className="time">{commitTime(c.committedAt)}</span>
              <span className="name">{shortSubject(c.subject)}</span>
            </div>
            {expanded === c.hash && <pre className="commit-log">{details[c.hash] ?? "불러오는 중…"}</pre>}
          </div>
        ))}
      </div>
      <div className="git-section">{isOther ? `${branch} 가 바꾼 파일 (공통 조상 기준)` : "작업 트리 변경 (마지막 커밋 기준)"}</div>
      <div className="git-changes">
        {changes === null && <div className="empty">불러오는 중…</div>}
        {changes?.length === 0 && <div className="empty">변경된 파일이 없습니다.</div>}
        {changes?.map((c) => (
          <div key={c.path} className="row" title={c.path} onClick={() => openDiff(c)}>
            <span className={`gk ${c.kind}`}>{c.untracked ? "U" : KIND_LABEL[c.kind]}</span>
            <span className="name">{c.path}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

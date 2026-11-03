import { create } from "zustand";
import type { GitChange } from "../generated/entity/GitChange";
import type { GitCommit } from "../generated/entity/GitCommit";
import type { GitDiff } from "../generated/entity/GitDiff";
import type { GitOverview } from "../generated/entity/GitOverview";
import { call } from "./invoke";

type GitVM = {
  root: string | null;
  overview: GitOverview | null;
  /** 선택한 브랜치. null 이면 현재 브랜치(작업 트리). */
  branch: string | null;
  commits: GitCommit[] | null;
  changes: GitChange[] | null;
  /** 펼친 커밋 해시와 그 전체 로그 */
  expanded: string | null;
  details: Record<string, string>;
  load: (root: string) => Promise<void>;
  refresh: () => Promise<void>;
  selectBranch: (branch: string | null) => Promise<void>;
  toggleCommit: (hash: string) => Promise<void>;
};

export const useGitVM = create<GitVM>((set, get) => ({
  root: null,
  overview: null,
  branch: null,
  commits: null,
  changes: null,
  expanded: null,
  details: {},

  load: async (root) => {
    if (root !== get().root) set({ root, overview: null, branch: null, commits: null, changes: null, expanded: null, details: {} });
    await get().refresh();
  },

  refresh: async () => {
    const { root, branch } = get();
    if (!root) return;
    const overview = await call<GitOverview>("git_overview", { root });
    if (!overview || get().root !== root) return;
    set({ overview });
    if (!overview.isRepo) {
      set({ commits: [], changes: [] });
      return;
    }
    const [commits, changes] = await Promise.all([
      call<GitCommit[]>("git_commits", { root, branch }),
      call<GitChange[]>("git_changes", { root, branch }),
    ]);
    if (get().root !== root || get().branch !== branch) return;
    set({ commits: commits ?? [], changes: changes ?? [] });
  },

  selectBranch: async (branch) => {
    set({ branch, commits: null, changes: null, expanded: null });
    await get().refresh();
  },

  toggleCommit: async (hash) => {
    const { root, expanded, details } = get();
    if (expanded === hash) {
      set({ expanded: null });
      return;
    }
    set({ expanded: hash });
    if (root && !details[hash]) {
      const detail = await call<string>("git_commit_detail", { root, hash });
      if (detail !== undefined) set({ details: { ...get().details, [hash]: detail } });
    }
  },
}));

export const readDiff = (root: string, branch: string | null, path: string, untracked: boolean) =>
  call<GitDiff>("git_diff", { root, branch, path, untracked });

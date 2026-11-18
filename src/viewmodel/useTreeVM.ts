import { useEffect, useState } from "react";
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import type { FsNode } from "../generated/entity/FsNode";
import { call } from "./invoke";
import { useToasts } from "../app/toast";

type TreeVM = {
  root: string | null;
  tree: FsNode | null;
  /** 세션 입력창에 @경로로 붙는 파일 */
  selectedFile: string | null;
  show: (root: string | null) => Promise<void>;
  selectFile: (path: string | null) => void;
  rename: (path: string, newName: string) => Promise<void>;
  move: (path: string, destDir: string) => Promise<void>;
  remove: (path: string) => Promise<void>;
  createFile: (dir: string, name: string) => Promise<void>;
  createDir: (dir: string, name: string) => Promise<void>;
  copyPath: (path: string, relative: boolean) => Promise<void>;
  reveal: (path: string) => Promise<void>;
  run: (path: string) => Promise<void>;
};

export const useTreeVM = create<TreeVM>((set, get) => ({
  root: null,
  tree: null,
  selectedFile: null,

  show: async (root) => {
    if (root === get().root) return;
    set({ root, tree: null, selectedFile: null });
    if (!root) return;
    const tree = await call<FsNode>("tree_read", { root });
    if (tree && get().root === root) set({ tree });
  },

  selectFile: (selectedFile) => set({ selectedFile }),

  // 변경 결과는 파일 감시가 tree://changed 로 다시 보내 준다.
  rename: async (path, newName) => void (await call("tree_rename", { path, newName })),
  move: async (path, destDir) => void (await call("tree_move", { path, destDir })),
  remove: async (path) => {
    if ((await call("tree_delete", { path })) !== undefined && get().selectedFile === path) {
      set({ selectedFile: null });
    }
  },
  createFile: async (dir, name) => void (await call("tree_create_file", { dir, name })),
  createDir: async (dir, name) => void (await call("tree_create_dir", { dir, name })),
  copyPath: async (path, relative) => {
    if ((await call("tree_copy_path", { path, relative })) !== undefined) {
      useToasts.getState().push(relative ? "상대 경로를 복사했습니다" : "절대 경로를 복사했습니다", "info");
    }
  },
  reveal: async (path) => void (await call("tree_reveal", { path })),
  run: async (path) => {
    if ((await call("tree_run", { path })) !== undefined) {
      useToasts.getState().push(`실행: ${path.split(/[\\/]/).pop()}`, "info");
    }
  },
}));

export function startTreeEvents() {
  const unFiles = listen<string[]>("fs://changed", ({ payload }) => notifyFilesChanged(payload));
  const unTree = listen<FsNode>("tree://changed", ({ payload }) => {
    if (useTreeVM.getState().root === payload.path) useTreeVM.setState({ tree: payload });
    useTreeChanged.setState((s) => ({ nonce: s.nonce + 1 }));
  });
  return () => {
    void unFiles.then((f) => f());
    void unTree.then((f) => f());
  };
}

/** 프로젝트 안 무엇이든 바뀌면 올라가는 카운터. Git 패널처럼 전체를 다시 봐야 하는 곳만 쓴다. */
export const useTreeChanged = create<{ nonce: number }>(() => ({ nonce: 0 }));

/** 경로 비교용: 구분자를 `/` 로, Windows 라 대소문자를 무시한다. */
const normPath = (p: string) => p.replace(/\\/g, "/").toLowerCase();

/** 마지막으로 바뀐 파일들. `paths === null` 이면 무엇이 바뀌었는지 모른다 (예: git pull) → 모두 다시 읽는다. */
const useFileChanges = create<{ seq: number; paths: string[] | null }>(() => ({ seq: 0, paths: null }));

export function notifyFilesChanged(paths: string[] | null) {
  useFileChanges.setState((s) => ({ seq: s.seq + 1, paths: paths && paths.map(normPath) }));
}

/** 바뀐 경로 `changed` 가 `self` 를 보는 화면에 영향을 주는가. 두 값 모두 normPath 를 거쳤다. */
export type Relevance = (changed: string, self: string) => boolean;
export const sameFile: Relevance = (changed, self) => changed === self;

/**
 * `path` 와 관련된 파일이 바뀔 때만 올라가는 버전. 뷰어는 이걸 의존성으로 다시 읽는다.
 * `related` 는 모듈 수준 상수로 넘긴다 (렌더마다 새 함수면 구독을 다시 건다).
 */
export function useFileVersion(path: string, related: Relevance = sameFile, enabled = true): number {
  const [version, setVersion] = useState(0);
  useEffect(() => {
    if (!enabled) return;
    const self = normPath(path);
    return useFileChanges.subscribe(({ paths }) => {
      if (!paths || paths.some((p) => related(p, self))) setVersion((v) => v + 1);
    });
  }, [path, related, enabled]);
  return version;
}

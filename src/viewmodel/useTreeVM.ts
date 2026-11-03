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
  const unlisten = listen<FsNode>("tree://changed", ({ payload }) => {
    if (useTreeVM.getState().root === payload.path) useTreeVM.setState({ tree: payload });
    useTreeChanged.setState((s) => ({ nonce: s.nonce + 1 }));
  });
  return () => void unlisten.then((f) => f());
}

/** 파일이 바뀌면 뷰어의 텍스트 탭을 다시 읽게 하는 카운터. */
export const useTreeChanged = create<{ nonce: number }>(() => ({ nonce: 0 }));

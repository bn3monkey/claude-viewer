import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { TerminalKind } from "../generated/entity/TerminalKind";
import { useToasts } from "../app/toast";
import { call, errorMessage } from "./invoke";
import { useGitVM } from "./useGitVM";
import { useTreeChanged } from "./useTreeVM";
import { openShell } from "./useTerminalVM";

/** pull 이 실패했을 때 보여줄 대화상자 내용 */
type PullFailure = { root: string; reason: string };

type PullVM = {
  busy: boolean;
  failure: PullFailure | null;
  /** git pull origin <현재 브랜치>. 실패하면 이유와 함께 대화상자를 띄운다. */
  pull: (root: string) => Promise<void>;
  /** git reset --hard HEAD 후 pull 을 다시 시도한다. */
  reset: () => Promise<void>;
  /** git bash 가 있으면 그것, 없으면 OS 기본 터미널을 프로젝트 루트에서 연다. */
  openTerminal: () => Promise<void>;
  cancel: () => void;
};

/** git 명령을 실행한다. 성공하면 null, 실패하면 git 이 낸 이유를 돌려준다. */
const runGit = (command: string, root: string): Promise<string | null> =>
  invoke<void>(command, { root }).then(
    () => null,
    (e) => errorMessage(e) || "(git 이 오류 메시지를 남기지 않았습니다)",
  );

/** 파일이 바뀌었을 수 있으니 Git 패널과 트리·뷰어를 다시 읽게 한다. */
const afterChange = () => {
  void useGitVM.getState().refresh();
  useTreeChanged.setState((s) => ({ nonce: s.nonce + 1 }));
};

export const usePullVM = create<PullVM>((set, get) => ({
  busy: false,
  failure: null,

  pull: async (root) => {
    set({ busy: true });
    const reason = await runGit("git_pull", root);
    set({ busy: false });
    if (reason === null) {
      afterChange();
      useToasts.getState().push("pull 완료", "info");
    } else {
      set({ failure: { root, reason } });
    }
  },

  reset: async () => {
    const failure = get().failure;
    if (!failure) return;
    const { root } = failure;
    set({ busy: true });
    const resetError = await runGit("git_reset_hard", root);
    // reset 이 실패했으면 pull 은 하지 않고 그 이유를 보여준다.
    const reason = resetError ?? (await runGit("git_pull", root));
    set({ busy: false });
    if (resetError === null) afterChange();
    if (reason === null) {
      set({ failure: null });
      useToasts.getState().push("Reset 후 pull 완료", "info");
    } else {
      set({ failure: { root, reason } });
    }
  },

  openTerminal: async () => {
    const failure = get().failure;
    if (!failure) return;
    const kind = await call<TerminalKind>("terminal_repo_shell");
    set({ failure: null });
    if (kind) await openShell(failure.root, kind);
  },

  cancel: () => set({ failure: null }),
}));

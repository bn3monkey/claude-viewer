import { useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { open } from "@tauri-apps/plugin-dialog";
import type { TerminalPaths } from "../generated/entity/TerminalPaths";
import { useTerminalSettingsVM } from "../viewmodel/useTerminalVM";

const FIELDS: { key: keyof TerminalPaths; label: string }[] = [
  { key: "cmd", label: "cmd" },
  { key: "powerShell", label: "PowerShell" },
  { key: "gitBash", label: "git bash" },
  { key: "macTerminal", label: "macOS Terminal" },
];

const EMPTY: TerminalPaths = { cmd: null, powerShell: null, gitBash: null, macTerminal: null };

/** 설정: 터미널 실행 파일 경로. 비워 두면 자동 감지한 경로를 쓴다. */
export function SettingsDialog() {
  const { open: isOpen, saved, detected, hide, save } = useTerminalSettingsVM();
  const [draft, setDraft] = useState<TerminalPaths>(EMPTY);

  useEffect(() => {
    if (isOpen) setDraft(saved ?? EMPTY);
  }, [isOpen, saved]);

  const browse = async (key: keyof TerminalPaths) => {
    const path = await open({ multiple: false, directory: false, title: "터미널 실행 파일 선택" });
    if (typeof path === "string") setDraft({ ...draft, [key]: path });
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={(o) => !o && hide()}>
      <Dialog.Portal>
        <Dialog.Overlay className="overlay" />
        <Dialog.Content className="dialog" style={{ width: 640 }} aria-describedby={undefined}>
          <Dialog.Title asChild>
            <h2>설정 — 터미널 경로</h2>
          </Dialog.Title>
          <div className="dbody">
            <div className="hint" style={{ marginBottom: 10 }}>
              비워 두면 자동 감지한 경로를 씁니다. 터미널로 실행할 때 이 프로그램이 오른쪽 창에서 열립니다.
            </div>
            {FIELDS.map(({ key, label }) => (
              <div className="settings-row" key={key}>
                <span className="label">{label}</span>
                <input
                  value={draft[key] ?? ""}
                  placeholder={detected?.[key] ?? "찾지 못함"}
                  spellCheck={false}
                  onChange={(e) => setDraft({ ...draft, [key]: e.target.value || null })}
                />
                <button onClick={() => browse(key)}>찾기…</button>
              </div>
            ))}
          </div>
          <div className="dfoot">
            <button onClick={() => setDraft(EMPTY)}>전부 자동 감지로</button>
            <span className="spacer" />
            <button onClick={hide}>취소</button>
            <button className="primary" onClick={() => save(draft)}>
              저장
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

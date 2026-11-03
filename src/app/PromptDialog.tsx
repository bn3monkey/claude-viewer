import { useEffect, useRef, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { create } from "zustand";

/** window.prompt / confirm 은 WebView 에서 막혀 있어 직접 그린다. */
type Request =
  | { kind: "prompt"; title: string; label: string; initial: string; resolve: (value: string | null) => void }
  | { kind: "confirm"; title: string; label: string; danger: boolean; resolve: (ok: boolean) => void };

const usePrompt = create<{ request: Request | null }>(() => ({ request: null }));

export function askText(title: string, label: string, initial = ""): Promise<string | null> {
  return new Promise((resolve) => usePrompt.setState({ request: { kind: "prompt", title, label, initial, resolve } }));
}

export function askConfirm(title: string, label: string, danger = true): Promise<boolean> {
  return new Promise((resolve) => usePrompt.setState({ request: { kind: "confirm", title, label, danger, resolve } }));
}

export function PromptDialog() {
  const request = usePrompt((s) => s.request);
  const [value, setValue] = useState("");
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (request?.kind !== "prompt") return;
    setValue(request.initial);
    // 확장자 앞까지만 선택해 바로 이름을 고칠 수 있게 한다.
    requestAnimationFrame(() => {
      const dot = request.initial.lastIndexOf(".");
      input.current?.setSelectionRange(0, dot > 0 ? dot : request.initial.length);
    });
  }, [request]);

  const finish = (ok: boolean) => {
    if (!request) return;
    usePrompt.setState({ request: null });
    if (request.kind === "prompt") request.resolve(ok && value.trim() ? value.trim() : null);
    else request.resolve(ok);
  };

  return (
    <Dialog.Root open={!!request} onOpenChange={(open) => !open && finish(false)}>
      <Dialog.Portal>
        <Dialog.Overlay className="overlay" />
        <Dialog.Content className="dialog" style={{ width: 420 }} aria-describedby={undefined}>
          <Dialog.Title asChild>
            <h2>{request?.title}</h2>
          </Dialog.Title>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              finish(true);
            }}
          >
            <div className="dbody">
              <div style={{ marginBottom: 6, overflowWrap: "anywhere" }}>{request?.label}</div>
              {request?.kind === "prompt" && (
                <input ref={input} autoFocus value={value} onChange={(e) => setValue(e.target.value)} spellCheck={false} />
              )}
            </div>
            <div className="dfoot">
              <button type="button" onClick={() => finish(false)}>
                취소
              </button>
              <button
                type="submit"
                autoFocus={request?.kind === "confirm"}
                className={request?.kind === "confirm" && request.danger ? "danger" : "primary"}
              >
                확인
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

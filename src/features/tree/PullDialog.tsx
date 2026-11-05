import * as Dialog from "@radix-ui/react-dialog";
import { usePullVM } from "../../viewmodel/usePullVM";

/** git pull 이 실패하면 이유를 보여 주고 Reset · 터미널 열기 · 취소 중 고르게 한다. */
export function PullDialog() {
  const { failure, busy, reset, openTerminal, cancel } = usePullVM();

  return (
    <Dialog.Root open={!!failure} onOpenChange={(open) => !open && cancel()}>
      <Dialog.Portal>
        <Dialog.Overlay className="overlay" />
        <Dialog.Content className="dialog" style={{ width: 520 }} aria-describedby={undefined}>
          <Dialog.Title asChild>
            <h2>git pull 실패</h2>
          </Dialog.Title>
          <div className="dbody">
            <div style={{ marginBottom: 6 }}>git pull origin 현재 브랜치 가 실패했습니다.</div>
            <pre className="commit-log" style={{ margin: 0, maxHeight: 200, overflow: "auto" }}>
              {failure?.reason}
            </pre>
            <div style={{ marginTop: 10, color: "var(--red)", overflowWrap: "anywhere" }}>
              Reset 은 git reset --hard HEAD 를 실행합니다. 추적 중인 파일의 커밋하지 않은 변경이 모두 사라지며 되돌릴 수 없습니다. 이후 pull 을 다시 시도합니다.
            </div>
          </div>
          <div className="dfoot">
            <button className="danger" disabled={busy} onClick={() => void reset()}>
              Reset
            </button>
            <button disabled={busy} onClick={() => void openTerminal()}>
              터미널 열기
            </button>
            <button disabled={busy} onClick={cancel}>
              취소
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

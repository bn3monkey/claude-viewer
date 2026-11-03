import type { RateWindow } from "../../generated/entity/RateWindow";

const WINDOW_LABEL: Record<string, string> = {
  five_hour: "세션(5h)",
  seven_day: "이번 주",
};

const fmtTokens = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(n >= 100_000 ? 0 : 1)}k` : String(Math.round(n)));
const fmtReset = (epochSec: number) => {
  const d = new Date(epochSec * 1000);
  const sameDay = d.toDateString() === new Date().toDateString();
  return sameDay ? d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : d.toLocaleDateString();
};
const pct = (ratio: number) => `${Math.round(ratio * 100)}%`;
const level = (ratio: number) => (ratio >= 0.9 ? "hot" : ratio >= 0.7 ? "warm" : "");

type Props = {
  contextTokens: number;
  contextWindow: number;
  costUsd: number;
  outputTokens: number;
  rateWindows: RateWindow[] | null;
  canCompact: boolean;
  onCompact: () => void;
};

/** 세션 상단의 사용량 요약. 컨텍스트 칸을 누르면 /compact. */
export function UsageBar(props: Props) {
  const ctxRatio = props.contextWindow > 0 ? props.contextTokens / props.contextWindow : 0;
  const windows = props.rateWindows ?? [];
  const fable = windows.find((w) => /fable/i.test(w.name));
  const known = windows.filter((w) => w.name in WINDOW_LABEL);
  const others = windows.filter((w) => !(w.name in WINDOW_LABEL) && w !== fable);

  return (
    <div className="usage">
      <button
        className={`meter ${level(ctxRatio)}`}
        title={`컨텍스트 ${fmtTokens(props.contextTokens)} / ${fmtTokens(props.contextWindow)} 토큰 · 누르면 /compact`}
        disabled={!props.canCompact}
        onClick={props.onCompact}
      >
        <span className="bar" style={{ width: pct(Math.min(1, ctxRatio)) }} />
        <span className="label">
          Context {fmtTokens(props.contextTokens)} / {fmtTokens(props.contextWindow)} ({pct(ctxRatio)}) · Compact
        </span>
      </button>
      <span className="stat" title="이 세션에서 쓴 비용과 출력 토큰">
        세션 ${props.costUsd.toFixed(3)} · 출력 {fmtTokens(props.outputTokens)}
      </span>
      {known.map((w) => (
        <Window key={w.name} label={WINDOW_LABEL[w.name]} window={w} />
      ))}
      <Window label="Fable" window={fable ?? null} />
      {others.map((w) => (
        <Window key={w.name} label={w.name} window={w} />
      ))}
      {!props.rateWindows && <span className="hint">사용량은 첫 응답 후 표시됩니다</span>}
    </div>
  );
}

function Window({ label, window }: { label: string; window: RateWindow | null }) {
  if (!window) return <span className="stat hint">{label} —</span>;
  const remaining = Math.max(0, 1 - window.utilization);
  return (
    <span className={`stat meter ${level(window.utilization)}`} title={`${label} 사용 ${pct(window.utilization)} · ${fmtReset(window.resetsAt)} 초기화`}>
      <span className="bar" style={{ width: pct(Math.min(1, window.utilization)) }} />
      <span className="label">
        {label} 남음 {pct(remaining)} · {fmtReset(window.resetsAt)}
      </span>
    </span>
  );
}

/* eslint-disable */
// AgentFeed — visual timeline. Each event has a tiny chart/visualisation inline.

const FEED = [
  { t: "14:32:08", agent: "Atlas",  kind: "trade",     title: "Long NVDA · 120 @ 1,237.40",
    spark: [3,4,4,5,6,5,6,7,8,7,9,10,11,10,12], tone: "up", body: "Sized for 0.8% portfolio risk" },
  { t: "14:31:42", agent: "Vega",   kind: "approval",  title: "Pair trade · long AMD / short INTC",
    pair: { a: [4,5,6,5,7,8,7,9,8,10], b: [10,9,8,9,7,6,7,5,6,4] }, tone: "warn", body: "Spread 2.1σ from mean", action: "Review" },
  { t: "14:30:12", agent: "Echo",   kind: "exit",      title: "Closed SPY mean-revert · +0.42%",
    spark: [5,6,7,8,7,8,9,10,9,10,9,8,7,6,5], tone: "up", body: "Target hit · z-score 0.3" },
  { t: "14:28:55", agent: "Helios", kind: "halt",      title: "Self-paused · drawdown threshold",
    spark: [10,9,9,8,8,7,7,6,5,4,4,3,3,2,2], tone: "down", body: "5 consecutive losing trades" },
  { t: "14:27:01", agent: "Atlas",  kind: "signal",    title: "Volume cluster · NVDA",
    bars: [2,3,3,4,5,7,9,12,8,5,4,3,3,2,2], tone: "info", body: "Confidence rising to 78%" },
  { t: "14:25:33", agent: "Orion",  kind: "rebalance", title: "Trimmed EUR/USD by 18%",
    spark: [8,8,7,7,7,6,6,6,5,5,5,5,5,5,5], tone: "neutral", body: "Carry envelope check" },
  { t: "14:22:19", agent: "Nyx",    kind: "backtest",  title: "Vol Harvest · 3y backtest done",
    spark: [5,5,6,5,6,7,7,8,8,9,10,10,11,12,13], tone: "info", body: "Sharpe 1.62 · proposing live deploy", action: "Review" },
];

function MiniLine({ values, tone="up", w=120, h=28 }) {
  const min = Math.min(...values), max = Math.max(...values), range = max - min || 1;
  const pts = values.map((v,i) => `${(i/(values.length-1))*w},${h - ((v-min)/range)*h*0.85 - 2}`).join(" ");
  const area = `0,${h} ${pts} ${w},${h}`;
  const c = tone === "down" ? "var(--down-500)" : tone === "info" ? "var(--info-500)" : tone === "warn" ? "var(--info-500)" : tone === "neutral" ? "var(--fg-3)" : "var(--up-500)";
  return (
    <svg width={w} height={h} style={{display:"block"}}>
      <polygon points={area} fill={c} fillOpacity="0.10"/>
      <polyline points={pts} fill="none" stroke={c} strokeWidth="1.5"/>
    </svg>
  );
}

function MiniBars({ values, tone="info", w=120, h=28 }) {
  const max = Math.max(...values) || 1;
  const bw = w / values.length - 1;
  const c = tone === "down" ? "var(--down-500)" : tone === "info" ? "var(--info-500)" : "var(--up-500)";
  return (
    <svg width={w} height={h} style={{display:"block"}}>
      {values.map((v,i) => {
        const bh = (v/max) * h * 0.9;
        return <rect key={i} x={i*(bw+1)} y={h-bh} width={bw} height={bh} fill={c} fillOpacity={0.5 + (v/max)*0.5}/>;
      })}
    </svg>
  );
}

function MiniPair({ a, b, w=120, h=28 }) {
  const all = [...a, ...b];
  const min = Math.min(...all), max = Math.max(...all), range = max - min || 1;
  const ptsA = a.map((v,i) => `${(i/(a.length-1))*w},${h - ((v-min)/range)*h*0.85 - 2}`).join(" ");
  const ptsB = b.map((v,i) => `${(i/(b.length-1))*w},${h - ((v-min)/range)*h*0.85 - 2}`).join(" ");
  return (
    <svg width={w} height={h} style={{display:"block"}}>
      <polyline points={ptsA} fill="none" stroke="var(--up-500)" strokeWidth="1.4"/>
      <polyline points={ptsB} fill="none" stroke="var(--down-500)" strokeWidth="1.4"/>
    </svg>
  );
}

function FeedViz({ ev }) {
  if (ev.bars) return <MiniBars values={ev.bars} tone={ev.tone}/>;
  if (ev.pair) return <MiniPair a={ev.pair.a} b={ev.pair.b}/>;
  if (ev.spark) return <MiniLine values={ev.spark} tone={ev.tone}/>;
  return null;
}

function AgentFeed() {
  return (
    <LN.Panel
      title="Agent activity"
      meta="Live · 6 agents"
      actions={<LN.IconButton name="filter" label="Filter"/>}
    >
      <div className="ln-feed">
        {FEED.map((f,i) => (
          <div key={i} className="ln-feed-item">
            <div className="ln-feed-time"><span className="ln-num">{f.t}</span></div>
            <div className="ln-feed-body">
              <div className="ln-feed-line">
                <span className="ln-feed-agent">{f.agent}</span>
                <span className="ln-feed-text">{f.title}</span>
                {f.action && <button className="ln-btn ln-btn--sm ln-btn--secondary" style={{marginLeft:"auto"}}>{f.action}</button>}
              </div>
              {f.body && <div className="ln-feed-sub">{f.body}</div>}
            </div>
            <div className="ln-feed-viz"><FeedViz ev={f}/></div>
          </div>
        ))}
      </div>
    </LN.Panel>
  );
}

window.AgentFeed = AgentFeed;

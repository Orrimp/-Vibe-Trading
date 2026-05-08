/* eslint-disable */
// Fleet summary — visual hero. Multi-strategy equity overlay + KPI chips below.

function FleetEquityOverlay({ height=180 }) {
  const w = 1200, h = height, pad = 12;
  const strategies = STRATEGIES.slice(0, 6);
  // Build all series
  const series = strategies.map(s => s.equity);
  const all = series.flat();
  const min = Math.min(...all), max = Math.max(...all), range = max - min || 1;
  const colors = ["var(--accent)", "var(--up-500)", "var(--down-500)", "var(--info-500)", "#9B86C9", "#D4A574"];

  return (
    <div style={{position:"relative"}}>
      <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{width:"100%", height:h, display:"block"}}>
        {/* gridlines */}
        {[0.25, 0.5, 0.75].map((p,i) => (
          <line key={i} x1={pad} x2={w-pad} y1={pad + p*(h-pad*2)} y2={pad + p*(h-pad*2)}
                stroke="var(--border-1)" strokeWidth="1"/>
        ))}
        {/* lines */}
        {series.map((arr, idx) => {
          const pts = arr.map((v,i) => `${pad + (i/(arr.length-1))*(w-pad*2)},${h - pad - ((v-min)/range)*(h-pad*2)}`).join(" ");
          return <polyline key={idx} points={pts} fill="none" stroke={colors[idx]} strokeWidth="1.5" strokeLinejoin="round" opacity="0.85"/>;
        })}
      </svg>
      <div className="ln-fleet-legend">
        {strategies.map((s,i) => (
          <span className="ln-fleet-legend-item" key={s.id}>
            <span className="ln-leg" style={{background: colors[i]}}/>
            <span>{s.agent}</span>
          </span>
        ))}
      </div>
    </div>
  );
}

function FleetSummary() {
  const stats = [
    { label: "Fleet P/L · today",  value: "+$22,873", tone: "up", sub: "+1.84%" },
    { label: "Active agents",      value: "4 / 6",    sub: "1 paused · 1 review" },
    { label: "Live capital",       value: "$1.24M",   sub: "62% deployed" },
    { label: "Avg confidence",     value: "64%",      sub: "↑ 6pt vs yesterday" },
    { label: "Pending approvals",  value: "3",        tone: "warn", sub: "Oldest 4m ago" },
  ];
  return (
    <LN.Panel
      title="Fleet"
      meta="6 strategies · live equity vs backtest"
      actions={<>
        <div className="ln-segmented">
          {["1D","1W","1M","3M","YTD"].map((p,i) => (
            <button key={p} className={`ln-seg${i===2?" ln-seg--active":""}`}>{p}</button>
          ))}
        </div>
      </>}
    >
      <div className="ln-fleet-chart">
        <FleetEquityOverlay/>
      </div>
      <div className="ln-fleet">
        {stats.map((s,i) => (
          <div className="ln-fleet-cell" key={i}>
            <div className="ln-l">{s.label}</div>
            <div className={`ln-num ln-num-big${s.tone==="up"?" ln-num--up":""}${s.tone==="down"?" ln-num--down":""}`}>{s.value}</div>
            <div className="ln-fleet-sub">{s.sub}</div>
          </div>
        ))}
      </div>
    </LN.Panel>
  );
}

window.FleetSummary = FleetSummary;

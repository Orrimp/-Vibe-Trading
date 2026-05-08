/* eslint-disable */
// ApprovalQueue — each pending trade shows a tiny signal chart explaining the agent's reasoning.

const APPROVALS = [
  { id: "a1", agent: "Vega",  side: "buy",  qty: 200, sym: "AMD",  px: 167.42, type: "limit",
    reason: "Pair spread vs INTC at 2.1σ from mean", conf: 0.71,
    viz: { kind: "spread", values: [0.5,0.7,1.0,1.3,1.6,1.4,1.2,1.5,1.8,2.0,2.1] } },
  { id: "a2", agent: "Vega",  side: "sell", qty: 240, sym: "INTC", px: 30.18,  type: "limit",
    reason: "Other leg of AMD/INTC pair", conf: 0.71,
    viz: { kind: "spread", values: [-0.5,-0.7,-1.0,-1.3,-1.6,-1.4,-1.2,-1.5,-1.8,-2.0,-2.1] } },
  { id: "a3", agent: "Nyx",   side: "sell", qty: 5,   sym: "SPX 5800P", px: 12.40, type: "limit",
    reason: "Implied vs realized vol gap", conf: 0.62,
    viz: { kind: "twoline", a: [22,23,24,25,26,28,29,30,31,32,32], b: [18,18,17,17,16,16,15,15,14,14,14] } },
];

function ApprovalViz({ v, side }) {
  const w = 120, h = 36;
  if (v.kind === "spread") {
    const arr = v.values;
    const max = Math.max(...arr.map(Math.abs)) || 1;
    const bw = w / arr.length - 1;
    return (
      <svg width={w} height={h} style={{display:"block"}}>
        <line x1="0" x2={w} y1={h/2} y2={h/2} stroke="var(--border-2)" strokeDasharray="2 2"/>
        {arr.map((v,i) => {
          const bh = (Math.abs(v)/max) * (h/2) * 0.9;
          const y = v >= 0 ? h/2 - bh : h/2;
          const c = v >= 0 ? "var(--up-500)" : "var(--down-500)";
          return <rect key={i} x={i*(bw+1)} y={y} width={bw} height={bh} fill={c} fillOpacity={0.4 + (Math.abs(v)/max)*0.6}/>;
        })}
      </svg>
    );
  }
  if (v.kind === "twoline") {
    const all = [...v.a, ...v.b];
    const min = Math.min(...all), max = Math.max(...all), range = max - min || 1;
    const toPts = arr => arr.map((x,i) => `${(i/(arr.length-1))*w},${h - ((x-min)/range)*h*0.85 - 2}`).join(" ");
    return (
      <svg width={w} height={h} style={{display:"block"}}>
        <polyline points={toPts(v.a)} fill="none" stroke="var(--down-500)" strokeWidth="1.4"/>
        <polyline points={toPts(v.b)} fill="none" stroke="var(--up-500)" strokeWidth="1.4"/>
      </svg>
    );
  }
  return null;
}

function ApprovalQueue() {
  return (
    <LN.Panel
      title="Approvals"
      meta={`${APPROVALS.length} pending`}
      actions={<button className="ln-btn ln-btn--sm ln-btn--ghost">Approve all</button>}
    >
      <div className="ln-approvals">
        {APPROVALS.map(a => (
          <div className="ln-approval" key={a.id}>
            <div className="ln-approval-h">
              <div className="ln-approval-agent">
                <div className="ln-agent-avatar"><img src="../../assets/brand/lumen-mark.svg" width="11" height="13" alt=""/></div>
                <span>{a.agent}</span>
              </div>
              <span className={`ln-tag ln-tag--${a.side==="buy"?"up":"down"}`}>{a.side.toUpperCase()}</span>
            </div>
            <div className="ln-approval-trade">
              <span className="ln-num">{a.qty}</span>
              <span className="ln-sym">{a.sym}</span>
              <span className="ln-l">@</span>
              <span className="ln-num">${a.px.toFixed(2)}</span>
              <span className="ln-l ln-approval-type">{a.type}</span>
            </div>
            <div className="ln-approval-viz">
              <ApprovalViz v={a.viz} side={a.side}/>
            </div>
            <div className="ln-approval-reason">{a.reason}</div>
            <div className="ln-approval-foot">
              <div className="ln-conf" style={{flex:1}}>
                <span className="ln-l">Conf</span>
                <div className="ln-conf-bar"><div className="ln-conf-fill" style={{width:(a.conf*100)+"%"}}/></div>
                <span className="ln-num ln-l">{Math.round(a.conf*100)}%</span>
              </div>
              <div className="ln-approval-actions">
                <button className="ln-btn ln-btn--sm ln-btn--ghost">Skip</button>
                <button className="ln-btn ln-btn--sm ln-btn--primary">Approve</button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </LN.Panel>
  );
}

window.ApprovalQueue = ApprovalQueue;

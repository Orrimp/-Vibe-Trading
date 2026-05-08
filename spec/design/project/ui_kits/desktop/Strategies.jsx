/* eslint-disable */
// Strategies — visual-first AI agent grid. Each card is a small chart dashboard for one strategy.

const STRATEGIES = [
  { id: "moma",  name: "Momentum · Tech",      agent: "Atlas",   status: "running",      pnl: 12480.30, pnlPct: 4.21, sharpe: 1.84, trades: 24, conf: 0.78,
    equity: [100,101,103,102,104,107,106,109,112,114,113,116,118,120,122,121,124,127,130,132,135,138,141,143,146,150],
    wins: 15, losses: 9, btSharpe: 1.62, btCagr: 17.6, approval: false },
  { id: "mrev",  name: "Mean Reversion · ETFs", agent: "Echo",    status: "running",     pnl: 3204.80,  pnlPct: 1.12, sharpe: 1.21, trades: 18, conf: 0.62,
    equity: [100,100,99,101,100,102,101,103,102,104,103,105,104,106,105,107,106,108,107,109,108,110,109,111,110,112],
    wins: 11, losses: 7, btSharpe: 1.18, btCagr: 9.4, approval: false },
  { id: "pairs", name: "Pairs · Semis",         agent: "Vega",    status: "needs-review", pnl: -842.40, pnlPct: -0.34, sharpe: 0.42, trades: 11, conf: 0.51,
    equity: [100,101,102,101,100,101,99,100,98,99,97,98,96,97,95,96,94,95,93,94,93,94,92,93,92,93],
    wins: 5, losses: 6, btSharpe: 0.71, btCagr: 4.2, approval: true },
  { id: "bsig",  name: "Breakout Signals",      agent: "Helios",  status: "paused",      pnl: 6210.00,  pnlPct: 2.40, sharpe: 1.55, trades: 9,  conf: 0.69,
    equity: [100,101,102,103,103,104,105,106,107,108,108,109,110,111,112,112,113,114,115,116,116,117,118,119,120,121],
    wins: 6, losses: 3, btSharpe: 1.42, btCagr: 13.8, approval: false },
  { id: "vol",   name: "Vol Harvest · Index",   agent: "Nyx",     status: "backtesting", pnl: 0,        pnlPct: 0,    sharpe: 0,    trades: 0,  conf: 0.0,
    equity: [100,101,100,102,101,103,102,104,103,105,104,106,105,107,106,108,107,109,108,110,109,111,110,112,111,113],
    wins: 0, losses: 0, btSharpe: 1.62, btCagr: 11.2, approval: false },
  { id: "drift", name: "Drift Capture · FX",    agent: "Orion",   status: "running",     pnl: 1820.50,  pnlPct: 0.71, sharpe: 1.04, trades: 32, conf: 0.58,
    equity: [100,100,101,100,102,101,103,102,104,103,105,104,106,105,107,106,108,107,109,108,110,109,111,110,112,111],
    wins: 18, losses: 14, btSharpe: 1.08, btCagr: 7.9, approval: false },
];

function statusTone(s) {
  if (s === "running") return "up";
  if (s === "needs-review") return "down";
  if (s === "paused") return "warn";
  if (s === "backtesting") return "info";
  return "neutral";
}
function statusLabel(s) {
  return ({ "running": "Running", "needs-review": "Needs review", "paused": "Paused", "backtesting": "Backtesting" })[s] || s;
}

/* ---------- Chart primitives ---------- */

// Equity area chart with optional benchmark dashed line
function EquityChart({ live, bt, tone="up", height=72 }) {
  const w = 280, h = height, pad = 4;
  const all = [...(live||[]), ...(bt||[])];
  const min = Math.min(...all), max = Math.max(...all), range = max - min || 1;
  const toPts = arr => arr.map((v,i) => `${pad + (i/(arr.length-1))*(w-pad*2)},${h - pad - ((v-min)/range)*(h-pad*2)}`).join(" ");
  const livePts = toPts(live);
  const btPts = bt ? toPts(bt) : null;
  const area = `${pad},${h-pad} ${livePts} ${w-pad},${h-pad}`;
  const c = tone === "down" ? "var(--down-500)" : "var(--up-500)";
  const gid = "g" + Math.random().toString(36).slice(2,7);
  return (
    <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{width:"100%", height:h, display:"block"}}>
      <defs>
        <linearGradient id={gid} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor={c} stopOpacity="0.20"/>
          <stop offset="100%" stopColor={c} stopOpacity="0"/>
        </linearGradient>
      </defs>
      <polygon points={area} fill={`url(#${gid})`}/>
      {btPts && <polyline points={btPts} fill="none" stroke="var(--fg-3)" strokeWidth="1" strokeDasharray="3 2" opacity="0.6"/>}
      <polyline points={livePts} fill="none" stroke={c} strokeWidth="1.5" strokeLinejoin="round"/>
    </svg>
  );
}

// Win/loss bar
function WinLossBar({ wins, losses }) {
  const total = wins + losses || 1;
  const wp = (wins/total)*100, lp = (losses/total)*100;
  return (
    <div className="ln-wlbar">
      <div className="ln-wlbar-track">
        <div className="ln-wlbar-w" style={{width: wp+"%"}}/>
        <div className="ln-wlbar-l" style={{width: lp+"%"}}/>
      </div>
      <div className="ln-wlbar-labels">
        <span><span className="ln-num ln-num--up">{wins}</span><span className="ln-l"> wins</span></span>
        <span><span className="ln-l">losses </span><span className="ln-num ln-num--down">{losses}</span></span>
      </div>
    </div>
  );
}

// Confidence radial gauge
function ConfGauge({ value, size=48 }) {
  const r = size/2 - 4, cx = size/2, cy = size/2;
  const circ = 2 * Math.PI * r;
  const off = circ * (1 - value);
  return (
    <div className="ln-gauge" style={{width:size, height:size}}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle cx={cx} cy={cy} r={r} fill="none" stroke="var(--panel-sunken)" strokeWidth="3"/>
        <circle cx={cx} cy={cy} r={r} fill="none" stroke="var(--accent)" strokeWidth="3" strokeLinecap="round"
                strokeDasharray={circ} strokeDashoffset={off}
                transform={`rotate(-90 ${cx} ${cy})`}/>
      </svg>
      <div className="ln-gauge-label"><span className="ln-num">{Math.round(value*100)}</span></div>
    </div>
  );
}

/* ---------- Card ---------- */

function StrategyCard({ s, selected, onSelect }) {
  const tone = s.pnl >= 0 ? "up" : "down";
  return (
    <div className={`ln-strat ${selected===s.id?"ln-strat--active":""}`} onClick={()=>onSelect&&onSelect(s.id)}>
      <div className="ln-strat-h">
        <div className="ln-strat-agent">
          <div className="ln-agent-avatar"><img src="../../assets/brand/lumen-mark.svg" width="14" height="16" alt=""/></div>
          <div>
            <div className="ln-strat-name">{s.name}</div>
            <div className="ln-l">Agent · {s.agent}</div>
          </div>
        </div>
        <LN.Tag tone={statusTone(s.status)} dot>{statusLabel(s.status)}</LN.Tag>
      </div>

      <div className="ln-strat-pnlrow">
        <div>
          <div className="ln-l">P/L · today</div>
          <div className={`ln-num ln-num-big ln-num--${tone}`}>{s.pnl>=0?"+":"−"}${Math.abs(s.pnl).toLocaleString(undefined,{minimumFractionDigits:0,maximumFractionDigits:0})}</div>
          <LN.Num tone={tone} sign suffix="%">{s.pnlPct}</LN.Num>
        </div>
        <ConfGauge value={s.conf}/>
      </div>

      <div className="ln-strat-chart">
        <div className="ln-strat-chart-h">
          <span className="ln-l">Live equity</span>
          <span className="ln-strat-legend">
            <span className="ln-leg ln-leg--live"/><span className="ln-l">live</span>
            <span className="ln-leg ln-leg--bt"/><span className="ln-l">backtest</span>
          </span>
        </div>
        <EquityChart live={s.equity} bt={s.equity.map((v,i)=>v - 1.5 + Math.sin(i*0.5)*1.2)} tone={tone}/>
      </div>

      <div className="ln-strat-foot">
        <WinLossBar wins={s.wins} losses={s.losses}/>
        <div className="ln-strat-bt">
          <div className="ln-bt-pill" title="Backtest Sharpe">
            <span className="ln-l">BT Sharpe</span>
            <span className="ln-num">{s.btSharpe.toFixed(2)}</span>
          </div>
          <div className="ln-bt-pill" title="Backtest CAGR">
            <span className="ln-l">CAGR</span>
            <span className="ln-num">{s.btCagr.toFixed(1)}%</span>
          </div>
        </div>
      </div>

      {s.approval && (
        <div className="ln-strat-approve">
          <LN.Icon name="bell" size={12}/>
          <span>2 trades awaiting your approval</span>
          <button className="ln-btn ln-btn--sm ln-btn--primary">Review</button>
        </div>
      )}
    </div>
  );
}

function Strategies({ selected, onSelect }) {
  return (
    <LN.Panel
      title="Strategies"
      meta={`${STRATEGIES.filter(s=>s.status==="running").length} running · ${STRATEGIES.filter(s=>s.status==="needs-review").length} need review`}
      actions={<>
        <button className="ln-btn ln-btn--sm ln-btn--secondary"><LN.Icon name="plus" size={12}/>New strategy</button>
        <LN.IconButton name="filter" label="Filter"/>
      </>}
    >
      <div className="ln-strat-grid">
        {STRATEGIES.map(s => <StrategyCard key={s.id} s={s} selected={selected} onSelect={onSelect}/>)}
      </div>
    </LN.Panel>
  );
}

window.Strategies = Strategies;
window.STRATEGIES = STRATEGIES;

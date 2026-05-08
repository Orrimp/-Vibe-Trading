/* eslint-disable */
// StrategyDetail — deep view of a single strategy: parameters, equity curve, recent trades.

function EquityCurve({ tone="up" }) {
  const w = 800, h = 160, pad = 12;
  // synthetic equity curve, gently up
  const pts = Array.from({length: 60}, (_,i) => {
    const drift = i * 0.45;
    const noise = Math.sin(i*0.6)*3 + Math.cos(i*0.31)*4 + (Math.random()-0.5)*2;
    return drift + noise;
  });
  const min = Math.min(...pts), max = Math.max(...pts), range = max - min || 1;
  const xy = pts.map((v,i) => `${pad + (i/(pts.length-1))*(w-pad*2)},${h - pad - ((v-min)/range)*(h-pad*2)}`).join(" ");
  const area = `${pad},${h-pad} ${xy} ${w-pad},${h-pad}`;
  const c = tone === "down" ? "var(--down-500)" : "var(--up-500)";
  return (
    <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{width:"100%", height:"160px", display:"block"}}>
      <defs>
        <linearGradient id="eqg" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor={c} stopOpacity="0.18"/>
          <stop offset="100%" stopColor={c} stopOpacity="0"/>
        </linearGradient>
      </defs>
      <polygon points={area} fill="url(#eqg)"/>
      <polyline points={xy} fill="none" stroke={c} strokeWidth="1.5" strokeLinejoin="round"/>
    </svg>
  );
}

const RECENT_TRADES = [
  { t: "14:32", side: "buy",  sym: "NVDA", qty: 120, px: 1237.40, pnl: null },
  { t: "13:58", side: "sell", sym: "NVDA", qty: 80,  px: 1235.10, pnl: 184.00 },
  { t: "12:14", side: "buy",  sym: "NVDA", qty: 80,  px: 1232.80, pnl: null },
  { t: "11:02", side: "sell", sym: "AMD",  qty: 220, px: 168.30,  pnl: 412.50 },
  { t: "10:31", side: "buy",  sym: "AMD",  qty: 220, px: 166.42,  pnl: null },
];

function StrategyDetail({ strategy: s }) {
  const tone = s.pnl >= 0 ? "up" : "down";
  return (
    <LN.Panel
      title={s.name}
      meta={`Agent · ${s.agent}`}
      actions={<>
        <button className="ln-btn ln-btn--sm ln-btn--ghost"><LN.Icon name="settings" size={12}/>Params</button>
        <button className="ln-btn ln-btn--sm ln-btn--secondary"><LN.Icon name="book" size={12}/>Backtest</button>
        <button className="ln-btn ln-btn--sm ln-btn--secondary">{s.status==="paused"?"Resume":"Pause"}</button>
      </>}
    >
      <div className="ln-stratd">
        <div className="ln-stratd-kpis">
          <div><span className="ln-l">P/L · today</span>
            <span className={`ln-num ln-num-big ln-num--${tone}`}>{s.pnl>=0?"+":"−"}${Math.abs(s.pnl).toLocaleString(undefined,{minimumFractionDigits:2,maximumFractionDigits:2})}</span>
          </div>
          <div><span className="ln-l">Sharpe · 30d</span><span className="ln-num ln-num-big">{s.sharpe.toFixed(2)}</span></div>
          <div><span className="ln-l">Win rate</span><span className="ln-num ln-num-big">62%</span></div>
          <div><span className="ln-l">Max drawdown</span><span className="ln-num ln-num-big ln-num--down">−2.4%</span></div>
          <div><span className="ln-l">Confidence</span>
            <div className="ln-conf" style={{marginTop:4}}>
              <div className="ln-conf-bar" style={{flex:1}}><div className="ln-conf-fill" style={{width:(s.conf*100)+"%"}}/></div>
              <span className="ln-num">{Math.round(s.conf*100)}%</span>
            </div>
          </div>
        </div>

        <div className="ln-stratd-curve">
          <div className="ln-stratd-curve-h">
            <span className="ln-l">Equity curve · 60d</span>
            <span className="ln-num ln-num--up">+12.4%</span>
          </div>
          <EquityCurve tone={tone}/>
        </div>

        <div className="ln-stratd-trades">
          <div className="ln-l" style={{padding:"8px 12px 4px"}}>Recent trades</div>
          <table className="ln-table">
            <thead><tr>
              <th className="ln-td-sym">Time</th><th>Side</th><th className="ln-td-sym">Symbol</th>
              <th>Qty</th><th>Price</th><th>P/L</th>
            </tr></thead>
            <tbody>
              {RECENT_TRADES.map((t,i) => (
                <tr key={i}>
                  <td className="ln-td-sym">{t.t}</td>
                  <td><span className={`ln-tag ln-tag--${t.side==="buy"?"up":"down"}`}>{t.side.toUpperCase()}</span></td>
                  <td className="ln-td-sym"><span className="ln-sym">{t.sym}</span></td>
                  <td>{t.qty}</td>
                  <td>${t.px.toFixed(2)}</td>
                  <td className={t.pnl==null?"":(t.pnl>=0?"ln-num--up":"ln-num--down")}>
                    {t.pnl==null ? "—" : (t.pnl>=0?"+":"−")+"$"+Math.abs(t.pnl).toFixed(2)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </LN.Panel>
  );
}

window.StrategyDetail = StrategyDetail;

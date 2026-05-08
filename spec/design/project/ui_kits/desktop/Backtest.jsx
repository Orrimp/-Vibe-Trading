/* eslint-disable */
// Backtest — runner panel + result panel.

const { useState: useStateBT } = React;

function BacktestRunner() {
  const [strat, setStrat] = useStateBT("moma");
  return (
    <LN.Panel title="New backtest" meta="Configure & run">
      <div className="ln-bt-form">
        <div className="ln-field">
          <label>Strategy</label>
          <div className="ln-input-group">
            <select className="ln-select" value={strat} onChange={e=>setStrat(e.target.value)}>
              {STRATEGIES.map(s => <option key={s.id} value={s.id}>{s.name}</option>)}
            </select>
          </div>
        </div>
        <div className="ln-field-row">
          <div className="ln-field"><label>From</label>
            <div className="ln-input-group"><input defaultValue="2022-01-01"/></div></div>
          <div className="ln-field"><label>To</label>
            <div className="ln-input-group"><input defaultValue="2025-01-01"/></div></div>
        </div>
        <div className="ln-field-row">
          <div className="ln-field"><label>Capital</label>
            <div className="ln-input-group"><span className="ln-input-pre">$</span><input defaultValue="100,000"/></div></div>
          <div className="ln-field"><label>Slippage (bps)</label>
            <div className="ln-input-group"><input defaultValue="3"/></div></div>
        </div>
        <div className="ln-field">
          <label>Universe</label>
          <div className="ln-bt-chips">
            {["S&P 500","NASDAQ 100","Russell 2000","Custom…"].map((c,i) => (
              <span key={i} className={`ln-chip${i===0?" ln-chip--on":""}`}>{c}</span>
            ))}
          </div>
        </div>
        <div className="ln-field">
          <label>Walk-forward</label>
          <div className="ln-segmented" style={{width:"100%"}}>
            {["Off","6m / 1m","1y / 3m"].map((m,i) => (
              <button key={i} className={`ln-seg${i===1?" ln-seg--active":""}`} style={{flex:1}}>{m}</button>
            ))}
          </div>
        </div>
        <button className="ln-btn ln-btn--lg ln-btn--primary"><LN.Icon name="trending" size={14}/>Run backtest</button>
        <div className="ln-bt-hint">Estimated runtime · 1m 14s · uses agent <strong>Atlas</strong></div>
      </div>
    </LN.Panel>
  );
}

function BacktestResult() {
  const w = 800, h = 240, pad = 16;
  const pts = Array.from({length:200}, (_,i) => {
    const trend = i*0.4;
    const wave = Math.sin(i*0.13)*8 + Math.cos(i*0.07)*5;
    return trend + wave + (Math.random()-0.5)*3;
  });
  const min = Math.min(...pts), max = Math.max(...pts), range = max - min || 1;
  const xy = pts.map((v,i) => `${pad + (i/(pts.length-1))*(w-pad*2)},${h - pad - ((v-min)/range)*(h-pad*2)}`).join(" ");
  const area = `${pad},${h-pad} ${xy} ${w-pad},${h-pad}`;

  const dd = Array.from({length:200}, (_,i) => Math.min(0, -Math.abs(Math.sin(i*0.09))*4 - (Math.random()*2)));
  const ddmin = Math.min(...dd);
  const ddxy = dd.map((v,i) => `${pad + (i/(dd.length-1))*(w-pad*2)},${pad + ((v-0)/(ddmin-0))*(h-pad*2)}`).join(" ");
  const ddarea = `${pad},${pad} ${ddxy} ${w-pad},${pad}`;

  return (
    <LN.Panel
      title="Backtest result"
      meta="Momentum · Tech · 2022–2025"
      actions={<>
        <button className="ln-btn ln-btn--sm ln-btn--ghost"><LN.Icon name="book" size={12}/>Export</button>
        <button className="ln-btn ln-btn--sm ln-btn--primary">Deploy live</button>
      </>}
    >
      <div className="ln-bt-result">
        <div className="ln-bt-kpis">
          <div><span className="ln-l">Total return</span><span className="ln-num ln-num-big ln-num--up">+62.4%</span></div>
          <div><span className="ln-l">CAGR</span><span className="ln-num ln-num-big">17.6%</span></div>
          <div><span className="ln-l">Sharpe</span><span className="ln-num ln-num-big">1.84</span></div>
          <div><span className="ln-l">Max DD</span><span className="ln-num ln-num-big ln-num--down">−9.2%</span></div>
          <div><span className="ln-l">Win rate</span><span className="ln-num ln-num-big">61%</span></div>
          <div><span className="ln-l">Trades</span><span className="ln-num ln-num-big">412</span></div>
        </div>

        <div className="ln-bt-chartwrap">
          <div className="ln-bt-chart-h"><span className="ln-l">Equity curve</span><span className="ln-l">vs SPY</span></div>
          <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{width:"100%", height:"240px", display:"block"}}>
            <defs>
              <linearGradient id="bteq" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="var(--up-500)" stopOpacity="0.18"/>
                <stop offset="100%" stopColor="var(--up-500)" stopOpacity="0"/>
              </linearGradient>
            </defs>
            <polygon points={area} fill="url(#bteq)"/>
            <polyline points={xy} fill="none" stroke="var(--up-500)" strokeWidth="1.5"/>
          </svg>
          <div className="ln-bt-chart-h" style={{marginTop:8}}><span className="ln-l">Drawdown</span></div>
          <svg viewBox={`0 0 ${w} ${h*0.5}`} preserveAspectRatio="none" style={{width:"100%", height:"100px", display:"block"}}>
            <polygon points={ddarea} fill="var(--down-500)" fillOpacity="0.18"/>
            <polyline points={ddxy} fill="none" stroke="var(--down-500)" strokeWidth="1.2"/>
          </svg>
        </div>
      </div>
    </LN.Panel>
  );
}

window.BacktestRunner = BacktestRunner;
window.BacktestResult = BacktestResult;

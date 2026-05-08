/* eslint-disable */
// Chart panel — stylized candlestick frame, not real data.

function generateCandles(n, seed = 1) {
  let p = 1230;
  const out = [];
  let s = seed;
  const rand = () => (s = (s * 9301 + 49297) % 233280) / 233280;
  for (let i = 0; i < n; i++) {
    const open = p;
    const move = (rand() - 0.45) * 8;
    const close = open + move;
    const high = Math.max(open, close) + rand() * 4;
    const low = Math.min(open, close) - rand() * 4;
    out.push({ open, close, high, low });
    p = close;
  }
  return out;
}

function Chart({ symbol = "NVDA" }) {
  const candles = generateCandles(60, 7);
  const W = 720, H = 280, padL = 40, padR = 12, padT = 18, padB = 28;
  const all = candles.flatMap(c => [c.high, c.low]);
  const max = Math.max(...all), min = Math.min(...all);
  const range = max - min || 1;
  const cw = (W - padL - padR) / candles.length;
  const yFor = v => padT + (1 - (v - min) / range) * (H - padT - padB);

  const last = candles[candles.length - 1].close;
  const first = candles[0].open;
  const chg = ((last - first) / first) * 100;
  const tone = chg >= 0 ? "up" : "down";

  return (
    <LN.Panel
      title={symbol}
      meta="1D · 5m · NASDAQ"
      actions={<>
        <div className="ln-segmented">
          {["1m","5m","15m","1h","1D"].map((t,i)=>(
            <button key={t} className={`ln-seg${i===1?" ln-seg--active":""}`}>{t}</button>
          ))}
        </div>
        <LN.IconButton name="settings" label="Chart settings"/>
        <LN.IconButton name="more" label="More"/>
      </>}
    >
      <div className="ln-chart-head">
        <div className="ln-chart-price">
          <span className="ln-num ln-num-big">{last.toFixed(2)}</span>
          <LN.Tag tone={tone} dot>
            <LN.Num tone={tone} sign suffix="%">{Number(chg.toFixed(2))}</LN.Num>
          </LN.Tag>
        </div>
        <div className="ln-chart-stats">
          <div><span className="ln-l">Open</span><span className="ln-num">{candles[0].open.toFixed(2)}</span></div>
          <div><span className="ln-l">High</span><span className="ln-num">{Math.max(...candles.map(c=>c.high)).toFixed(2)}</span></div>
          <div><span className="ln-l">Low</span><span className="ln-num">{Math.min(...candles.map(c=>c.low)).toFixed(2)}</span></div>
          <div><span className="ln-l">Vol</span><span className="ln-num">42.1M</span></div>
        </div>
      </div>
      <div className="ln-chart-stage">
        <svg width="100%" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none">
          {/* horizontal grid */}
          {[0.25,0.5,0.75].map(p => (
            <line key={p} x1={padL} x2={W-padR} y1={padT+p*(H-padT-padB)} y2={padT+p*(H-padT-padB)}
                  stroke="var(--border-1)" strokeDasharray="2 4"/>
          ))}
          {/* y labels */}
          {[0,0.25,0.5,0.75,1].map(p => (
            <text key={p} x={padL-6} y={padT+p*(H-padT-padB)+3}
                  fontSize="9" fontFamily="JetBrains Mono" fill="var(--fg-3)" textAnchor="end">
              {(max - p*range).toFixed(2)}
            </text>
          ))}
          {/* candles */}
          {candles.map((c,i) => {
            const x = padL + i*cw + cw*0.5;
            const up = c.close >= c.open;
            const bodyT = yFor(Math.max(c.open, c.close));
            const bodyB = yFor(Math.min(c.open, c.close));
            const color = up ? "var(--up-500)" : "var(--down-500)";
            return (
              <g key={i}>
                <line x1={x} x2={x} y1={yFor(c.high)} y2={yFor(c.low)} stroke={color} strokeWidth="1"/>
                <rect x={x - cw*0.32} y={bodyT} width={cw*0.64} height={Math.max(1,bodyB-bodyT)} fill={color}/>
              </g>
            );
          })}
          {/* last price line */}
          <line x1={padL} x2={W-padR} y1={yFor(last)} y2={yFor(last)} stroke="var(--accent)" strokeDasharray="3 3" strokeWidth="1"/>
          <rect x={W-padR-44} y={yFor(last)-9} width="44" height="18" fill="var(--accent)" rx="2"/>
          <text x={W-padR-22} y={yFor(last)+4} fontSize="10" fontFamily="JetBrains Mono" fill="#fff" textAnchor="middle">{last.toFixed(2)}</text>
        </svg>
      </div>
    </LN.Panel>
  );
}

window.Chart = Chart;

/* eslint-disable */
// Watchlist panel

const WATCHLIST = [
  { sym: "NVDA", name: "NVIDIA Corp",        last: 1240.50, chg: 1.24,  vol: "42.1M", spark: [3,4,2,5,4,6,5,7,6,8,7,9] },
  { sym: "AAPL", name: "Apple Inc.",         last: 214.08, chg: -0.83, vol: "38.5M", spark: [6,5,7,6,5,4,5,4,3,4,3,2] },
  { sym: "TSLA", name: "Tesla, Inc.",        last: 198.20, chg: 0.12,  vol: "61.2M", spark: [4,5,4,5,4,5,4,5,5,5,5,5] },
  { sym: "MSFT", name: "Microsoft Corp",     last: 431.77, chg: -1.04, vol: "28.0M", spark: [7,7,6,6,5,5,4,5,4,4,3,3] },
  { sym: "GOOG", name: "Alphabet Inc.",      last: 178.50, chg: 0.41,  vol: "19.4M", spark: [4,4,4,5,5,5,6,6,6,6,7,7] },
  { sym: "AMD",  name: "AMD",                last: 168.20, chg: 2.14,  vol: "55.1M", spark: [3,3,4,4,5,5,6,6,7,8,9,10] },
  { sym: "META", name: "Meta Platforms",     last: 510.40, chg: -0.22, vol: "14.2M", spark: [6,6,7,6,6,5,5,5,5,4,5,5] },
  { sym: "SPY",  name: "S&P 500 ETF",        last: 552.13, chg: 0.18,  vol: "62.8M", spark: [5,5,5,6,5,6,6,6,6,7,6,7] },
];

function Sparkline({ values, tone = "up" }) {
  const w = 60, h = 18, max = Math.max(...values), min = Math.min(...values);
  const range = max - min || 1;
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w;
    const y = h - ((v - min) / range) * h;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const color = tone === "down" ? "var(--down-500)" : "var(--up-500)";
  return (
    <svg width={w} height={h} style={{ display: "block" }}>
      <polyline points={pts} fill="none" stroke={color} strokeWidth="1.25" strokeLinejoin="round" strokeLinecap="round"/>
    </svg>
  );
}

function Watchlist({ selected, onSelect }) {
  return (
    <LN.Panel
      title="Watchlist"
      meta="Tech · 8 symbols"
      actions={<>
        <LN.IconButton name="filter" label="Filter"/>
        <LN.IconButton name="plus" label="Add"/>
        <LN.IconButton name="more" label="More"/>
      </>}
    >
      <table className="ln-table">
        <thead>
          <tr>
            <th style={{textAlign:"left"}}>Symbol</th>
            <th>Last</th>
            <th>Δ %</th>
            <th>Trend</th>
            <th>Vol</th>
          </tr>
        </thead>
        <tbody>
          {WATCHLIST.map(r => (
            <tr key={r.sym}
                className={selected === r.sym ? "ln-tr--active" : ""}
                onClick={() => onSelect && onSelect(r.sym)}>
              <td className="ln-td-sym">
                <div className="ln-sym-cell">
                  <span className="ln-sym">{r.sym}</span>
                  <span className="ln-sym-name">{r.name}</span>
                </div>
              </td>
              <td><LN.Num>{r.last.toLocaleString(undefined,{minimumFractionDigits:2,maximumFractionDigits:2})}</LN.Num></td>
              <td><LN.Num tone={r.chg>=0?"up":"down"} sign>{r.chg}</LN.Num></td>
              <td><Sparkline values={r.spark} tone={r.chg>=0?"up":"down"}/></td>
              <td><LN.Num>{r.vol}</LN.Num></td>
            </tr>
          ))}
        </tbody>
      </table>
    </LN.Panel>
  );
}

window.Watchlist = Watchlist;

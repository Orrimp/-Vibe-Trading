/* eslint-disable */
// Positions panel — dense, sortable-feeling table.

const POSITIONS = [
  { sym: "NVDA", qty: 120, avg: 1180.20, last: 1240.50, pnl:  7236.00, pnlPct: 5.10 },
  { sym: "AAPL", qty:  80, avg:  220.10, last:  214.08, pnl:  -481.60, pnlPct: -2.74 },
  { sym: "AMD",  qty: 200, avg:  155.00, last:  168.20, pnl:  2640.00, pnlPct: 8.52 },
  { sym: "MSFT", qty:  40, avg:  445.50, last:  431.77, pnl:  -549.20, pnlPct: -3.08 },
  { sym: "SPY",  qty:  60, avg:  548.00, last:  552.13, pnl:   247.80, pnlPct: 0.75 },
];

function Positions() {
  const totalPnl = POSITIONS.reduce((s,p)=>s+p.pnl, 0);
  return (
    <LN.Panel
      title="Positions"
      meta={`${POSITIONS.length} open · Net ${totalPnl >= 0 ? "+" : "−"}$${Math.abs(totalPnl).toLocaleString(undefined,{minimumFractionDigits:2,maximumFractionDigits:2})}`}
      actions={<LN.IconButton name="more" label="More"/>}
    >
      <table className="ln-table">
        <thead>
          <tr>
            <th style={{textAlign:"left"}}>Symbol</th>
            <th>Qty</th>
            <th>Avg</th>
            <th>Last</th>
            <th>P/L</th>
            <th>P/L %</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {POSITIONS.map(p => (
            <tr key={p.sym}>
              <td className="ln-td-sym"><span className="ln-sym">{p.sym}</span></td>
              <td><LN.Num>{p.qty}</LN.Num></td>
              <td><LN.Num>{p.avg.toFixed(2)}</LN.Num></td>
              <td><LN.Num>{p.last.toFixed(2)}</LN.Num></td>
              <td><LN.Num tone={p.pnl>=0?"up":"down"} sign prefix="$">{p.pnl}</LN.Num></td>
              <td><LN.Num tone={p.pnlPct>=0?"up":"down"} sign suffix="%">{p.pnlPct}</LN.Num></td>
              <td><LN.IconButton name="close" label="Close"/></td>
            </tr>
          ))}
        </tbody>
      </table>
    </LN.Panel>
  );
}

window.Positions = Positions;

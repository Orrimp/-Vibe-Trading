/* eslint-disable */
// Order book + depth — calm two-column ladder.

function buildBook(mid = 1240.5) {
  const asks = [], bids = [];
  for (let i = 0; i < 8; i++) {
    asks.unshift({ p: +(mid + 0.05 + i * 0.05).toFixed(2), s: Math.round(800 - i*60 + Math.random()*200) });
    bids.push({ p: +(mid - 0.05 - i * 0.05).toFixed(2), s: Math.round(800 - i*60 + Math.random()*200) });
  }
  return { asks, bids };
}

function OrderBook({ symbol = "NVDA" }) {
  const { asks, bids } = buildBook(1240.50);
  const maxSize = Math.max(...asks.map(a=>a.s), ...bids.map(b=>b.s));

  return (
    <LN.Panel
      title="Order book"
      meta={symbol}
      actions={<LN.IconButton name="more" label="More"/>}
    >
      <div className="ln-book">
        <div className="ln-book-head">
          <span>Price</span>
          <span>Size</span>
          <span>Total</span>
        </div>
        {asks.map((a,i) => {
          const tot = asks.slice(i).reduce((s,r)=>s+r.s,0);
          const pct = (a.s / maxSize) * 100;
          return (
            <div key={"a"+i} className="ln-book-row ln-book-row--ask">
              <span className="ln-book-bar" style={{ width: pct + "%" }}/>
              <span className="ln-num ln-num--down">{a.p.toFixed(2)}</span>
              <span className="ln-num">{a.s}</span>
              <span className="ln-num ln-l">{tot}</span>
            </div>
          );
        })}
        <div className="ln-book-spread">
          <span>Spread</span>
          <span className="ln-num">0.10</span>
          <span className="ln-num ln-l">0.008%</span>
        </div>
        {bids.map((b,i) => {
          const tot = bids.slice(0,i+1).reduce((s,r)=>s+r.s,0);
          const pct = (b.s / maxSize) * 100;
          return (
            <div key={"b"+i} className="ln-book-row ln-book-row--bid">
              <span className="ln-book-bar" style={{ width: pct + "%" }}/>
              <span className="ln-num ln-num--up">{b.p.toFixed(2)}</span>
              <span className="ln-num">{b.s}</span>
              <span className="ln-num ln-l">{tot}</span>
            </div>
          );
        })}
      </div>
    </LN.Panel>
  );
}

window.OrderBook = OrderBook;

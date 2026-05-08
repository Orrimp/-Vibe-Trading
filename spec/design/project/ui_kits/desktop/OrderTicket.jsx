/* eslint-disable */
// Order ticket — buy/sell with limit/market, qty, TIF.

const { useState: useStateOT } = React;

function OrderTicket({ symbol = "NVDA", price = 1240.50, onSubmit }) {
  const [side, setSide] = useStateOT("buy");
  const [type, setType] = useStateOT("limit");
  const [qty, setQty] = useStateOT(10);
  const [limit, setLimit] = useStateOT(price.toFixed(2));
  const [tif, setTif] = useStateOT("day");

  const est = side === "buy" ? Number(limit) * qty : Number(limit) * qty;

  return (
    <LN.Panel
      title="Order ticket"
      meta={symbol}
      actions={<LN.IconButton name="more" label="More"/>}
    >
      <div className="ln-ticket">
        <div className="ln-segmented ln-segmented--side">
          <button className={`ln-seg ln-seg--side ${side==="buy"?"ln-seg--buy":""}`} onClick={()=>setSide("buy")}>Buy</button>
          <button className={`ln-seg ln-seg--side ${side==="sell"?"ln-seg--sell":""}`} onClick={()=>setSide("sell")}>Sell</button>
        </div>

        <div className="ln-field">
          <label>Type</label>
          <div className="ln-segmented">
            {["market","limit","stop"].map(t => (
              <button key={t} className={`ln-seg${type===t?" ln-seg--active":""}`} onClick={()=>setType(t)}>
                {t[0].toUpperCase()+t.slice(1)}
              </button>
            ))}
          </div>
        </div>

        <div className="ln-field-row">
          <div className="ln-field">
            <label>Quantity</label>
            <div className="ln-stepper">
              <button onClick={()=>setQty(q=>Math.max(1,q-1))}>−</button>
              <input className="ln-num" value={qty} onChange={e=>setQty(Number(e.target.value)||0)}/>
              <button onClick={()=>setQty(q=>q+1)}>+</button>
            </div>
          </div>
          <div className="ln-field">
            <label>Limit price</label>
            <div className="ln-input-group">
              <span className="ln-input-pre">$</span>
              <input className="ln-num" value={limit} onChange={e=>setLimit(e.target.value)}
                     disabled={type==="market"} style={{opacity: type==="market"?0.5:1}}/>
            </div>
          </div>
        </div>

        <div className="ln-field">
          <label>Time in force</label>
          <div className="ln-segmented">
            {["day","gtc","ioc"].map(t => (
              <button key={t} className={`ln-seg${tif===t?" ln-seg--active":""}`} onClick={()=>setTif(t)}>
                {t.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        <div className="ln-ticket-summary">
          <div><span>Estimated</span><span className="ln-num">${est.toLocaleString(undefined,{minimumFractionDigits:2,maximumFractionDigits:2})}</span></div>
          <div><span>Buying power</span><span className="ln-num">$148,420.00</span></div>
          <div><span>Commission</span><span className="ln-num">$0.00</span></div>
        </div>

        <button
          className={`ln-btn ln-btn--lg ln-btn--${side==="buy"?"buy":"sell"}`}
          onClick={()=>onSubmit && onSubmit({ side, type, qty, limit, tif })}
        >
          {side === "buy" ? "Buy" : "Sell"} {qty} {symbol} @ {type === "market" ? "MKT" : "$"+limit}
        </button>
      </div>
    </LN.Panel>
  );
}

window.OrderTicket = OrderTicket;

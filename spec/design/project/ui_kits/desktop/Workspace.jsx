/* eslint-disable */
// Workspace — tabbed layout. Overview is the default (AI strategies + agent feed + human control).
// Trade tab houses the legacy order book / order ticket flow.

const { useState: useStateW } = React;

const TABS = [
  { id: "overview",   label: "Overview",   icon: "layers" },
  { id: "strategies", label: "Strategies", icon: "bot" },
  { id: "backtest",   label: "Backtest",   icon: "book" },
  { id: "trade",      label: "Trade",      icon: "trending" },
  { id: "positions",  label: "Positions",  icon: "wallet" },
];

function WorkspaceTabs({ active, setActive }) {
  return (
    <div className="ln-wstabs">
      <div className="ln-wstabs-inner">
        {TABS.map(t => (
          <button key={t.id}
                  className={`ln-wstab${active===t.id?" ln-wstab--active":""}`}
                  onClick={()=>setActive(t.id)}>
            <LN.Icon name={t.icon} size={13}/>
            <span>{t.label}</span>
          </button>
        ))}
        <div style={{flex:1}}/>
        <div className="ln-wstabs-mode">
          <span className="ln-l">Mode</span>
          <span className="ln-tag ln-tag--info"><span className="ln-tag-dot ln-tag-dot--info"/>Supervised</span>
        </div>
      </div>
    </div>
  );
}

function Workspace() {
  const [tab, setTab] = useStateW("overview");
  const [sym, setSym] = useStateW("NVDA");
  const [stratSel, setStratSel] = useStateW("moma");
  const [toast, setToast] = useStateW(null);

  function handleSubmit(order) {
    setToast({
      title: `Order submitted — ${order.qty} ${sym}`,
      meta: `${order.type.toUpperCase()} · ${order.tif.toUpperCase()}`
    });
    setTimeout(() => setToast(null), 3500);
  }

  return (
    <div className="ln-ws">
      <WorkspaceTabs active={tab} setActive={setTab}/>
      <div className="ln-ws-body">
        {tab === "overview"   && <OverviewView   sym={sym} setSym={setSym} stratSel={stratSel} setStratSel={setStratSel}/>}
        {tab === "strategies" && <StrategiesView stratSel={stratSel} setStratSel={setStratSel}/>}
        {tab === "backtest"   && <BacktestView/>}
        {tab === "trade"      && <TradeView sym={sym} setSym={setSym} onSubmit={handleSubmit}/>}
        {tab === "positions"  && <PositionsView/>}
      </div>

      {toast && (
        <div className="ln-toast">
          <LN.Icon name="trending" size={14}/>
          <div>
            <div className="ln-toast-title">{toast.title}</div>
            <div className="ln-toast-meta">{toast.meta}</div>
          </div>
        </div>
      )}
    </div>
  );
}

/* ---------- Overview: AI-first dashboard ---------- */
function OverviewView({ sym, setSym, stratSel, setStratSel }) {
  return (
    <div className="ln-view ln-view--overview">
      <div className="ln-col ln-col--main">
        <FleetSummary/>
        <Strategies selected={stratSel} onSelect={setStratSel}/>
        <AgentFeed/>
      </div>
      <div className="ln-col ln-col--side">
        <HumanControl/>
        <ApprovalQueue/>
        <Assistant/>
      </div>
    </div>
  );
}

/* ---------- Strategies tab: deep view of a single strategy ---------- */
function StrategiesView({ stratSel, setStratSel }) {
  const s = STRATEGIES.find(x => x.id === stratSel) || STRATEGIES[0];
  return (
    <div className="ln-view ln-view--two">
      <div className="ln-col ln-col--narrow">
        <Strategies selected={stratSel} onSelect={setStratSel}/>
      </div>
      <div className="ln-col ln-col--main">
        <StrategyDetail strategy={s}/>
        <Chart symbol="NVDA"/>
        <AgentFeed/>
      </div>
    </div>
  );
}

/* ---------- Backtest tab ---------- */
function BacktestView() {
  return (
    <div className="ln-view ln-view--two">
      <div className="ln-col ln-col--narrow">
        <BacktestRunner/>
      </div>
      <div className="ln-col ln-col--main">
        <BacktestResult/>
      </div>
    </div>
  );
}

/* ---------- Trade tab: legacy order book + ticket live here now ---------- */
function TradeView({ sym, setSym, onSubmit }) {
  return (
    <div className="ln-view ln-view--trade">
      <div className="ln-col ln-col--narrow">
        <Watchlist selected={sym} onSelect={setSym}/>
      </div>
      <div className="ln-col ln-col--main">
        <Chart symbol={sym}/>
        <Positions/>
      </div>
      <div className="ln-col ln-col--side">
        <OrderBook symbol={sym}/>
        <OrderTicket symbol={sym} onSubmit={onSubmit}/>
      </div>
    </div>
  );
}

/* ---------- Positions tab ---------- */
function PositionsView() {
  return (
    <div className="ln-view ln-view--single">
      <Positions/>
    </div>
  );
}

window.Workspace = Workspace;

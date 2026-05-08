/* eslint-disable */
// Lumen — Title bar + side rail + status bar
const { useState: useStateShell } = React;

function TitleBar({ theme, setTheme, onCmd }) {
  return (
    <div className="ln-titlebar">
      <div className="ln-traffic">
        <span className="ln-traffic-dot" style={{ background: "#E27263" }}/>
        <span className="ln-traffic-dot" style={{ background: "#E0B45C" }}/>
        <span className="ln-traffic-dot" style={{ background: "#6E9B6A" }}/>
      </div>
      <div className="ln-tb-brand">
        <img src="../../assets/brand/lumen-mark.svg" width="14" height="14"
             style={{ filter: theme === "dark" ? "invert(1)" : "none" }}/>
        <span>Lumen</span>
        <span className="ln-tb-sep">/</span>
        <span className="ln-tb-workspace">Equities · US</span>
      </div>
      <div className="ln-tb-search" onClick={onCmd}>
        <LN.Icon name="search" size={13}/>
        <span>Search symbol, order, command…</span>
        <kbd>⌘K</kbd>
      </div>
      <div className="ln-tb-right">
        <button className="ln-iconbtn" onClick={() => setTheme(theme === "dark" ? "light" : "dark")} aria-label="Theme">
          <LN.Icon name={theme === "dark" ? "sun" : "moon"} size={15}/>
        </button>
        <button className="ln-iconbtn" aria-label="Notifications"><LN.Icon name="bell" size={15}/></button>
        <button className="ln-iconbtn" aria-label="Settings"><LN.Icon name="settings" size={15}/></button>
      </div>
    </div>
  );
}

function SideRail({ active, setActive }) {
  const items = [
    { id: "dash",      icon: "layers",   label: "Dashboard" },
    { id: "agents",    icon: "bot",      label: "Agents" },
    { id: "research",  icon: "book",     label: "Research" },
    { id: "watchlist", icon: "star",     label: "Watchlists" },
    { id: "wallet",    icon: "wallet",   label: "Accounts" },
    { id: "audit",     icon: "search",   label: "Audit" },
  ];
  return (
    <nav className="ln-rail">
      {items.map(it => (
        <button
          key={it.id}
          className={`ln-rail-item${active === it.id ? " ln-rail-item--active" : ""}`}
          onClick={() => setActive(it.id)}
          title={it.label}
        >
          <LN.Icon name={it.icon} size={18}/>
          <span className="ln-rail-label">{it.label}</span>
        </button>
      ))}
      <div style={{ flex: 1 }}/>
      <button className="ln-rail-item" title="Settings">
        <LN.Icon name="settings" size={18}/>
        <span className="ln-rail-label">Settings</span>
      </button>
    </nav>
  );
}

function StatusBar({ latency, account }) {
  return (
    <div className="ln-statusbar">
      <span className="ln-st-item">
        <span className="ln-st-dot ln-st-dot--ok"/> Connected · NYSE · NASDAQ · ARCA
      </span>
      <span className="ln-st-item">Latency <span className="ln-num">{latency}ms</span></span>
      <span className="ln-st-item">{account}</span>
      <div style={{ flex: 1 }}/>
      <span className="ln-st-item">Server <span className="ln-num">14:32:08 ET</span></span>
      <span className="ln-st-item">CPU <span className="ln-num">2.1%</span></span>
      <span className="ln-st-item">v0.4.1 · rust</span>
    </div>
  );
}

window.Shell = { TitleBar, SideRail, StatusBar };

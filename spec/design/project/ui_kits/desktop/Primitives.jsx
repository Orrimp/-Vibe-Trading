/* eslint-disable */
// Lumen primitives — Panel, Button, Input, Tag, Icon
// Loaded via <script type="text/babel">

const { useState, useEffect, useRef } = React;

/* --------------------------------- Icon -------------------------------- */
// Tiny Lucide-style icon set. 1.5px stroke. 16px default.
const ICONS = {
  search:   <><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.5-4.5"/></>,
  plus:     <><path d="M12 5v14M5 12h14"/></>,
  minus:    <><path d="M5 12h14"/></>,
  close:    <><path d="M18 6 6 18M6 6l12 12"/></>,
  star:     <><path d="M12 2l3 6 7 1-5 5 1 7-6-3-6 3 1-7-5-5 7-1z"/></>,
  trending: <><path d="M3 17l6-6 4 4 8-8"/><path d="M14 7h7v7"/></>,
  bell:     <><path d="M6 8a6 6 0 1 1 12 0c0 7 3 7 3 9H3c0-2 3-2 3-9z"/><path d="M10 21h4"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3h.1a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8v.1a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></>,
  layers:   <><path d="M12 3l9 4-9 4-9-4 9-4z"/><path d="M3 12l9 4 9-4"/><path d="M3 17l9 4 9-4"/></>,
  book:     <><path d="M4 5a2 2 0 0 1 2-2h12v18H6a2 2 0 0 1-2-2z"/><path d="M4 17h14"/></>,
  wallet:   <><rect x="3" y="6" width="18" height="13" rx="2"/><path d="M16 12h3"/></>,
  bot:      <><rect x="4" y="8" width="16" height="11" rx="2"/><path d="M12 4v4M9 13h.01M15 13h.01"/></>,
  chevron:  <><path d="M6 9l6 6 6-6"/></>,
  arrowUp:  <><path d="M12 19V5M5 12l7-7 7 7"/></>,
  arrowDn:  <><path d="M12 5v14M5 12l7 7 7-7"/></>,
  sun:      <><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></>,
  moon:     <><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></>,
  command:  <><path d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3 3 3 0 0 0-3-3H6a3 3 0 0 0-3 3 3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3 3 3 0 0 0 3 3h12a3 3 0 0 0 3-3 3 3 0 0 0-3-3z"/></>,
  filter:   <><path d="M4 5h16l-6 8v6l-4-2v-4z"/></>,
  more:     <><circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/></>,
  link:     <><path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1"/><path d="M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1"/></>,
  send:     <><path d="M22 2 11 13"/><path d="M22 2 15 22l-4-9-9-4z"/></>,
};

function Icon({ name, size = 16, color, style, ...rest }) {
  const path = ICONS[name];
  if (!path) return null;
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none"
         stroke={color || "currentColor"} strokeWidth="1.5"
         strokeLinecap="round" strokeLinejoin="round"
         style={{ display: "inline-block", verticalAlign: "middle", flexShrink: 0, ...style }}
         {...rest}>
      {path}
    </svg>
  );
}

/* -------------------------------- Panel -------------------------------- */
function Panel({ title, meta, children, actions, padded = false, style }) {
  return (
    <div className="ln-panel" style={style}>
      {(title || actions) && (
        <div className="ln-panel-h">
          <div className="ln-panel-title">
            <span>{title}</span>
            {meta && <span className="ln-panel-meta">{meta}</span>}
          </div>
          <div className="ln-panel-actions">{actions}</div>
        </div>
      )}
      <div className={padded ? "ln-panel-body--p" : "ln-panel-body"}>{children}</div>
    </div>
  );
}

/* -------------------------------- Button ------------------------------- */
function Button({ kind = "secondary", size = "md", icon, children, onClick, ...rest }) {
  return (
    <button className={`ln-btn ln-btn--${kind} ln-btn--${size}`} onClick={onClick} {...rest}>
      {icon && <Icon name={icon} size={14} />}
      {children}
    </button>
  );
}

/* --------------------------------- Tag --------------------------------- */
function Tag({ tone = "neutral", children, dot }) {
  return (
    <span className={`ln-tag ln-tag--${tone}`}>
      {dot && <span className={`ln-tag-dot ln-tag-dot--${tone}`}/>}
      {children}
    </span>
  );
}

/* --------------------------- Tabular number ---------------------------- */
function Num({ children, tone, sign = false, prefix, suffix }) {
  let n = children;
  let display = String(n);
  if (sign && typeof n === "number") {
    display = (n > 0 ? "+" : n < 0 ? "−" : "") + Math.abs(n).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }
  return (
    <span className={tone ? `ln-num ln-num--${tone}` : "ln-num"}>
      {prefix}{display}{suffix}
    </span>
  );
}

/* ---------------------------- IconButton ------------------------------- */
function IconButton({ name, label, onClick, active }) {
  return (
    <button className={`ln-iconbtn${active ? " ln-iconbtn--active" : ""}`} aria-label={label} onClick={onClick}>
      <Icon name={name} size={16}/>
    </button>
  );
}

window.LN = { Icon, Panel, Button, Tag, Num, IconButton };

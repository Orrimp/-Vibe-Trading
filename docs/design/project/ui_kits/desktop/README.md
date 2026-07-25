# Lumen Trading Desktop — UI kit

A high-fidelity, cosmetic recreation of the Lumen desktop trading app. Built as React + JSX components for visual reference; not wired to any data feed.

## Files
- `index.html` — interactive demo. Light/dark toggle, click between panels, place a mock order.
- `App.jsx` — composes the desktop shell.
- `Shell.jsx` — title bar, side rail, status bar.
- `Workspace.jsx` — multi-panel grid: watchlist, chart, order book, positions, ticket, assistant.
- `Watchlist.jsx`, `Chart.jsx`, `OrderBook.jsx`, `Positions.jsx`, `OrderTicket.jsx`, `Assistant.jsx` — panels.
- `Primitives.jsx` — Panel, Button, Input, Tag, Icon, etc.

## What's faithful
- Token usage from `colors_and_type.css` (light + dark).
- Surface tiers: canvas → panel → raised; hairline borders; whisper shadows.
- All numerics in JetBrains Mono with tabular figures.
- Lucide-style icons, 1.5 px stroke.

## What's mocked
- Chart: a stylized SVG candlestick frame, not real OHLC.
- Order book / depth: hand-rolled snapshot.
- AI assistant: pre-canned message and a fake "review signal" action.

/* eslint-disable */
// Lumen AI assistant panel — chat-like, with one canned signal.

const { useState: useStateAI, useRef: useRefAI, useEffect: useEffectAI } = React;

const SEED_MESSAGES = [
  { role: "ai", kind: "signal",
    title: "NVDA — momentum building",
    body: "Volume is +38% above its 20-day average and the 5-minute trend just crossed VWAP. Risk-adjusted entry around 1,236.",
    confidence: 0.72,
    actions: ["Review signal", "Mute symbol"]
  },
  { role: "user", body: "What's my exposure to semis?" },
  { role: "ai", body: "Across NVDA, AMD, and adjacent ETFs you hold $356,820 — roughly 41% of book. Sector beta is 1.6 against SPY." },
];

function Confidence({ value }) {
  const pct = Math.round(value * 100);
  return (
    <div className="ln-conf">
      <div className="ln-conf-bar">
        <div className="ln-conf-fill" style={{ width: pct + "%" }}/>
      </div>
      <span className="ln-num ln-l">{pct}%</span>
    </div>
  );
}

function Assistant() {
  const [messages, setMessages] = useStateAI(SEED_MESSAGES);
  const [draft, setDraft] = useStateAI("");
  const endRef = useRefAI(null);

  useEffectAI(() => { endRef.current?.scrollTo(0, endRef.current.scrollHeight); }, [messages]);

  function send() {
    if (!draft.trim()) return;
    const q = draft.trim();
    setMessages(m => [...m, { role: "user", body: q }]);
    setDraft("");
    setTimeout(() => {
      setMessages(m => [...m, {
        role: "ai",
        body: "Drafting an answer based on your portfolio and current market state. (This is a UI mock — connect a model to see real responses.)"
      }]);
    }, 600);
  }

  return (
    <LN.Panel
      title="Lumen"
      meta="AI assistant"
      actions={<>
        <LN.IconButton name="link" label="Context"/>
        <LN.IconButton name="more" label="More"/>
      </>}
    >
      <div className="ln-chat" ref={endRef}>
        {messages.map((m, i) => (
          <div key={i} className={`ln-msg ln-msg--${m.role}`}>
            {m.role === "ai" && (
              <div className="ln-msg-avatar">
                <img src="../../assets/brand/lumen-ai-lens.svg" width="14" height="14" alt=""/>
              </div>
            )}
            <div className="ln-msg-body">
              {m.kind === "signal" ? (
                <div className="ln-signal">
                  <div className="ln-signal-head">
                    <LN.Tag tone="info" dot>Signal</LN.Tag>
                    <span className="ln-signal-title">{m.title}</span>
                  </div>
                  <p>{m.body}</p>
                  <div className="ln-signal-meta">
                    <span className="ln-l">Confidence</span>
                    <Confidence value={m.confidence}/>
                  </div>
                  <div className="ln-signal-actions">
                    {m.actions.map((a, k) => (
                      <button key={k} className={`ln-btn ln-btn--sm ln-btn--${k===0?"primary":"secondary"}`}>{a}</button>
                    ))}
                  </div>
                </div>
              ) : (
                <div className="ln-msg-text">{m.body}</div>
              )}
            </div>
          </div>
        ))}
      </div>
      <div className="ln-composer">
        <input
          placeholder="Ask Lumen — e.g. risk on my book"
          value={draft}
          onChange={e=>setDraft(e.target.value)}
          onKeyDown={e=>{ if(e.key==="Enter") send(); }}
        />
        <button className="ln-iconbtn ln-iconbtn--accent" onClick={send} aria-label="Send">
          <LN.Icon name="send" size={14}/>
        </button>
      </div>
    </LN.Panel>
  );
}

window.Assistant = Assistant;

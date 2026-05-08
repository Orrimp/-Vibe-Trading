/* eslint-disable */
// HumanControl — global kill switch + approval mode toggles. Always visible.

const { useState: useStateHC } = React;

function HumanControl() {
  const [mode, setMode] = useStateHC("supervised");
  return (
    <LN.Panel title="You're in control" meta="Human-in-the-loop">
      <div className="ln-control">
        <div className="ln-control-mode">
          <div className="ln-l">Execution mode</div>
          <div className="ln-segmented" style={{width:"100%"}}>
            {[
              {id:"observe",   label:"Observe"},
              {id:"supervised",label:"Supervised"},
              {id:"auto",      label:"Auto"}
            ].map(m => (
              <button key={m.id}
                      className={`ln-seg${mode===m.id?" ln-seg--active":""}`}
                      style={{flex:1}}
                      onClick={()=>setMode(m.id)}>
                {m.label}
              </button>
            ))}
          </div>
          <div className="ln-control-hint">
            {mode==="observe" && "Agents simulate but never place orders."}
            {mode==="supervised" && "Each trade requires your approval before going live."}
            {mode==="auto" && "Agents trade within preset risk envelopes. You can halt at any time."}
          </div>
        </div>

        <div className="ln-control-limits">
          <div className="ln-control-row">
            <span className="ln-l">Daily loss limit</span>
            <span className="ln-num">$5,000.00</span>
          </div>
          <div className="ln-control-row">
            <span className="ln-l">Max position</span>
            <span className="ln-num">5% NAV</span>
          </div>
          <div className="ln-control-row">
            <span className="ln-l">Used today</span>
            <span className="ln-num ln-num--up">+$22,873.20</span>
          </div>
        </div>

        <button className="ln-btn ln-btn--lg ln-btn--sell" style={{marginTop:4}}>
          <LN.Icon name="close" size={14}/> Halt all agents
        </button>
      </div>
    </LN.Panel>
  );
}

window.HumanControl = HumanControl;

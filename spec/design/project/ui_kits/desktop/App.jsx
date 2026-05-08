/* eslint-disable */
// App — title bar + side rail + workspace + status bar

const { useState: useStateApp, useEffect: useEffectApp } = React;

function App() {
  const [theme, setTheme] = useStateApp("light");

  useEffectApp(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  return (
    <div className="ln-app" data-screen-label="Lumen Desktop">
      <Shell.TitleBar theme={theme} setTheme={setTheme}/>
      <div className="ln-body">
        <Shell.SideRail active="dash" setActive={()=>{}}/>
        <main className="ln-main">
          <Workspace/>
        </main>
      </div>
      <Shell.StatusBar latency={4} account="Margin · ****1402"/>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App/>);

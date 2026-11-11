//! bug-log `#197`: the F9 narration task must use the boot-built LLM provider.
//!
//! Until 2026-11-11 `runtime::run` hard-coded `llm_provider = None` in BOTH branches
//! of `if config.llm.enabled`, so every "Explain" answered `FellBack` while the
//! CHANGELOG, PRD FR-11 and README said an LLM wrote the narration. These two tests
//! drive the REAL runtime (`agent::runtime::run`) over the real request/outcome
//! channels and differ ONLY in `RunHandles::llm_provider`:
//!
//! - a configured (no-network) faithful provider → `Ready(prose)`: the provider was
//!   called and its text passed the faithfulness post-check;
//! - no provider → `FellBack`.
//!
//! T-T4 probe: in `runtime::run`, shadow the destructured `llm_provider` with
//! `let llm_provider: Option<Arc<dyn llm::LlmProvider>> = None;` → the first test
//! turns red (it gets `FellBack`), the second stays green.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::config::{BusConfig, Config};
use agent::narration::{
    CandidateKpiStrings, FaithfulFakeProvider, NarrationFacts, NarrationOutcome, NarrationOutcome_,
    NarrationRequest,
};
use agent::{EventBus, IncidentSpawner, KillSwitch, MockIncidentSpawner, RunHandles};
use smol_str::SmolStr;
use tokio_util::sync::CancellationToken;

fn facts() -> NarrationFacts {
    let kpi = |id: &str, sharpe: &str, ret: &str, dd: &str, trades: &str| CandidateKpiStrings {
        strategy_id: SmolStr::new(id),
        sharpe: sharpe.to_string(),
        sortino: "0.6789".to_string(),
        calmar: "0.7890".to_string(),
        total_return_pct: ret.to_string(),
        max_drawdown: dd.to_string(),
        trade_count: trades.to_string(),
    };
    NarrationFacts {
        outcome: NarrationOutcome_::ActiveWins,
        winner_id: SmolStr::new("v0.5.macd"),
        candidate_ids: vec![SmolStr::new("v0.5.macd"), SmolStr::new("v0.buyhold")],
        candidate_kpi_strings: vec![
            kpi("v0.5.macd", "0.5678", "15.00%", "-5.00%", "88"),
            kpi("v0.buyhold", "0.2500", "8.00%", "-12.00%", "1"),
        ],
        winner_robustness_label: Some(SmolStr::new("robust")),
        reason_codes: vec![SmolStr::new("highest Sharpe among robust candidates")],
    }
}

/// Boot the real runtime with the narration channels wired, send one "Explain"
/// request, and return the outcome the agent sends back.
async fn explain_once(llm_provider: Option<Arc<dyn llm::LlmProvider>>) -> NarrationOutcome {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cfg = Config::default(); // research mode: no exchange reach-out
    cfg.audit.ledger_db_path = temp.path().join("ledger.db").to_string_lossy().into_owned();
    cfg.kill_switch.halt_file = temp.path().join(".halt").to_string_lossy().into_owned();
    cfg.data.historical.parquet_root = temp.path().to_string_lossy().into_owned();

    let ledger = Arc::new(
        audit::Ledger::open(&cfg.audit.ledger_db_path)
            .await
            .expect("ledger"),
    );
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("chart");
    let spawner: Arc<dyn IncidentSpawner> = Arc::new(MockIncidentSpawner::new());
    let kill_switch = Arc::new(KillSwitch::with_audit(
        &cfg.kill_switch.halt_file,
        32,
        Arc::clone(&ledger),
        spawner,
    ));
    let boot_id = uuid::Uuid::new_v4().to_string();
    audit::journal::open_uptime_interval(&ledger, &boot_id, None)
        .await
        .expect("uptime");

    let (req_tx, req_rx) = tokio::sync::mpsc::channel::<NarrationRequest>(4);
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<NarrationOutcome>(4);
    let handles = RunHandles {
        config: Arc::new(cfg),
        ledger: Arc::clone(&ledger),
        bus: Arc::new(EventBus::new(&BusConfig::default())),
        kill_switch,
        registry: Arc::new(strategy::StrategyRegistry::new()),
        boot_id,
        equity_store: None,
        reflection_writer: None,
        forward_rx: None,
        plan_tx: None,
        narration_request_rx: Some(req_rx),
        narration_outcome_tx: Some(out_tx),
        llm_provider,
    };
    let cancel = CancellationToken::new();
    let runtime = tokio::spawn(agent::runtime::run(handles, cancel.clone()));

    req_tx
        .send(NarrationRequest { facts: facts() })
        .await
        .expect("request sent");
    let outcome = tokio::time::timeout(Duration::from_secs(10), out_rx.recv())
        .await
        .expect("the narration task answered within 10 s")
        .expect("the outcome channel stayed open");

    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(5), runtime).await;
    outcome
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_configured_provider_is_called_and_its_narration_returns() {
    let provider: Arc<dyn llm::LlmProvider> = Arc::new(FaithfulFakeProvider { facts: facts() });
    let outcome = explain_once(Some(provider)).await;
    assert!(
        matches!(&outcome, NarrationOutcome::Ready(prose) if prose.contains("v0.5.macd")),
        "a configured provider must be CALLED and its faithful text returned; got {outcome:?} \
         (FellBack here means the runtime ignored RunHandles::llm_provider — bug-log #197)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_provider_falls_back_to_the_templated_copy() {
    let outcome = explain_once(None).await;
    assert_eq!(outcome, NarrationOutcome::FellBack);
}

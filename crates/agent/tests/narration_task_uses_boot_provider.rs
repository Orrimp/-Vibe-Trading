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
//!
//! bug-log `#201`: the narration task is ONE long-lived loop, so a panic inside
//! `generate_narration` used to end it, and with it Explain for the session. A third
//! test sends two requests on one boot to a provider that panics on its first call
//! only: the first answer is `FellBack`, the second `Ready`. Probe: drop the task's
//! `catch_unwind` → the outcome channel closes and that test fails at its first
//! answer.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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

/// Boot the real runtime with the narration channels wired, send `requests`
/// "Explain" requests one after another, and return the outcomes the agent sends
/// back, in order. Every send and receive is bounded, so a dead task fails the test.
async fn explain(
    llm_provider: Option<Arc<dyn llm::LlmProvider>>,
    requests: usize,
) -> Vec<NarrationOutcome> {
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

    let mut outcomes = Vec::with_capacity(requests);
    for i in 0..requests {
        let send = req_tx.send(NarrationRequest { facts: facts() });
        match tokio::time::timeout(Duration::from_secs(10), send).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => panic!("request {i}: the request channel closed (bug-log #201)"),
            Err(_) => panic!("request {i}: not taken within 10 s"),
        }
        let outcome = match tokio::time::timeout(Duration::from_secs(10), out_rx.recv()).await {
            Ok(Some(outcome)) => outcome,
            Ok(None) => panic!(
                "request {i}: the outcome channel closed, so the narration task ended \
                 (bug-log #201)"
            ),
            Err(_) => panic!("request {i}: the narration task did not answer within 10 s"),
        };
        outcomes.push(outcome);
    }

    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(5), runtime).await;
    outcomes
}

/// Boot, send one request, return its outcome.
async fn explain_once(llm_provider: Option<Arc<dyn llm::LlmProvider>>) -> NarrationOutcome {
    explain(llm_provider, 1).await.remove(0)
}

/// Bug-log `#201`: panics on its FIRST call only, then answers as the faithful fake
/// does, so a second request shows whether the narration task outlived the panic.
struct PanicsOnceProvider {
    calls: AtomicUsize,
    then: FaithfulFakeProvider,
}

#[async_trait::async_trait]
impl llm::LlmProvider for PanicsOnceProvider {
    fn name(&self) -> &str {
        "panics_once_fake"
    }

    fn provider_kind(&self) -> llm::ProviderKind {
        llm::ProviderKind::Anthropic
    }

    async fn complete(
        &self,
        request: llm::ChatRequest,
    ) -> Result<llm::ChatResponse, llm::LlmError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            panic!("PanicsOnceProvider: the first narration panics (bug-log #201)");
        }
        llm::LlmProvider::complete(&self.then, request).await
    }
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

/// Bug-log `#201`: a panic inside `generate_narration` costs that one narration
/// (`FellBack`), and the same task serves the next request (`Ready`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panicking_narration_falls_back_and_the_task_serves_the_next_request() {
    let provider = Arc::new(PanicsOnceProvider {
        calls: AtomicUsize::new(0),
        then: FaithfulFakeProvider { facts: facts() },
    });
    let dyn_provider: Arc<dyn llm::LlmProvider> = provider.clone();
    let outcomes = tokio::time::timeout(Duration::from_secs(60), explain(Some(dyn_provider), 2))
        .await
        .expect("boot, two requests and shutdown finished within 60 s");

    assert_eq!(
        outcomes[0],
        NarrationOutcome::FellBack,
        "the panicking call must answer FellBack"
    );
    assert!(
        matches!(&outcomes[1], NarrationOutcome::Ready(prose) if prose.contains("v0.5.macd")),
        "the task must outlive the panic and serve the second request; got {:?}",
        outcomes[1]
    );
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        2,
        "both requests reached the provider: the FellBack came from the panic"
    );
}

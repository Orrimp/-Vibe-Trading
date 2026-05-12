//! `llm-smoke` — T1923 — end-to-end smoke binary that exercises the
//! full v2 LLM stack against wiremock servers (no real network) or,
//! in operator-environment mode, against real provider APIs read
//! from `agent.toml.local`.
//!
//! ## Modes
//!
//! - `--mode live` — uses `agent.toml` + `agent.toml.local` keys.
//!   `LlmProviderFactory::build(Mode::Live, ...)`. Talks to real
//!   providers if keys are present; falls back to ollama if only
//!   ollama is configured.
//! - `--mode paper` — wraps the live stack in `RecordingProvider`
//!   that writes every successful `complete()` into the configured
//!   replay cache (default `data/llm-replay.db`). Run this against
//!   real APIs once per provider × role to refresh the fixture
//!   cache (Q8d / T1925).
//! - `--mode research` — opens `crates/llm/fixtures/replay-v1.db`
//!   read-only and replays canned responses; cache miss panics with
//!   `LlmError::ReplayMiss { hash, provider, model }` per D2.
//!
//! ## CLI
//!
//! ```text
//! llm-smoke --mode {live|paper|research}
//!           [--replay-path <PATH>]
//!           [--reset]
//! ```
//!
//! `--reset` deletes the replay cache before opening it (Q8c —
//! operator-managed cache). Combined with `--mode paper` it gives a
//! clean re-record from a blank slate.
//!
//! ## Exit codes
//!
//! - `0` — every provider returned the literal `OK`.
//! - `1` — at least one provider's response was not `OK`, or a
//!   cache miss panicked under research mode (the panic is converted
//!   to a non-zero exit through `process::exit` so CI sees a clean
//!   error code).
//! - `2` — config / CLI parse error.
//!
//! ## Output
//!
//! Renders an aligned ASCII table via `tracing::info!` lines at
//! `target = "llm.smoke"`. The R10.1 result table column order is
//! `provider / model / tokens_in / tokens_out / usd / latency_ms /
//! result`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, ValueEnum};
use cost::{AgentRole, CostBudget, CostSink, LlmTier, NoopCostSink};
use llm::factory::{LlmProviderFactory, Mode as FactoryMode};
use llm::{ChatMessage, ChatRequest, ContentBlock, LlmConfig, LlmError, MessageRole};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliMode {
    Live,
    Paper,
    Research,
}

impl CliMode {
    fn to_factory(self) -> FactoryMode {
        match self {
            CliMode::Live => FactoryMode::Live,
            CliMode::Paper => FactoryMode::Paper,
            CliMode::Research => FactoryMode::Research,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "llm-smoke",
    about = "End-to-end smoke test for the v2 LLM stack."
)]
struct Cli {
    /// Operating mode: live (real APIs), paper (record), research (replay).
    #[arg(long, value_enum, default_value_t = CliMode::Research)]
    mode: CliMode,

    /// Override the replay cache path. Defaults to LlmConfig::default()
    /// which is `data/llm-replay.db` (live/paper) or
    /// `crates/llm/fixtures/replay-v1.db` (research, set via
    /// `--replay-path` or env override).
    #[arg(long)]
    replay_path: Option<PathBuf>,

    /// `agent.toml` path for key loading. Defaults to `config/agent.toml`
    /// (live/paper modes only; research mode skips key load).
    #[arg(long, default_value = "config/agent.toml")]
    agent_toml: PathBuf,

    /// Delete the replay cache before opening (Q8c). Only meaningful
    /// for `--mode paper`; ignored under `live` and `research`.
    #[arg(long)]
    reset: bool,
}

fn main() -> ExitCode {
    // Init tracing — JSON in CI, pretty locally. Test runs always
    // get the line-based shape so the integration test can grep.
    tracing_subscriber::fmt()
        .with_target(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("llm=info,llm.smoke=info")),
        )
        .try_init()
        .ok();

    let cli = Cli::parse();
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!(target: "llm.smoke", error = %e, "build tokio runtime");
            return ExitCode::from(2);
        }
    };
    rt.block_on(run(cli))
}

async fn run(cli: Cli) -> ExitCode {
    let mut cfg = LlmConfig::default();
    if let Some(p) = cli.replay_path.clone() {
        cfg.replay_cache_path = p;
    }
    let cfg = Arc::new(cfg);

    if cli.reset && cli.mode == CliMode::Paper {
        let path = &cfg.replay_cache_path;
        if path.exists() {
            if let Err(e) = tokio::fs::remove_file(path).await {
                tracing::error!(
                    target: "llm.smoke",
                    error = %e,
                    path = %path.display(),
                    "reset: remove_file failed"
                );
                return ExitCode::from(2);
            }
            // Also drop the WAL sidecar if present.
            for suffix in [".wal", ".shm"] {
                let mut sib = path.clone().into_os_string();
                sib.push(suffix);
                let sib = PathBuf::from(sib);
                let _ = tokio::fs::remove_file(&sib).await;
            }
        }
    }

    let budget = Arc::new(CostBudget::new(cfg.budget_usd_month));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let provider = match LlmProviderFactory::build(
        Arc::clone(&cfg),
        cli.mode.to_factory(),
        Arc::clone(&budget),
        sink,
        &cli.agent_toml,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(target: "llm.smoke", error = %e, "factory build");
            return ExitCode::from(2);
        }
    };

    // Drive one prompt per role and render the result table.
    let roles = [
        AgentRole::Trader,
        AgentRole::SentimentAnalyst,
        AgentRole::Other("smoke".to_string()),
    ];
    let mut all_ok = true;
    tracing::info!(
        target: "llm.smoke",
        "provider | model | tokens_in | tokens_out | usd | latency_ms | result"
    );
    for role in &roles {
        let mut req = ChatRequest::new(
            cfg.deep_think.model.clone(),
            LlmTier::DeepThink,
            role.clone(),
        );
        req.messages.push(ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text(
                "Reply with the literal string `OK` and nothing else.".to_string(),
            )],
        });

        let start = Instant::now();
        let outcome = provider.complete(req).await;
        let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

        match outcome {
            Ok(resp) => {
                let text = first_text(&resp.content).unwrap_or_default();
                let result = if text.trim() == "OK" {
                    "OK"
                } else {
                    "MISMATCH"
                };
                if result != "OK" {
                    all_ok = false;
                }
                tracing::info!(
                    target: "llm.smoke",
                    provider = %provider.name(),
                    model = %resp.model,
                    tokens_in = resp.usage.tokens_in,
                    tokens_out = resp.usage.tokens_out,
                    usd = %Decimal::ZERO,
                    latency_ms,
                    result = %result,
                    role = ?role,
                    "row"
                );
            }
            Err(e) => {
                all_ok = false;
                tracing::error!(
                    target: "llm.smoke",
                    provider = %provider.name(),
                    error = %e,
                    latency_ms,
                    role = ?role,
                    "row_error"
                );
                // ReplayMiss is the operator's actionable signal under
                // research mode — surface it loudly.
                if matches!(e, LlmError::ReplayMiss { .. }) {
                    tracing::error!(
                        target: "llm.smoke",
                        "research mode: cache miss (D2 strict-only). \
                         Refresh fixtures via `cargo run --bin llm-smoke -- --mode paper`."
                    );
                }
            }
        }
    }

    if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn first_text(blocks: &[ContentBlock]) -> Option<&str> {
    for b in blocks {
        if let ContentBlock::Text(t) = b {
            return Some(t.as_str());
        }
    }
    None
}

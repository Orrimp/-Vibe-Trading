//! LLM provider construction at boot, shared by the headless `trading` bin and
//! `cockpit_live` (bug-log `#197`).
//!
//! Until 2026-11-11 the headless bin built the provider and parked it in an unused
//! local ("until a consumer brief plucks the provider"), `cockpit_live` never built
//! one at all, and the F9 narration task hard-coded `llm_provider = None` in BOTH
//! branches of `if config.llm.enabled` — so "Explain" always fell back to the
//! templated copy while the CHANGELOG, PRD FR-11 and README said an LLM wrote it.
//! One function now builds it for both bins, and `RunHandles::llm_provider` carries
//! it to the narration task.

use std::path::Path;
use std::sync::Arc;

use cost::{CostBudget, CostSink, NoopCostSink};
use llm::factory::{LlmProviderFactory, Mode as LlmMode};
use tracing::{info, warn};

use crate::config::{Config, Mode};

/// Build the LLM provider stack once, when `cfg.llm.enabled`; `None` otherwise.
///
/// With `llm.enabled = false` (the default) nothing is constructed, no key file is
/// read and no `.local` overlay is required, so a fresh checkout boots cleanly. A
/// factory error (missing key, unopenable replay cache) is non-fatal: it is logged
/// and `None` is returned, and every consumer falls back honestly.
pub async fn build_llm_provider(
    cfg: &Config,
    config_path: &Path,
) -> Option<Arc<dyn llm::LlmProvider>> {
    if !cfg.llm.enabled {
        info!("llm subsystem disabled (cfg.llm.enabled = false)");
        return None;
    }
    let llm_cfg = Arc::new(cfg.llm.clone());
    let budget = Arc::new(CostBudget::new(cfg.llm.budget_usd_month));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let llm_mode = match cfg.mode {
        Mode::Research => LlmMode::Research,
        Mode::Paper => LlmMode::Paper,
    };
    match LlmProviderFactory::build(llm_cfg, llm_mode, budget, sink, config_path).await {
        Ok(p) => {
            info!(provider = %p.name(), mode = %cfg.mode, "llm provider constructed");
            Some(p)
        }
        Err(e) => {
            warn!(error = %e, "llm factory build failed (non-fatal); subsystem disabled");
            None
        }
    }
}

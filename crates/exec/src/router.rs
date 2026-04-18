//! `ExecRouter` trait and paper-mode implementation stub.
use async_trait::async_trait;
use thiserror::Error;
use trading_core::{Fill, Order};

/// Error from the execution router.
#[derive(Debug, Error)]
pub enum ExecError {
    #[error("unsupported mode: {0}")]
    UnsupportedMode(String),
    #[error("order rejected: {0}")]
    OrderRejected(String),
    #[error("fill failed: {0}")]
    FillFailed(String),
}

/// Routes orders to the appropriate matching engine.
#[async_trait]
pub trait ExecRouter: Send + Sync {
    async fn submit(&mut self, order: Order) -> Result<Fill, ExecError>;
}

/// Paper-mode exec router — delegates to `backtest::PaperEngine` (T24).
pub struct PaperExecRouter;

#[async_trait]
impl ExecRouter for PaperExecRouter {
    async fn submit(&mut self, _order: Order) -> Result<Fill, ExecError> {
        Err(ExecError::UnsupportedMode(
            "PaperExecRouter not yet wired (T24)".into(),
        ))
    }
}

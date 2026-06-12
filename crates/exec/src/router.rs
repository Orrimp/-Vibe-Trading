//! `ExecRouter` trait and paper-mode implementation stub.
//!
//! The `ExecError` type is defined in `crate::live::error` and re-exported
//! from `crate::live::error::ExecError` for the F1 live taxonomy extension.
//! Paper variants (`UnsupportedMode`/`OrderRejected`/`FillFailed`) live there
//! alongside the live variants — `PaperExecRouter` uses them unchanged.
use async_trait::async_trait;
use trading_core::{Fill, Order};

pub use crate::live::error::ExecError;

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

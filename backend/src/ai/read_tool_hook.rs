//! # Read Tool Hook  -  Scaffolding for AI tool-call result capture
//!
//! This module provides the scaffolding for a "read tool hook": a callback that
//! fires whenever an AI integration reads the result of a tool invocation. It lets
//! downstream consumers (logging, auditing, caching, re-ranking) observe tool
//! outputs without coupling to any specific tool implementation.
//!
//! ## Status
//!
//! Scaffolding only. The hook trait and a no-op default implementation are
//! provided; wiring into concrete tool runners is left to follow-up work.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// A captured tool invocation result that a read hook can observe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReadEvent {
    /// Stable identifier of the tool that was invoked.
    pub tool_name: String,
    /// Raw JSON (or text) payload returned by the tool.
    pub result: String,
    /// Optional correlation id tying the read back to a request/turn.
    pub correlation_id: Option<String>,
    /// Wall-clock timestamp (ms since epoch) when the read occurred.
    pub timestamp_ms: u64,
}

/// Hook invoked when an AI integration reads a tool result.
#[async_trait]
pub trait ReadToolHook: Send + Sync {
    /// Called with the captured event. Implementations must be non-blocking
    /// or offload heavy work internally.
    async fn on_tool_read(&self, event: ToolReadEvent);

    /// Human-readable name of the hook, used for diagnostics.
    fn name(&self) -> &'static str;
}

/// No-op implementation, useful as a default when no hook is configured.
pub struct NoOpReadToolHook;

#[async_trait]
impl ReadToolHook for NoOpReadToolHook {
    async fn on_tool_read(&self, event: ToolReadEvent) {
        debug!(tool = %event.tool_name, "read tool hook (no-op)");
    }

    fn name(&self) -> &'static str {
        "noop"
    }
}

/// A composite hook that fans out to multiple inner hooks.
pub struct CompositeReadToolHook {
    hooks: Vec<Arc<dyn ReadToolHook>>,
}

impl CompositeReadToolHook {
    pub fn new(hooks: Vec<Arc<dyn ReadToolHook>>) -> Self {
        Self { hooks }
    }

    pub fn empty() -> Self {
        Self { hooks: Vec::new() }
    }
}

#[async_trait]
impl ReadToolHook for CompositeReadToolHook {
    async fn on_tool_read(&self, event: ToolReadEvent) {
        for hook in &self.hooks {
            let ev = event.clone();
            let h = hook.clone();
            tokio::spawn(async move {
                h.on_tool_read(ev).await;
            });
        }
    }

    fn name(&self) -> &'static str {
        "composite"
    }
}

/// Convenience: build a default hook chain.
pub fn default_read_tool_hook() -> Arc<dyn ReadToolHook> {
    info!("initializing default read tool hook (no-op)");
    Arc::new(NoOpReadToolHook)
}
PY
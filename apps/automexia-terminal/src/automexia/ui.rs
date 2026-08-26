//! Automexia-owned renderer-neutral UI contracts for core and optional features.
//!
//! This module contains renderer-neutral geometry/data only. The terminal engine,
//! PTY and VT parser must not depend on product-domain or extension implementations.

/// One semantic-prompt row geometry that an application contribution may decorate.
///
/// Historical anchors come from OSC 133 row metadata. The live anchor is
/// resolved from the current Prompt/PromptContinuation pair every frame and is
/// gated by generic shell prompt-readiness metadata; cursor geometry is only a
/// first-paint fallback before semantic rows arrive. Extensions therefore never
/// inspect terminal-engine row/cell types directly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PromptAnchor {
    /// Stable identity published by the shell in `OSC 133;A;aid=<id>`.
    /// Older integrations may omit it; `key` remains the compatibility
    /// fallback for those sessions.
    pub generation: Option<u64>,
    /// Absolute-row key for the current layout snapshot. It is stable while the
    /// row remains in the same reflow layout, but may change after resize/reflow.
    pub key: u64,
    /// Logical-pixel left edge of the active panel's terminal grid.
    pub x: f32,
    /// Logical-pixel top edge of this prompt row.
    pub y: f32,
    /// Logical-pixel width available to this prompt row.
    pub width: f32,
    /// Logical-pixel row height.
    pub height: f32,
}

/// Geometry and durable completion metadata for one command row.
///
/// The terminal engine owns the lifecycle data; Automexia receives this small
/// renderer-neutral projection and decides how to present it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommandResultAnchor {
    /// Stable semantic prompt identity; preferred across resize/reflow.
    pub generation: Option<u64>,
    /// Pane-local monotonic completion identity. This remains stable through
    /// resize/reflow even for integrations without stable prompt identities.
    pub key: u64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// First row below the owned editable prompt, when it can be proven from
    /// semantic metadata. None prevents decorating uncertain terminal text.
    pub output_top: Option<f32>,
    /// True when completion is painted on the following prompt's reserved row,
    /// making that row the visual boundary after the command output. In this
    /// state y is also the exclusive lower bound of the result region.
    pub separates_next_prompt: bool,
    /// Shell-reported exit status. None means the shell proved completion but
    /// did not expose a status; the renderer must use a neutral treatment.
    pub exit_code: Option<i32>,
    /// Terminal-measured execution duration, absent for boundary-only shells.
    pub elapsed_ms: Option<u64>,
}

/// Bound historical per-prompt extension UI state. This is intentionally small:
/// it only covers recent scrollback context rows and prevents an unbounded cache
/// when a terminal is left open for days.
pub const MAX_PROMPT_CONTEXT_HISTORY: usize = 256;

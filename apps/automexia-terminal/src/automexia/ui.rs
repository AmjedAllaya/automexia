//! Automexia-owned UI contracts shared by extension contributions.
//!
//! This module contains renderer-neutral geometry/data only. The terminal engine,
//! PTY and VT parser must not depend on extension implementations.

/// One semantic-prompt row geometry that an application contribution may decorate.
///
/// Historical anchors come from OSC 133 row metadata. The live anchor is
/// resolved from the current Prompt/PromptContinuation pair every frame and is
/// gated by generic shell prompt-readiness metadata; cursor geometry is only a
/// first-paint fallback before semantic rows arrive. Extensions therefore never
/// inspect terminal-engine row/cell types directly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PromptAnchor {
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

/// Bound historical per-prompt extension UI state. This is intentionally small:
/// it only covers recent scrollback context rows and prevents an unbounded cache
/// when a terminal is left open for days.
pub const MAX_PROMPT_CONTEXT_HISTORY: usize = 256;

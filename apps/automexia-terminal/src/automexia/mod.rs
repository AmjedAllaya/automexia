//! Automexia application platform.
//!
//! Rio remains the terminal engine in the 0.x line, but Automexia-owned
//! features live behind this boundary so they do not spread through VT/PTY,
//! renderer backends, or platform-specific windowing code.

pub mod api;
pub mod builtins;
pub mod marketplace;
pub mod migration;
pub mod runtime;
#[cfg(target_os = "windows")]
pub mod shell;
mod state;
pub mod theme;
pub mod ui;

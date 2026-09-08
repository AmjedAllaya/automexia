//! Automexia application platform.
//!
//! Rio remains the terminal engine in the 0.x line, but Automexia-owned
//! features live behind this boundary so they do not spread through VT/PTY,
//! renderer backends, or platform-specific windowing code.

pub mod api;
pub mod builtins;
#[cfg(not(target_arch = "wasm32"))]
pub mod connections;
#[cfg(not(target_arch = "wasm32"))]
pub mod ecosystem;
#[doc(hidden)]
pub mod export;
pub mod ghostty_migration;
pub mod marketplace;
pub mod migration;
#[cfg(not(target_arch = "wasm32"))]
pub mod preferences;
pub(crate) mod private_fs;
pub mod quick_actions;
pub mod runtime;
#[cfg(target_os = "windows")]
pub mod shell;
pub mod shell_integration;
#[doc(hidden)]
pub mod shortcut_preferences;
mod state;
pub mod suggestions;
pub mod theme;
pub mod ui;
#[cfg(feature = "visual-test-hooks")]
pub mod visual_test_hooks;

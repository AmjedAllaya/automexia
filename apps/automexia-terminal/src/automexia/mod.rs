//! Automexia application platform.
//!
//! Rio remains the terminal engine in the 0.x line, but Automexia-owned
//! features live behind this boundary so they do not spread through VT/PTY,
//! renderer backends, or platform-specific windowing code.

pub mod api;
pub mod browser_search;
pub mod builtins;
pub(crate) mod cli_process;
#[cfg(not(target_arch = "wasm32"))]
pub mod connections;
pub mod desktop_open;
#[cfg(not(target_arch = "wasm32"))]
pub mod ecosystem;
#[doc(hidden)]
pub mod export;
pub mod ghostty_migration;
pub mod google;
#[doc(hidden)]
pub mod local_tools;
pub mod marketplace;
pub mod migration;
#[cfg(not(target_arch = "wasm32"))]
pub mod preferences;
pub(crate) mod private_fs;
#[cfg(target_os = "windows")]
#[doc(hidden)]
pub mod prompt_discovery;
pub mod quick_actions;
pub mod runtime;
#[doc(hidden)]
pub mod semantic_surfaces;
#[cfg(target_os = "windows")]
pub mod shell;
pub mod shell_integration;
#[doc(hidden)]
pub mod shortcut_preferences;
mod state;
pub mod suggestions;
pub mod table_output;
pub mod theme;
pub mod ui;
#[cfg(feature = "visual-test-hooks")]
pub mod visual_test_hooks;

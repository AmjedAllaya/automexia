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
mod desktop_path;
pub mod directory_open;
#[cfg(not(target_arch = "wasm32"))]
pub mod ecosystem;
pub mod editor;
#[doc(hidden)]
pub mod export;
#[cfg(not(target_arch = "wasm32"))]
pub mod font_preferences;
pub mod ghostty_migration;
pub mod google;
#[doc(hidden)]
pub mod inline_tables;
#[cfg(not(target_arch = "wasm32"))]
pub mod interface_preferences;
mod kubernetes_probe;
#[doc(hidden)]
pub mod local_tools;
pub mod marketplace;
pub mod migration;
#[doc(hidden)]
pub mod output_semantics;
#[cfg(not(target_arch = "wasm32"))]
pub mod package_customizations;
#[cfg(not(target_arch = "wasm32"))]
pub mod preferences;
#[doc(hidden)]
pub mod presentation;
pub(crate) mod private_fs;
pub mod profiles;
#[cfg(target_os = "windows")]
#[doc(hidden)]
pub mod prompt_discovery;
pub mod quick_actions;
pub mod repository_open;
pub mod runtime;
#[doc(hidden)]
pub mod semantic_surfaces;
pub mod session_recovery;
pub mod settings_extensions;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub mod shell;
pub mod shell_integration;
#[doc(hidden)]
pub mod shortcut_preferences;
#[doc(hidden)]
pub mod ssh_helper;
pub mod ssh_integration;
pub(crate) mod ssh_scope;
pub(crate) mod ssh_upload;
pub mod ssh_wrapper;
mod state;
pub mod suggestions;
pub mod table_output;
pub mod theme;
pub mod theme_gallery;
pub mod theme_gallery_io;
pub mod ui;
#[cfg(feature = "visual-test-hooks")]
pub mod visual_test_hooks;
#[cfg(windows)]
pub(crate) mod windows_pipe_security;

//! Capability-free command-productivity contracts shared by terminal
//! composition, renderer-independent UI policy, and optional provider adapters.
//!
//! Shell editors remain authoritative. This crate performs no filesystem,
//! process, network, PTY, renderer, credential, or extension lifecycle work.

pub mod actions;
pub mod suggestions;

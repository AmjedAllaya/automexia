//! Capability-free connectivity contracts shared by the application and
//! independently disabled provider adapters.
//!
//! This crate owns validation, review, planning, authentication state, and
//! declarative workspace models. It owns no filesystem, process, network, PTY,
//! renderer, credential, or extension lifecycle authority.

pub mod connections;

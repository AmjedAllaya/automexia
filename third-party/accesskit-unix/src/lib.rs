#![cfg(all(unix, not(target_os = "macos")))]
// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

/// ## Compatibility with async runtimes
///
/// While this crate's API is purely blocking, it internally spawns asynchronous tasks on an executor.
///
/// Automexia uses the existing async-io executor. The unused alternative Tokio
/// runtime is excluded from this unpublished local adaptation.

#[cfg(not(feature = "async-io"))]
compile_error!("The \"async-io\" feature must be enabled.");

mod adapter;
mod atspi;
mod context;
mod executor;
mod queue;
mod util;

pub use adapter::Adapter;

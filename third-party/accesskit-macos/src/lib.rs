// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "macos")]
mod context;
#[cfg(target_os = "macos")]
mod filters;
#[cfg(target_os = "macos")]
mod node;
#[cfg(target_os = "macos")]
mod util;

#[cfg(target_os = "macos")]
mod adapter;
#[cfg(target_os = "macos")]
pub use adapter::Adapter;

#[cfg(target_os = "macos")]
mod event;
#[cfg(target_os = "macos")]
pub use event::QueuedEvents;

#[cfg(target_os = "macos")]
mod patch;
#[cfg(target_os = "macos")]
pub use patch::add_focus_forwarder_to_window_class;

#[cfg(target_os = "macos")]
mod subclass;
#[cfg(target_os = "macos")]
pub use subclass::SubclassingAdapter;

#[cfg(target_os = "macos")]
pub use objc2_foundation::{NSArray, NSObject, NSPoint};

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod boundary;

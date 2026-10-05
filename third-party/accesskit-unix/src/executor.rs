// Copyright 2024 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

// Derived from zbus.
// Copyright 2024 Zeeshan Ali Khan.
// Licensed under the MIT license (found in the LICENSE-MIT file).

use async_executor::Executor as AsyncExecutor;
use async_task::Task as AsyncTask;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// Automexia uses the existing async-io executor; there is no second runtime.
#[derive(Debug, Clone)]
pub(crate) struct Executor<'a> {
    executor: Arc<AsyncExecutor<'a>>,
}

impl Executor<'_> {
    pub(crate) fn spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
        _name: &str,
    ) -> Task<T> {
        Task(self.executor.spawn(future))
    }

    pub(crate) fn new() -> Self {
        Self {
            executor: Arc::new(AsyncExecutor::new()),
        }
    }

    pub(crate) async fn run<T>(&self, future: impl Future<Output = T>) -> T {
        self.executor.run(future).await
    }
}

/// Retains async_task's cancellation-on-drop semantics. Only explicit detach
/// relinquishes cancellation ownership; no blocking foreign join is introduced.
#[derive(Debug)]
pub(crate) struct Task<T>(AsyncTask<T>);

impl<T> Task<T> {
    #[allow(unused)]
    pub(crate) fn detach(self) {
        self.0.detach();
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).poll(cx)
    }
}

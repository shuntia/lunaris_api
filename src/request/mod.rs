use std::ops::{Deref, DerefMut};

use tokio::sync::oneshot;
use futures::future::BoxFuture;

use crate::util::error::NResult;

/// A synchronous, CPU-bound job.
/// Preferrably short-lived.
pub struct Job<F: FnOnce() + Send + 'static> {
    pub inner: F,
    pub priority: Priority,
}

/// An Asynchronous job, which is most beneficial for background tasks.
/// Use this instead of `Priority::Background`.
pub struct AsyncJob<F, Fut>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: core::future::Future<Output = ()> + Send + 'static,
{
    pub inner: F,
    pub priority: Priority,
    pub(crate) _phantom: core::marker::PhantomData<Fut>,
}

impl<F, Fut> AsyncJob<F, Fut>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: core::future::Future<Output = ()> + Send + 'static,
{
    pub fn new(c: F) -> Self {
        Self {
            inner: c,
            priority: Priority::Normal,
            _phantom: core::marker::PhantomData,
        }
    }
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    pub async fn exec(self) {
        (self.inner)().await
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Priority {
    /// Immediately launch task. Orchestrator pops this first as soon as possible.
    Immediate,
    /// Blocks the generation of current frame until this task is complete
    VideoFrame,
    /// Normal priority. Will execute task whenever other tasks are open.
    Normal,
    /// Deferred execution. Lowest priority.
    Deferred,
    /// Background execution. Will run regardless of contention.
    Background,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

impl<F: FnOnce() + Send + 'static> Job<F> {
    pub fn new(c: F) -> Self {
        Self {
            inner: c,
            priority: Priority::Normal,
        }
    }
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    pub fn exec(self) {
        (self.inner)()
    }
}

/// Job handle that completes whenever a task is completed.
#[repr(transparent)]
pub struct JobHandle {
    oneshot: oneshot::Receiver<()>,
}

impl Deref for JobHandle {
    type Target = oneshot::Receiver<()>;
    fn deref(&self) -> &Self::Target {
        &self.oneshot
    }
}

impl DerefMut for JobHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.oneshot
    }
}

pub trait OrchestratorHandle {
    /// Submit synchronous, CPU-bound job to the orchestrator
    fn submit_job<T: FnOnce() + Send + 'static>(&self, job: Job<T>) -> NResult;
    /// Submit asynchronous, IO-bound job to the orchestrator
    fn submit_async<F, Fut>(&self, job: AsyncJob<F, Fut>) -> NResult
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: core::future::Future<Output = ()> + Send + 'static;
    fn join_foreground(&self) -> NResult;
    /// Not reccomended. bg threads don't have an obligation to join.
    #[deprecated]
    fn join_all(&self) -> NResult;
    /// reconfigure amount of threads available at runtime
    fn set_threads(&self, default: usize, frame: usize, background: usize);
}

/// Object-safe orchestrator for plugins via dyn context.
pub trait DynOrchestrator: Send + Sync {
    fn submit_job_boxed(
        &self,
        job: Box<dyn FnOnce() + Send + 'static>,
        priority: Priority,
    ) -> NResult;
    fn submit_async_boxed(
        &self,
        fut: BoxFuture<'static, ()>,
        priority: Priority,
    ) -> NResult;
    fn join_foreground(&self) -> NResult;
    fn set_threads(&self, default: usize, frame: usize, background: usize);
}

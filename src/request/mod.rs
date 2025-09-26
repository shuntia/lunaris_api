use std::ops::{Deref, DerefMut};

use futures::future::BoxFuture;
use tokio::sync::oneshot;

use crate::util::error::Result;

/// A synchronous, CPU-bound job.
/// Preferrably short-lived.
pub struct Job<F: FnOnce() + Send + 'static> {
    pub inner: F,
    pub priority: Priority,
}

/// A helper struct to compose a Job.
pub struct OneshotJob<I, O, F>
where
    I: Send + 'static,
    O: Send + 'static,
    F: FnOnce(I) -> O + Send + 'static,
{
    pub param: Option<I>,
    pub oneshot: oneshot::Sender<O>,
    pub op: Option<F>,
    pub priority: Priority,
}

pub struct OneshotJobHandle<O>
where
    O: Send + 'static,
{
    pub oneshot: oneshot::Receiver<O>,
}

impl<O> OneshotJobHandle<O>
where
    O: Send + 'static,
{
    pub fn new(o: oneshot::Receiver<O>) -> Self {
        Self { oneshot: o }
    }
}

impl<O: Send + 'static> Deref for OneshotJobHandle<O> {
    type Target = oneshot::Receiver<O>;
    fn deref(&self) -> &Self::Target {
        &self.oneshot
    }
}

impl<O: Send + 'static> DerefMut for OneshotJobHandle<O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.oneshot
    }
}

impl<I, O, F> OneshotJob<I, O, F>
where
    I: Send,
    O: Send,
    F: FnOnce(I) -> O + Send + 'static,
{
    pub fn from_params(args: I, f: F, oneshot: oneshot::Sender<O>) -> Self {
        OneshotJob {
            param: Some(args),
            oneshot,
            op: Some(f),
            priority: Priority::Normal,
        }
    }
    pub fn with_sender(args: I, f: F, oneshot: oneshot::Sender<O>, priority: Priority) -> Self {
        OneshotJob {
            param: Some(args),
            oneshot,
            op: Some(f),
            priority,
        }
    }
    pub fn new(i: I, f: F, priority: Priority) -> (OneshotJob<I, O, F>, OneshotJobHandle<O>) {
        let (sender, receiver) = oneshot::channel();
        (
            Self::with_sender(i, f, sender, priority),
            OneshotJobHandle::new(receiver),
        )
    }
    pub fn exec(self) {
        if let Some(s) = self.op
            && let Some(p) = self.param
        {
            let _ = self.oneshot.send((s)(p));
        }
    }
}

impl<I, O, F> From<OneshotJob<I, O, F>> for Job<Box<dyn FnOnce() + Send + 'static>>
where
    I: Send + 'static,
    O: Send + 'static,
    F: FnOnce(I) -> O + Send + 'static,
{
    fn from(value: OneshotJob<I, O, F>) -> Self {
        let priority = value.priority;
        Job {
            inner: Box::new(|| value.exec()),
            priority,
        }
    }
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

    #[inline(always)]
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

    #[inline(always)]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[inline(always)]
    pub fn exec(self) {
        (self.inner)()
    }
}

#[repr(transparent)]
pub struct ParamJobHandle<T> {
    oneshot: oneshot::Receiver<T>,
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
pub struct OrchestratorProfile {
    pub immediate: u64,
    pub normal: u64,
    pub deferred: u64,
    pub frame: u64,
    pub running_tasks: u64,
}

/// Object-safe orchestrator for plugins via dyn context.
pub trait DynOrchestrator: Send + Sync {
    fn submit_job_boxed(
        &self,
        job: Box<dyn FnOnce() + Send + 'static>,
        priority: Priority,
    ) -> Result;
    fn submit_async_boxed(&self, fut: BoxFuture<'static, ()>, priority: Priority) -> Result;
    fn join_foreground(&self) -> Result;
    fn set_threads(&self, default: usize, frame: usize, background: usize);
    fn profile(&self) -> OrchestratorProfile;
}

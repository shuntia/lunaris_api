pub struct Job<F: FnOnce() -> () + Send + 'static> {
    pub inner: F,
    pub priority: Priority,
}

pub struct AsyncJob<F: AsyncFnOnce() -> () + Send + 'static> {
    pub inner: F,
    pub priority: Priority,
}

impl<F: AsyncFnOnce() -> () + Send + 'static> AsyncJob<F> {
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
    pub async fn exec(self) -> () {
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

impl<F: FnOnce() -> () + Send + 'static> Job<F> {
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

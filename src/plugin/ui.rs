use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use arc_swap::{ArcSwap, DefaultStrategy, Guard};
use bevy_ecs::resource::Resource;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// UI data wrapper that hides the backing storage choice.
/// This is for sharing shared state in between the UI loop and ECS loop.
#[derive(Resource)]
pub struct UiContext<S>
where
    S: UiContextStorage,
{
    storage: Arc<S>,
}

impl<S> UiContext<S>
where
    S: UiContextStorage,
{
    /// Create a context from an arbitrary storage backend.
    pub fn new(storage: S) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }

    /// Obtain a read handle without knowing the concrete storage type.
    pub fn read(&self) -> S::ReadGuard<'_> {
        self.storage.read()
    }

    /// Obtain a write handle without knowing the concrete storage type.
    pub fn write(&self) -> S::WriteGuard<'_> {
        self.storage.write()
    }

    /// Access the underlying storage for feature-specific helpers.
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

impl<C> UiContext<ArcSwapStorage<C>>
where
    C: Clone + Send + Sync,
{
    /// Construct a clone-on-write context backed by `ArcSwap`.
    pub fn new_clonable(content: C) -> Self {
        Self::new(ArcSwapStorage::new(content))
    }

    /// Fetch a snapshot of the `ArcSwap`. This is guaranteed to be lock free.
    pub fn snapshot(&self) -> ArcSwapReadGuard<'_, C> {
        self.storage.read()
    }

    /// Fetch a full, owned snapshot of the `ArcSwap`.
    ///
    /// This always clones the contents, which can be expensive for large owned data.
    pub fn full_snapshot(&self) -> ArcSwapWriteGuard<'_, C> {
        self.storage.write()
    }
}

impl<C> UiContext<RwLockStorage<C>>
where
    C: Send + Sync,
{
    /// Construct a context backed by `RwLock` for in-place mutation.
    pub fn new_rwlock(content: C) -> Self {
        Self::new(RwLockStorage::new(content))
    }

    /// Fetch the locking read guard of `RwLockStorage`.
    pub fn read_locking(&self) -> RwLockReadGuard<'_, C> {
        self.storage.inner.read()
    }

    /// Fetch the locking write guard from the `RwLockStorage`.
    pub fn write_locking(&self) -> RwLockWriteGuard<'_, C> {
        self.storage.inner.write()
    }

    /// Try to fetch the locking read guard of `RwLockStorage`.
    pub fn try_read_locking(&self) -> Option<RwLockReadGuard<'_, C>> {
        self.storage.inner.try_read()
    }

    /// Try to fetch the locking write guard from the `RwLockStorage`.
    pub fn try_write_locking(&self) -> Option<RwLockWriteGuard<'_, C>> {
        self.storage.inner.try_write()
    }
}

pub trait UiContextStorage: Send + Sync {
    type State: Send + Sync;
    type WriteGuard<'a>: Deref<Target = Self::State> + DerefMut<Target = Self::State> + 'a
    where
        Self: 'a;
    type ReadGuard<'a>: Deref<Target = Self::State> + 'a
    where
        Self: 'a;
    fn write(&self) -> Self::WriteGuard<'_>;
    fn read(&self) -> Self::ReadGuard<'_>;
}

pub struct ArcSwapStorage<C: Clone + Send + Sync> {
    inner: ArcSwap<C>,
}

impl<C> ArcSwapStorage<C>
where
    C: Clone + Send + Sync,
{
    /// Create a new clonable storage.
    /// It gets copied on write.
    pub fn new(content: C) -> ArcSwapStorage<C> {
        ArcSwapStorage {
            inner: ArcSwap::from_pointee(content),
        }
    }
}

pub struct ArcSwapWriteGuard<'a, C: Clone + Send + Sync> {
    swap: &'a ArcSwap<C>,
    writable: C,
}

impl<C> ArcSwapWriteGuard<'_, C>
where
    C: Clone + Send + Sync,
{
    /// Create a write guard from an ArcSwap.
    pub fn from_arcswap(swap: &ArcSwap<C>) -> ArcSwapWriteGuard<'_, C> {
        ArcSwapWriteGuard {
            swap,
            writable: swap.load_full().deref().clone(),
        }
    }

    /// Swaps itself in place of the ArcSwap, and returns the Arc it swapped
    /// out.
    pub fn swap(self) -> Arc<C> {
        self.swap.swap(Arc::new(self.writable))
    }
}

impl<C> Deref for ArcSwapWriteGuard<'_, C>
where
    C: Clone + Send + Sync,
{
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.writable
    }
}

impl<C> DerefMut for ArcSwapWriteGuard<'_, C>
where
    C: Clone + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.writable
    }
}

pub struct ArcSwapReadGuard<'a, C: Clone + Send + Sync> {
    _swap: &'a ArcSwap<C>,
    snapshot: Guard<Arc<C>, DefaultStrategy>,
}

impl<C> ArcSwapReadGuard<'_, C>
where
    C: Clone + Send + Sync,
{
    pub fn from_arcswap<'a>(swap: &'a ArcSwap<C>) -> ArcSwapReadGuard<'a, C> {
        ArcSwapReadGuard {
            _swap: swap,
            snapshot: swap.load(),
        }
    }
}

impl<C> Deref for ArcSwapReadGuard<'_, C>
where
    C: Clone + Send + Sync,
{
    type Target = C;
    fn deref(&self) -> &Self::Target {
        self.snapshot.deref()
    }
}

impl<C> UiContextStorage for ArcSwapStorage<C>
where
    C: Clone + Send + Sync,
{
    type State = C;
    type WriteGuard<'a>
        = ArcSwapWriteGuard<'a, C>
    where
        C: 'a,
        Self: 'a;
    type ReadGuard<'a>
        = ArcSwapReadGuard<'a, C>
    where
        C: 'a,
        Self: 'a;
    fn read(&self) -> Self::ReadGuard<'_> {
        ArcSwapReadGuard::from_arcswap(&self.inner)
    }
    fn write(&self) -> Self::WriteGuard<'_> {
        ArcSwapWriteGuard::from_arcswap(&self.inner)
    }
}

pub struct RwLockStorage<C: Send + Sync> {
    inner: RwLock<C>,
}

impl<C> RwLockStorage<C>
where
    C: Send + Sync,
{
    pub fn new(content: C) -> Self {
        Self {
            inner: RwLock::new(content),
        }
    }
}

impl<C> UiContextStorage for RwLockStorage<C>
where
    C: Send + Sync,
{
    type State = C;
    type ReadGuard<'a>
        = RwLockReadGuard<'a, C>
    where
        C: 'a,
        Self: 'a;
    type WriteGuard<'a>
        = RwLockWriteGuard<'a, C>
    where
        C: 'a,
        Self: 'a;
    fn read(&self) -> Self::ReadGuard<'_> {
        self.inner.read()
    }
    fn write(&self) -> Self::WriteGuard<'_> {
        self.inner.write()
    }
}

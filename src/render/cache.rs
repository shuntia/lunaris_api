use std::{
    num::NonZeroUsize,
    sync::{Arc, atomic::AtomicU32},
};

use arc_swap::ArcSwap;
use bevy_ecs::entity::Entity;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::time::Instant;
use tracing::warn;
use wgpu::Texture;

use crate::{
    prelude::Result,
    render::{
        RawImage,
        image::{CompressedImage, CompressionStrategy},
    },
};

/// Tiered cache for fully rendered frames.
pub struct TieredCache {
    capacity: (usize, usize, usize),
    low: DashMap<Entity, (AccessToken, CompressedImage)>,
    med: DashMap<Entity, (AccessToken, RawImage)>,
    high: DashMap<Entity, (AccessToken, Texture)>,
}

static COMPRESSION_STRATEGY: RwLock<CompressionStrategy> = RwLock::new(CompressionStrategy::Qoi);

impl TieredCache {
    pub fn with_capacity(low: NonZeroUsize, med: NonZeroUsize, high: NonZeroUsize) -> TieredCache {
        if low < med || med < high {
            warn!("Inverted capacities for caches. This may inefficiently take up memory.");
        }
        TieredCache {
            capacity: (low.into(), med.into(), high.into()),
            low: DashMap::new(),
            med: DashMap::new(),
            high: DashMap::new(),
        }
    }
    pub fn demote(&self, entity: Entity) -> Result {
        if self.low.contains_key(&entity) {
            self.low.remove(&entity);
            return Ok(());
        }
        if self.med.contains_key(&entity) {
            let (_, (tok, img)) = unsafe { self.med.remove(&entity).unwrap_unchecked() };
            self.low
                .insert(entity, (tok, img.compress(*COMPRESSION_STRATEGY.read())?));
            return Ok(());
        }
        if self.high.contains_key(&entity) {
            let (_, (tok, tex)) = unsafe { self.high.remove(&entity).unwrap_unchecked() };
            self.med.insert(entity, (tok, tex.into()));
            return Ok(());
        }
        Err(crate::prelude::LunarisError::NotFound {
            item: format!("Rendered Result for Entity: {entity}"),
        })
    }
    pub fn promote(&self, entity: Entity) -> Result {
        if self.low.contains_key(&entity) {
            let (_, (tok, img)) = unsafe { self.low.remove(&entity).unwrap_unchecked() };
            let img = img.decompress()?;
            self.med.insert(entity, (tok, img));
            return Ok(());
        }
        if self.med.contains_key(&entity) {
            let (_, (tok, img)) = unsafe { self.med.remove(&entity).unwrap_unchecked() };
            let tex = img.into();
            self.high.insert(entity, (tok, tex));
            return Ok(());
        }
        Err(crate::prelude::LunarisError::NotFound {
            item: format!("Rendered Result for Entity: {entity}"),
        })
    }
    pub fn update(&self) -> Result {
        let (low_cap, med_cap, high_cap) = self.capacity;
        loop {
            let mut changed = false;

            let high_snapshot: Vec<_> = self
                .high
                .iter()
                .map(|ref_multi| {
                    let (entity, (token, _)) = ref_multi.pair();
                    (*entity, AccessTokenSnapshot::from(token))
                })
                .collect();

            let med_snapshot: Vec<_> = self
                .med
                .iter()
                .map(|ref_multi| {
                    let (entity, (token, _)) = ref_multi.pair();
                    (*entity, AccessTokenSnapshot::from(token))
                })
                .collect();

            let low_snapshot: Vec<_> = self
                .low
                .iter()
                .map(|ref_multi| {
                    let (entity, (token, _)) = ref_multi.pair();
                    (*entity, AccessTokenSnapshot::from(token))
                })
                .collect();

            if self.high.len() > high_cap {
                if let Some((entity, _)) = high_snapshot.into_iter().max_by(|a, b| a.1.cmp(&b.1)) {
                    self.demote(entity)?;
                    changed = true;
                }
            } else if self.med.len() > med_cap {
                if let Some((entity, _)) = med_snapshot.into_iter().max_by(|a, b| a.1.cmp(&b.1)) {
                    self.demote(entity)?;
                    changed = true;
                }
            } else if self.low.len() > low_cap {
                if let Some((entity, _)) = low_snapshot.into_iter().max_by(|a, b| a.1.cmp(&b.1)) {
                    self.demote(entity)?;
                    changed = true;
                }
            } else if self.high.len() < high_cap {
                if let Some((entity, _)) = med_snapshot.into_iter().min_by(|a, b| a.1.cmp(&b.1)) {
                    self.promote(entity)?;
                    changed = true;
                }
            } else if self.med.len() < med_cap {
                if let Some((entity, _)) = low_snapshot.into_iter().min_by(|a, b| a.1.cmp(&b.1)) {
                    self.promote(entity)?;
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        Ok(())
    }
}

struct AccessTokenSnapshot {
    touched: Arc<Instant>,
    freq: u32,
}

impl From<&AccessToken> for AccessTokenSnapshot {
    fn from(value: &AccessToken) -> Self {
        AccessTokenSnapshot {
            touched: value.last_touched.load_full(),
            freq: value
                .touched_freq
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl AccessTokenSnapshot {
    fn score(&self) -> u32 {
        let since = self.touched.elapsed().as_millis() as u32;
        let freq = self.freq.max(1);
        since / freq
    }
}

impl Ord for AccessTokenSnapshot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score()
            .cmp(&other.score())
            .then_with(|| self.freq.cmp(&other.freq))
    }
}

impl PartialOrd for AccessTokenSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for AccessTokenSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for AccessTokenSnapshot {}

pub struct AccessToken {
    last_touched: ArcSwap<Instant>,
    touched_freq: AtomicU32,
}

impl AccessToken {
    pub fn increment(&self) {
        self.last_touched.store(Arc::new(Instant::now()));
        self.touched_freq
            .fetch_add(1, std::sync::atomic::Ordering::Release);
    }
    pub fn score(&self) -> u32 {
        let since = self.last_touched.load().elapsed().as_millis() as u32;
        let freq = self
            .touched_freq
            .load(std::sync::atomic::Ordering::Relaxed)
            .max(1);
        since / freq
    }
}

impl Ord for AccessToken {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score().cmp(&other.score())
    }
}

impl PartialOrd for AccessToken {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for AccessToken {
    fn eq(&self, other: &Self) -> bool {
        self.score() == other.score()
    }
}

impl Eq for AccessToken {}

use std::{any::Any, collections::HashMap, path::PathBuf, sync::Arc};

use crate::{render::RawImage, timeline::TimelineSpan, util::error::Result};

use bevy_ecs::{component::Component, entity::Entity};

#[derive(Component, Debug)]
pub struct TimelineElement {
    /// Track number of Timeline Element, or in other words, the Z-index.
    pub track_num: u64,
    pub position: TimelineSpan,
}

#[derive(Component, Debug)]
pub struct BindTo {
    pub id: Entity,
}

#[derive(Component, Default, Debug, Clone)]
pub struct Properties {
    pub properties: HashMap<String, Property>,
}

impl Properties {
    pub fn into_inner(self) -> HashMap<String, Property> {
        self.properties
    }

    pub fn get(&self, key: &str) -> Option<&Property> {
        self.properties.get(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: Property) -> Option<Property> {
        self.properties.insert(key.into(), value)
    }

    pub fn remove(&mut self, key: &str) -> Option<Property> {
        self.properties.remove(key)
    }
}

impl From<Properties> for HashMap<String, Property> {
    fn from(val: Properties) -> Self {
        val.properties
    }
}

#[derive(Component, Debug)]
pub struct Renderable {
    pub render_result: Result<RawImage>,
}

#[derive(Clone, Debug)]
pub enum Property {
    String(String),
    Integer(u64),
    Curve(Vec<u64>),
    Float(f64),
    Entity(Entity),
    Path(PathBuf),
    Custom(Arc<dyn Any + Send + Sync>),
}

impl Property {
    pub fn get_variant_name(&self) -> &'static str {
        match &self {
            Self::String(_) => "String",
            Self::Integer(_) => "Integer",
            Self::Curve(_) => "Curve",
            Self::Float(_) => "Float",
            Self::Entity(_) => "Entity",
            Self::Path(_) => "Path",
            Self::Custom(_) => "Custom",
        }
    }
    pub fn custom<T: Any + Send + Sync>(value: T) -> Self {
        Self::Custom(Arc::new(value))
    }

    pub fn as_custom<T: Any + Send + Sync>(&self) -> Option<&T> {
        match self {
            Self::Custom(inner) => inner.downcast_ref::<T>(),
            _ => None,
        }
    }
}

impl PartialEq for Property {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Curve(a), Self::Curve(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Entity(a), Self::Entity(b)) => a == b,
            (Self::Path(a), Self::Path(b)) => a == b,
            (Self::Custom(a), Self::Custom(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

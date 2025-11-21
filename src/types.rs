use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

use lunaris_ecs::prelude::*;

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

#[derive(Clone, Debug)]
pub enum Property {
    String(String),
    Integer(u64),
    Float(f64),
    Entity(Entity),
    Dynamic(DynamicProperty),
    Path(PathBuf),
    Custom(Arc<dyn Any + Send + Sync>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicProperty {
    timestamp: u64,
    inner: Box<Property>,
}

impl Deref for DynamicProperty {
    type Target = Property;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for DynamicProperty {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub enum SizedProperty {
    String(String),
    Integer(u64),
    Float(f64),
    Entity(Entity),
    Path(PathBuf),
}

impl Property {
    pub fn get_variant_name(&self) -> &'static str {
        match &self {
            Self::String(_) => "String",
            Self::Integer(_) => "Integer",
            Self::Float(_) => "Float",
            Self::Entity(_) => "Entity",
            Self::Dynamic(content) => content.get_variant_name(),
            Self::Path(_) => "Path",
            Self::Custom(_) => "Custom",
        }
    }

    pub fn custom<T: Any + Send + Sync>(value: T) -> Self {
        Self::Custom(Arc::new(value))
    }

    pub fn as_custom<T: Any>(&self) -> Option<&T> {
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
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Entity(a), Self::Entity(b)) => a == b,
            (Self::Path(a), Self::Path(b)) => a == b,
            (Self::Dynamic(a), Self::Dynamic(b)) => a == b,
            (Self::Custom(a), Self::Custom(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

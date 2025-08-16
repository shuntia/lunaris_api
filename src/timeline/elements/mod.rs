use std::{collections::HashMap, path::PathBuf};

use bevy_ecs::{component::Component, entity::Entity};
use once_cell::sync::OnceCell;

use crate::{
    render::RawImage,
    timeline::TimelineSpan,
    utils::errors::{LunarisError, Result},
};

#[derive(Component)]
pub struct TimelineElement {
    /// Track number of Timeline Element, or in other words, the Z-index.
    pub track_num: u64,
    pub position: TimelineSpan,
}

#[derive(Component, Default)]
pub struct Properties {
    pub properties: HashMap<String, Property>,
    checker: OnceCell<fn(HashMap<String, Property>) -> Vec<(String, LunarisError)>>,
}

impl Properties {
    pub fn into_inner(self) -> HashMap<String, Property> {
        self.properties
    }
}

impl Into<HashMap<String, Property>> for Properties {
    fn into(self) -> HashMap<String, Property> {
        self.properties
    }
}

/// Trait for
#[derive(Component)]
pub struct Renderable {
    pub previous_frame: Option<RawImage>,
    pub render_result: Result<RawImage>,
}

#[derive(Clone, PartialEq)]
pub enum Property {
    String(String),
    Integer(u64),
    Curve(Vec<u64>),
    Float(f64),
    Entity(Entity),
    Path(PathBuf),
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
        }
    }
}

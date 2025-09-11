use std::collections::HashMap;
use std::path::PathBuf;

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

#[derive(Component, Default, Debug)]
pub struct Properties {
    pub properties: HashMap<String, Property>,
}

impl Properties {
    pub fn into_inner(self) -> HashMap<String, Property> {
        self.properties
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

#[derive(Clone, PartialEq, Debug)]
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

use bevy_ecs::world::World;
use slab::Slab;

use crate::plugin::Plugin;

pub struct LunarisContext {
    pub title: String,
    pub world: World,
}

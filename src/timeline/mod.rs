use bevy_ecs::component::Component;

pub mod elements;

#[derive(Debug)]
pub struct TimelineSpan {
    pub start: u64,
    pub end: u64,
}

#[derive(Component)]
pub struct Playhead {
    pub current: u64,
}

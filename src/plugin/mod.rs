use bevy_ecs::prelude::*;
use egui::{MenuBar, Ui};

pub trait Plugin {
    fn new() -> Self
    where
        Self: Sized;
    /// Plugin's unique name
    fn name(&self) -> &'static str;
    /// Initialize plugin
    fn init(&self, ctx: PluginContext);
    fn update_world(&mut self, ctx: PluginContext);
    fn report(&self, ctx: PluginContext) -> PluginReport;
    fn shutdown(self, ctx: PluginContext);
    fn reset(&mut self, ctx: PluginContext);
    #[allow(unused)]
    fn register_menu(&self, menu_bar: &mut MenuBar) {}
}

pub trait Gui: Plugin {
    fn ui(&self, ui: &mut Ui, ctx: PluginContext);
}

pub enum PluginReport {
    Uninit,
    Operational,
    InvalidState,
    Fatal,
    Dead,
}

#[derive(Component)]
pub struct PluginContext<'a> {
    pub world: &'a mut World,
}

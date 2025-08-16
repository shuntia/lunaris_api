use bevy_ecs::prelude::*;
use egui::{MenuBar, Ui};

pub trait Plugin {
    /// Plugin's unique name
    fn name(&self) -> &'static str;
    /// Initialize plugin
    fn init(ctx: PluginContext) -> Self
    where
        Self: Sized;
    fn ui(&mut self, ctx: PluginContext, ui: Ui);
    fn update_world(&mut self, ctx: PluginContext);
    fn report(&self, ctx: PluginContext) -> PluginReport;
    fn shutdown(self, ctx: PluginContext);
    fn reset(&mut self, ctx: PluginContext);
    #[allow(unused)]
    fn register_menu(&self, menu_bar: &mut MenuBar) {}
}

pub trait GuiPlugin: Plugin {
    fn paint(&self, ui: Ui, ctx: PluginContext);
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
    pub local: &'a mut World,
}

use bevy_ecs::prelude::*;
use egui::{MenuBar, Ui};

use crate::{
    render::RawImage,
    request::DynOrchestrator,
    util::error::{NResult, Result},
};

// Object-safe plugin surface that the host can store behind dyn.
pub trait Plugin {
    fn new() -> Self
    where
        Self: Sized;
    fn name(&self) -> &'static str;
    fn init(&self, ctx: PluginContext<'_>) -> NResult;
    fn update_world(&mut self, ctx: PluginContext<'_>) -> NResult;
    fn report(&self, ctx: PluginContext<'_>) -> PluginReport;
    fn shutdown(&mut self, ctx: PluginContext<'_>);
    fn reset(&mut self, ctx: PluginContext<'_>);
    #[allow(unused)]
    fn register_menu(&self, _menu_bar: &mut MenuBar) {}
}

pub trait Renderer {
    fn render_entity(frame_num: u64, entity: Entity, ctx: PluginContext<'_>) -> Result<RawImage>;
}

// Optional GUI capability; separate trait keeps core Plugin object-safe.
pub trait Gui: Plugin {
    fn ui(&self, ui: &mut Ui, ctx: PluginContext<'_>);
}

pub enum PluginReport {
    Uninit,
    Operational,
    InvalidState,
    Fatal,
    Dead,
}

#[derive(bevy_ecs::prelude::Component)]
pub struct PluginContext<'a> {
    pub world: &'a mut World,
    pub orch: &'a dyn DynOrchestrator,
}

// Registration records collected via `inventory`.
pub struct PluginRegistration {
    pub name: &'static str,
    pub build: fn() -> Box<dyn Plugin>,
}

pub struct GuiRegistration {
    pub name: &'static str,
    pub build: fn() -> Box<dyn PluginGui>,
}

// Supertrait for convenience when we need a single box that supports both.
pub trait PluginGui: Plugin + Gui {}
impl<T: Plugin + Gui> PluginGui for T {}

inventory::collect!(PluginRegistration);
inventory::collect!(GuiRegistration);

// Compile-time check + registration macros
#[macro_export]
macro_rules! register_plugin {
    ($ty:ty, name: $name:expr) => {
        const _: fn() = || {
            fn assert_impl<T: $crate::plugin::Plugin>() {}
            let _ = assert_impl::<$ty>;
        };
        inventory::submit! {
            $crate::plugin::PluginRegistration {
                name: $name,
                build: || Box::new(<$ty>::new()),
            }
        }
    };
}

#[macro_export]
macro_rules! register_plugin_gui {
    ($ty:ty, name: $name:expr) => {
        const _: fn() = || {
            fn assert_impl<T: $crate::plugin::PluginGui>() {}
            let _ = assert_impl::<$ty>;
        };
        // Always register as core Plugin too
        $crate::register_plugin!($ty, name: $name);
        inventory::submit! {
            $crate::plugin::GuiRegistration {
                name: $name,
                build: || Box::new(<$ty>::new()),
            }
        }
    };
}

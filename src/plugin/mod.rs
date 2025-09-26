use bevy_ecs::{prelude::*, system::BoxedSystem};
use debug_unreachable::debug_unreachable;
use egui::{MenuBar, Ui};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    render::RawImage,
    request::DynOrchestrator,
    timeline::elements::{Properties, Property},
    util::error::Result,
};

pub mod ui;

// Object-safe plugin surface that the host can store behind dyn.
pub trait Plugin: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;
    fn name(&self) -> &'static str;
    fn init(&self, ctx: PluginContext<'_>) -> Result;
    fn update_world(&mut self, ctx: PluginContext<'_>) -> Result;
    fn report(&self, ctx: PluginContext<'_>) -> PluginReport;
    fn shutdown(&mut self, ctx: PluginContext<'_>);
    fn reset(&mut self, ctx: PluginContext<'_>);
    #[allow(unused)]
    fn register_menu(&self, _menu_bar: &mut MenuBar) {}
}

pub trait SystemPlugin: System {
    type UndoTok: Event + Serialize + for<'de> Deserialize<'de>;

    fn undo_system(&mut self) -> Option<BoxedSystem<(), ()>> {
        None
    }
}

pub struct RenderJob {
    pub frame: u64,
    pub entity: Entity,
    pub parameters: Properties,
}

impl RenderJob {
    pub fn new(frame: u64, entity: Entity, parameters: Properties) -> Self {
        Self {
            frame,
            entity,
            parameters,
        }
    }

    pub fn parameter(&self, key: &str) -> Option<&Property> {
        self.parameters.get(key)
    }

    pub fn parameters(&self) -> &Properties {
        &self.parameters
    }

    pub fn into_parameters(self) -> Properties {
        self.parameters
    }
}

pub type RenderTask = BoxFuture<'static, Result<RawImage>>;

pub trait Renderer: Plugin {
    fn schedule_render(&self, job: RenderJob) -> Result<RenderTask>;
}

// Optional GUI capability; separate trait keeps core Plugin object-safe.
pub trait Gui: Plugin {
    fn ui(&self, ui: &mut Ui, ctx: PluginContext<'_>);
}

pub trait Skeleton: Gui {
    fn skeleton(ui: &mut Ui);
}

pub type PluginGui = dyn Gui;

pub enum PluginReport {
    Uninit,
    Operational,
    InvalidState,
    Fatal,
    Dead,
}

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
    pub build: fn() -> Box<dyn Gui>,
}

pub struct RendererRegistration {
    pub name: &'static str,
    pub build: fn() -> Box<dyn Renderer>,
}

inventory::collect!(PluginRegistration);
inventory::collect!(GuiRegistration);
inventory::collect!(RendererRegistration);

// Optional: System contribution capability. Plugins that implement this trait can
// register ECS systems/resources/events into the host `World`.
pub trait SystemContributor: Send + Sync {
    fn contribute(&self, world: &mut World) -> Result;
}

pub struct SystemRegistration {
    pub name: &'static str,
    pub build: fn() -> Arc<dyn SystemContributor + Send + Sync>,
}

inventory::collect!(SystemRegistration);

#[macro_export]
macro_rules! submit_raw {
    ($expr:expr) => {
        inventory::submit! { $expr }
    };
}

// Internal adapters that allow sharing a single heavy plugin instance
// across multiple feature registrations without changing the external
// Box<dyn Trait> registration API.
#[doc(hidden)]
pub struct __ArcPluginAdapter<T> {
    inner: std::sync::Arc<parking_lot::RwLock<T>>,
}

impl<T> __ArcPluginAdapter<T> {
    pub fn new_with_shared(inner: std::sync::Arc<parking_lot::RwLock<T>>) -> Self {
        Self { inner }
    }
}

impl<T: Plugin> Plugin for __ArcPluginAdapter<T> {
    fn new() -> Self
    where
        Self: Sized,
    {
        unsafe {
            debug_unreachable!("__ArcPluginAdapter is constructed via export_plugin! macro");
        }
    }
    fn name(&self) -> &'static str {
        if let Some(guard) = self.inner.try_read() {
            Plugin::name(&*guard)
        } else {
            "<locked>"
        }
    }
    fn init(&self, ctx: PluginContext<'_>) -> Result {
        let guard = self.inner.read();
        Plugin::init(&*guard, ctx)
    }
    fn update_world(&mut self, ctx: PluginContext<'_>) -> Result {
        let mut guard = self.inner.write();
        Plugin::update_world(&mut *guard, ctx)
    }
    fn report(&self, ctx: PluginContext<'_>) -> PluginReport {
        if let Some(guard) = self.inner.try_read() {
            Plugin::report(&*guard, ctx)
        } else {
            PluginReport::Operational
        }
    }
    fn shutdown(&mut self, ctx: PluginContext<'_>) {
        let mut guard = self.inner.write();
        Plugin::shutdown(&mut *guard, ctx)
    }
    fn reset(&mut self, ctx: PluginContext<'_>) {
        let mut guard = self.inner.write();
        Plugin::reset(&mut *guard, ctx)
    }
    fn register_menu(&self, menu_bar: &mut MenuBar) {
        if let Some(guard) = self.inner.try_read() {
            Plugin::register_menu(&*guard, menu_bar)
        }
    }
}

#[doc(hidden)]
pub struct __ArcPluginGuiAdapter<T> {
    inner: std::sync::Arc<parking_lot::RwLock<T>>,
}

impl<T> __ArcPluginGuiAdapter<T> {
    pub fn new_with_shared(inner: std::sync::Arc<parking_lot::RwLock<T>>) -> Self {
        Self { inner }
    }
}

impl<T: Plugin> Plugin for __ArcPluginGuiAdapter<T> {
    fn new() -> Self
    where
        Self: Sized,
    {
        unsafe {
            debug_unreachable!("__ArcPluginGuiAdapter is constructed via export_plugin! macro")
        };
    }
    fn name(&self) -> &'static str {
        if let Some(guard) = self.inner.try_read() {
            Plugin::name(&*guard)
        } else {
            "<locked>"
        }
    }
    fn init(&self, ctx: PluginContext<'_>) -> Result {
        let guard = self.inner.read();
        Plugin::init(&*guard, ctx)
    }
    fn update_world(&mut self, ctx: PluginContext<'_>) -> Result {
        let mut guard = self.inner.write();
        Plugin::update_world(&mut *guard, ctx)
    }
    fn report(&self, ctx: PluginContext<'_>) -> PluginReport {
        if let Some(guard) = self.inner.try_read() {
            Plugin::report(&*guard, ctx)
        } else {
            PluginReport::Operational
        }
    }
    fn shutdown(&mut self, ctx: PluginContext<'_>) {
        let mut guard = self.inner.write();
        Plugin::shutdown(&mut *guard, ctx)
    }
    fn reset(&mut self, ctx: PluginContext<'_>) {
        let mut guard = self.inner.write();
        Plugin::reset(&mut *guard, ctx)
    }
    fn register_menu(&self, menu_bar: &mut MenuBar) {
        if let Some(guard) = self.inner.try_read() {
            Plugin::register_menu(&*guard, menu_bar)
        }
    }
}

impl<T: Plugin + Gui> Gui for __ArcPluginGuiAdapter<T> {
    fn ui(&self, ui: &mut Ui, ctx: PluginContext<'_>) {
        if let Some(guard) = self.inner.try_read() {
            Gui::ui(&*guard, ui, ctx)
        } else {
            // Skip UI this frame if locked by a writer
        }
    }
}

#[doc(hidden)]
pub struct __ArcPluginRendererAdapter<T> {
    inner: std::sync::Arc<parking_lot::RwLock<T>>,
}

impl<T> __ArcPluginRendererAdapter<T> {
    pub fn new_with_shared(inner: std::sync::Arc<parking_lot::RwLock<T>>) -> Self {
        Self { inner }
    }
}

impl<T: Plugin + Renderer> Plugin for __ArcPluginRendererAdapter<T> {
    fn new() -> Self
    where
        Self: Sized,
    {
        unsafe {
            debug_unreachable!(
                "__ArcPluginRendererAdapter is constructed via export_plugin! macro"
            );
        }
    }

    fn name(&self) -> &'static str {
        if let Some(guard) = self.inner.try_read() {
            Plugin::name(&*guard)
        } else {
            "<locked>"
        }
    }

    fn init(&self, ctx: PluginContext<'_>) -> Result {
        let guard = self.inner.read();
        Plugin::init(&*guard, ctx)
    }

    fn update_world(&mut self, ctx: PluginContext<'_>) -> Result {
        let mut guard = self.inner.write();
        Plugin::update_world(&mut *guard, ctx)
    }

    fn report(&self, ctx: PluginContext<'_>) -> PluginReport {
        if let Some(guard) = self.inner.try_read() {
            Plugin::report(&*guard, ctx)
        } else {
            PluginReport::Operational
        }
    }

    fn shutdown(&mut self, ctx: PluginContext<'_>) {
        let mut guard = self.inner.write();
        Plugin::shutdown(&mut *guard, ctx)
    }

    fn reset(&mut self, ctx: PluginContext<'_>) {
        let mut guard = self.inner.write();
        Plugin::reset(&mut *guard, ctx)
    }

    fn register_menu(&self, menu_bar: &mut MenuBar) {
        if let Some(guard) = self.inner.try_read() {
            Plugin::register_menu(&*guard, menu_bar)
        }
    }
}

impl<T: Plugin + Renderer> Renderer for __ArcPluginRendererAdapter<T> {
    fn schedule_render(&self, job: RenderJob) -> Result<RenderTask> {
        let guard = self.inner.read();
        Renderer::schedule_render(&*guard, job)
    }
}
// Map supported feature string literals to feature idents for the helper above.
#[doc(hidden)]
#[macro_export]
macro_rules! __map_feat_str {
    ("Gui") => {
        Gui
    };
    ("Renderer") => {
        Renderer
    };
    ($other:literal) => {
        compile_error!(concat!(
            "Unknown plugin feature string in register_plugin!: ",
            $other,
            ". Supported: \"Gui\", \"Renderer\""
        ));
    };
}

/// export_plugin!: Register a plugin with a single shared instance.
/// This uses a global OnceLock<Arc<RwLock<T>>> so that all registered
/// features (e.g., Gui) point to the same heavy instance, while the
/// host still receives Box<dyn Trait> objects via inventory.
///
/// Usage:
///   export_plugin!(MyType);                          // plugin only
///   export_plugin!(MyType, [Gui]);                   // plugin + Gui
///   export_plugin!(MyType, name: "Nice Name");      // custom name
///   export_plugin!(MyType, name: "Nice", [Gui]);    // custom + features
#[macro_export]
macro_rules! export_plugin {
    ($ty:ty) => {
        $crate::export_plugin!($ty, name: stringify!($ty), [ ]);
    };
    ($ty:ty, name: $name:expr, [ $($feat:ident),* $(,)? ]) => {
        // Shared instance initializer
        fn __lunaris_shared_instance() -> std::sync::Arc<$crate::parking_lot::RwLock<$ty>> {
            static INSTANCE: std::sync::OnceLock<std::sync::Arc<$crate::parking_lot::RwLock<$ty>>> =
                std::sync::OnceLock::new();
            INSTANCE
                .get_or_init(|| std::sync::Arc::new($crate::parking_lot::RwLock::new(<$ty>::new())))
                .clone()
        }

        // Always register core Plugin using the shared instance
        const _: fn() = || {
            fn assert_impl<T: $crate::plugin::Plugin>() {}
            let _ = assert_impl::<$ty>;
        };
        $crate::submit_raw! {
            $crate::plugin::PluginRegistration {
                name: $name,
                build: || Box::new($crate::plugin::__ArcPluginAdapter::<$ty>::new_with_shared(__lunaris_shared_instance())) ,
            }
        }
        $(
            $crate::__private_export_feature!($ty, $name, __lunaris_shared_instance, $feat);
        )*
    };
    ($ty:ty, [ $($feat:ident),* $(,)? ]) => {
        $crate::export_plugin!($ty, name: stringify!($ty), [ $($feat),* ]);
    };
    ($ty:ty, name: $name:expr) => {
        $crate::export_plugin!($ty, name: $name, [ ]);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __private_export_feature {
    ($ty:ty, $name:expr, $shared:path, Gui) => {
        const _: fn() = || {
            fn assert_impl<T: $crate::plugin::Gui>() {}
            let _ = assert_impl::<$ty>;
        };
        $crate::submit_raw! {
            $crate::plugin::GuiRegistration {
                name: $name,
                build: || Box::new($crate::plugin::__ArcPluginGuiAdapter::<$ty>::new_with_shared($shared())),
            }
        }
    };
    ($ty:ty, $name:expr, $shared:path, Skeleton) => {
        const _: fn() = || {
            fn assert_impl<T: $crate::plugin::Gui>() {}
            let _ = assert_impl::<$ty>;
        };
        $crate::submit_raw! {
            $crate::plugin::GuiRegistration {
                name: $name,
                build: || Box::new($crate::plugin::__ArcPluginGuiAdapter::<$ty>::new_with_shared($shared())),
            }
        }
    };
    ($ty:ty, $name:expr, $shared:path, Renderer) => {
        const _: fn() = || {
            fn assert_impl<T: $crate::plugin::Renderer>() {}
            let _ = assert_impl::<$ty>;
        };
        $crate::submit_raw! {
            $crate::plugin::RendererRegistration {
                name: $name,
                build: || Box::new($crate::plugin::__ArcPluginRendererAdapter::<$ty>::new_with_shared($shared())),
            }
        }
    };
    ($ty:ty, $name:expr, $shared:path, $other:ident) => {
        compile_error!(concat!(
            "Unknown plugin feature in export_plugin!: ",
            stringify!($other),
            ". Supported: Gui, Renderer"
        ));
    };
}
